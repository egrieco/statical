use chrono::Datelike;
use color_eyre::eyre::Result;
use dedup_iter::DedupAdapter;
use std::{
    collections::BTreeMap,
    iter,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};

use crate::{
    config::{CalendarView, ParsedConfig},
    model::{
        calendar::Calendar,
        calendar_collection::WeekContext,
        event::{EventList, WeekNum, Year},
    },
    util::write_template,
};

/// Type alias representing a specific week in time
pub(crate) type Week = (Year, WeekNum);

/// A BTreeMap of Vecs grouped by specific weeks
pub type WeekMap = BTreeMap<Week, EventList>;

/// A BTreeMap of Vecs grouped by specific weekday
pub type WeekDayMap = BTreeMap<u8, EventList>;

/// A triple with the previous, current, and next weeks present
///
/// Note that the previous and next weeks may be None
pub type WeekSlice<'a> = &'a [Option<(&'a Week, &'a EventList)>];

#[derive(Debug)]
pub struct WeekView {
    /// The output directory for week view files
    output_dir: PathBuf,
    week_map: WeekMap,
}

impl WeekView {
    pub fn new(output_dir: PathBuf, calendars: &Vec<Calendar>) -> Self {
        let mut week_map: BTreeMap<Week, EventList> = BTreeMap::new();

        // add events to the week_map
        for calendar in calendars {
            for event in calendar.events() {
                week_map
                    .entry((event.year(), event.week()))
                    .or_default()
                    .push(event.clone());
            }
        }

        WeekView {
            output_dir,
            week_map,
        }
    }

    /// Loops through all of the weeks in this view's collection.
    ///
    /// None values are prepended and appended to the list to properly handle the first and last intervals.
    ///
    /// # Errors
    ///
    /// This function will return an error if templates cannot be written.
    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // chain a None to the list of weeks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(self.week_map.iter().map(Some))
            .chain(iter::once(None));
        let week_windows = &chained_iter.collect::<Vec<Option<(&Week, &EventList)>>>();

        // iterate through all windows
        for window in week_windows.windows(3) {
            let next_week_opt = window[2];

            let mut index_paths = vec![];

            // figure out which dates should be the index files and pass in an array of index file paths
            if !index_written {
                if let Some(next_week) = next_week_opt {
                    // create the agenda start week tuple
                    let agenda_start_week = (
                        config.agenda_start_date.year(),
                        config.agenda_start_date.iso_week().week() as u8,
                    );

                    // write the index file if the next month is after the current date
                    if next_week.0 >= &agenda_start_week {
                        index_written = true;
                        index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                        // write the main index as the week view
                        if config.default_calendar_view == CalendarView::Week {
                            index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    // write the index if we get to the last week and still haven't met the condition
                    index_written = true;
                    index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                    // write the main index as the week view
                    if config.default_calendar_view == CalendarView::Week {
                        index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

            // write the actual files
            self.write_view(
                config,
                tera,
                &window,
                &self.output_dir,
                index_paths.as_slice(),
            )?;
        }

        Ok(())
    }

    /// Takes a `WeekSlice` and writes the corresponding file
    ///
    /// # Panics
    ///
    /// Panics if the current_week (in the middle of the slice) is ever None. This should never happen.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be written to disk.
    fn write_view(
        &self,
        config: &ParsedConfig,
        tera: &Tera,
        week_slice: &WeekSlice,
        output_dir: &Path,
        index_paths: &[PathBuf],
    ) -> Result<()> {
        let previous_week = week_slice[0];
        let current_week = week_slice[1].expect("Current week is None. This should never happen.");
        let next_week = week_slice[2];

        let ((year, week), events) = current_week;

        println!("week: {:?}", current_week);
        let mut week_day_map: WeekDayMap = BTreeMap::new();
        for event in events {
            println!(
                "  event: ({} {} {}) {} {}",
                event.start().weekday(),
                event.year(),
                event.week(),
                event.summary(),
                event.start(),
            );
            let day_of_week = event.start().weekday().num_days_from_sunday() as u8;
            week_day_map
                .entry(day_of_week)
                .or_default()
                .push(event.clone());
        }

        // setup file names
        // TODO make the file format a module constant
        let file_name = format!("{}-{}.html", year, week);
        let previous_file_name = previous_week.map(|((previous_year, previous_week), _events)| {
            format!("{}-{}.html", previous_year, previous_week)
        });
        let next_file_name = next_week
            .map(|((next_year, next_week), _events)| format!("{}-{}.html", next_year, next_week));

        // get the events grouped by day
        let week_dates = week_day_map.context(year, week, &config.display_timezone)?;

        // setup the tera context
        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", &config.display_timezone.name());
        context.insert("year", &year);
        // TODO add month numbers
        context.insert(
            "month_name",
            &week_dates
                .iter()
                .map(|d| d.month.clone())
                .dedup()
                .collect::<Vec<String>>()
                .join(" - "),
        );
        context.insert("week", &week);
        context.insert("week_dates", &week_dates);

        // create the main file path
        let binding = output_dir.join(PathBuf::from(&file_name));
        let mut file_paths = vec![&binding];
        // then add any additional index paths
        file_paths.extend(index_paths);

        // write the template to all specified paths
        for file_path in file_paths {
            // if the path matches the root path, prepend the default view to the next and previous links
            if file_path.parent() == Some(&config.output_dir) {
                context.insert(
                    "previous_file_name",
                    &previous_file_name
                        .as_ref()
                        .map(|path| ["week", path].join("/")),
                );
                context.insert(
                    "next_file_name",
                    &next_file_name.as_ref().map(|path| ["week", path].join("/")),
                );
            } else {
                context.insert("previous_file_name", &previous_file_name);
                context.insert("next_file_name", &next_file_name);
            }

            // write the actual template
            write_template(tera, "week.html", &context, file_path)?;
        }

        Ok(())
    }
}
