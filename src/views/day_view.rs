use color_eyre::eyre::Result;
use std::{
    collections::BTreeMap,
    iter,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};
use time::{macros::format_description, Date};
use time_tz::TimeZone;

use crate::{
    config::{CalendarView, ParsedConfig},
    model::{calendar::Calendar, event::EventList},
    util::write_template,
};

/// Type alias representing a specific day in time
type Day = Date;

/// A triple with the previous, current, and next days present
///
/// Note that the previous and next days may be None
pub type DaySlice<'a> = &'a [Option<(&'a Day, &'a EventList)>];

#[derive(Debug)]
pub struct DayView {
    /// The output directory for day view files
    output_dir: PathBuf,
    /// A BTreeMap of Vecs grouped by specific days
    day_map: BTreeMap<Day, EventList>,
}

impl DayView {
    pub fn new(output_dir: PathBuf, calendars: &Vec<Calendar>) -> Self {
        let mut day_map: BTreeMap<Date, EventList> = BTreeMap::new();

        // add events to the day_map
        for calendar in calendars {
            for event in calendar.events() {
                day_map
                    .entry(event.start().date())
                    .or_default()
                    .push(event.clone());
            }
        }

        DayView {
            output_dir,
            day_map,
        }
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // chain a None to the list of weeks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(self.day_map.iter().map(Some))
            .chain(iter::once(None).into_iter());
        let day_windows = &chained_iter.collect::<Vec<Option<(&Day, &EventList)>>>();

        // iterate through all windows
        for window in day_windows.windows(3) {
            let next_day_opt = window[2];

            let mut index_paths = vec![];

            // write the index page for the current day
            if !index_written {
                if let Some(next_day) = next_day_opt {
                    let next_day = next_day.0;
                    // write the index file if the next day is after the current date
                    if next_day > &config.agenda_start_date {
                        index_written = true;
                        index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                        // write the main index as the day view
                        if config.default_calendar_view == CalendarView::Day {
                            index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    // write the index if next_day is None and nothing has been written yet
                    index_written = true;
                    index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                    // write the main index as the day view
                    if config.default_calendar_view == CalendarView::Day {
                        index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

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

    fn write_view(
        &self,
        config: &ParsedConfig,
        tera: &Tera,
        day_slice: &DaySlice,
        output_dir: &Path,
        index_paths: &[PathBuf],
    ) -> Result<()> {
        let previous_day = day_slice[0];
        let current_day = day_slice[1].expect("Current week is None. This should never happen.");
        let next_day = day_slice[2];

        let (day, events) = current_day;
        println!("day: {}", day);
        for event in events {
            println!(
                "  event: ({} {} {}) {} {}",
                event.start().weekday(),
                event.year(),
                event.week(),
                event.summary(),
                event.start(),
            );
        }
        let file_name = format!(
            "{}.html",
            day.format(format_description!("[year]-[month]-[day]"))?
        );
        // TODO should we raise the error on format() failing?
        let previous_file_name = previous_day.map(|(previous_day, _events)| {
            previous_day
                .format(format_description!("[year]-[month]-[day]"))
                .map(|file_root| format!("{}.html", file_root))
                .unwrap_or_default()
        });
        let next_file_name = next_day.map(|(next_day, _events)| {
            next_day
                .format(format_description!("[year]-[month]-[day]"))
                .map(|file_root| format!("{}.html", file_root))
                .unwrap_or_default()
        });

        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", config.display_timezone.name());
        context.insert("year", &day.year());
        context.insert("month", &day.month());
        context.insert("day", &day.day());
        context.insert("events", events);

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
                        .map(|path| ["day", path].join("/")),
                );
                context.insert(
                    "next_file_name",
                    &next_file_name.as_ref().map(|path| ["day", path].join("/")),
                );
            } else {
                context.insert("previous_file_name", &previous_file_name);
                context.insert("next_file_name", &next_file_name);
            }

            // write the actual template
            write_template(tera, "day.html", &context, file_path)?;
        }

        Ok(())
    }
}
