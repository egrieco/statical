use color_eyre::eyre::Result;
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};

use crate::model::calendar_collection::CalendarCollection;
use crate::model::week::Week;
use crate::{
    config::{CalendarView, Config},
    model::{day::DayContext, event::EventList},
    util::write_template,
};

/// A BTreeMap of Vecs grouped by specific weeks
pub type WeekMap = BTreeMap<Week, EventList>;

/// A BTreeMap of Vecs grouped by specific weekday
pub type WeekDayMap = BTreeMap<u8, EventList>;

/// A triple with the previous, current, and next weeks present
///
/// Note that the previous and next weeks may be None
pub type WeekSlice<'a> = &'a [Option<Week>];

#[derive(Debug)]
pub struct WeekView<'a> {
    /// The output directory for week view files
    output_dir: PathBuf,
    calendars: &'a CalendarCollection,
}

impl WeekView<'_> {
    pub fn new(output_dir: PathBuf, calendars: &CalendarCollection) -> WeekView<'_> {
        WeekView {
            output_dir,
            calendars,
        }
    }

    /// Loops through all of the weeks in this view's collection.
    ///
    /// None values are prepended and appended to the list to properly handle the first and last intervals.
    ///
    /// # Errors
    ///
    /// This function will return an error if templates cannot be written.
    pub fn create_html_pages(&self, config: &Config, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // let week_windows = self.fun_name();

        // iterate through all windows
        for window in self.calendars.weeks_to_show()?.windows(3) {
            let next_week_opt = &window[2];

            let mut index_paths = vec![];

            // figure out which dates should be the index files and pass in an array of index file paths
            if !index_written {
                if let Some(next_week) = next_week_opt {
                    // write the index file if the next month is after the current date
                    if next_week.start_datetime >= config.agenda_start_date {
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
        config: &Config,
        tera: &Tera,
        week_slice: &WeekSlice,
        output_dir: &Path,
        index_paths: &[PathBuf],
    ) -> Result<()> {
        let previous_week = &week_slice[0].as_ref();
        let current_week = week_slice[1]
            .as_ref()
            .expect("Current week is None. This should never happen.");
        let next_week = &week_slice[2].as_ref();

        let mut week_dates = Vec::new();
        for day in current_week.days() {
            let events = self
                .calendars
                .events_by_day
                // TODO: I doubt that we need to adjust the timezone here, probably remove it
                .get(&day);
            println!(
                "  For day {}: there are {} events",
                day,
                events.map(|e| e.len()).unwrap_or(0)
            );
            week_dates.push(DayContext::new(
                day,
                events
                    .map(|l| {
                        l.iter()
                            .sorted()
                            .map(|e| e.context(&self.calendars.config))
                            .collect()
                    })
                    .unwrap_or_default(),
            ));
        }

        let year = current_week.year();
        let week = current_week.week();

        // setup file names
        // TODO make the file format a module constant
        let file_name = format!("{}-{}.html", year, week);
        let previous_file_name = previous_week
            .map(|previous_week| format!("{}-{}.html", previous_week.year(), previous_week.week()));
        let next_file_name =
            next_week.map(|next_week| format!("{}-{}.html", next_week.year(), next_week.week()));

        // get the events grouped by day
        // let week_dates = week_day_map.context(year, week, config)?;

        // setup the tera context
        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", &config.display_timezone.name());
        context.insert("year", &year);
        // TODO add month numbers
        context.insert("month_name", &current_week.month());
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
