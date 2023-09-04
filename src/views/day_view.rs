use chrono::Datelike;
use color_eyre::eyre::Result;
use itertools::Itertools;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};
use tera::{Context, Tera};

use crate::{
    config::{CalendarView, Config},
    model::{
        calendar_collection::CalendarCollection,
        day::{Day, DayContext},
        event::{Event, EventContext},
    },
    util::write_template,
};

const YMD_FORMAT: &str = "%Y-%m-%d";

/// A triple with the previous, current, and next days present
///
/// Note that the previous and next days may be None
pub type DaySlice<'a> = &'a [Option<Day>];

#[derive(Debug)]
pub struct DayView<'a> {
    /// The output directory for day view files
    output_dir: PathBuf,
    calendars: &'a CalendarCollection,
}

impl DayView<'_> {
    pub fn new(output_dir: PathBuf, calendars: &CalendarCollection) -> DayView<'_> {
        DayView {
            output_dir,
            calendars,
        }
    }

    pub fn create_html_pages(&self, config: &Config, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // iterate through all windows
        for window in self.calendars.days_to_show()?.windows(3) {
            let next_day_opt = &window[2];

            let mut index_paths = vec![];

            // write the index page for the current day
            if !index_written {
                if let Some(next_day) = next_day_opt {
                    // write the index file if the next day is after the current date
                    if next_day.start_datetime > config.agenda_start_date {
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
        config: &Config,
        tera: &Tera,
        day_slice: &DaySlice,
        output_dir: &Path,
        index_paths: &[PathBuf],
    ) -> Result<()> {
        let previous_day = &day_slice[0].as_ref();
        let current_day = day_slice[1]
            .as_ref()
            .expect("Current week is None. This should never happen.");
        let next_day = day_slice[2].as_ref();

        let day = current_day.start;
        let empty_vec = vec![];
        let events: &Vec<Rc<Event>> = self.calendars.events_by_day.get(&day).unwrap_or(&empty_vec);

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

        DayContext::new(
            day,
            events
                .iter()
                .sorted()
                .map(|e| e.context(&self.calendars.config))
                .collect::<Vec<_>>(),
        );

        let file_name = format!("{}.html", day.format(YMD_FORMAT));
        // TODO should we raise the error on format() failing?
        let previous_file_name =
            previous_day.map(|previous_day| format!("{}.html", previous_day.format(YMD_FORMAT)));
        let next_file_name =
            next_day.map(|next_day| format!("{}.html", next_day.format(YMD_FORMAT)));

        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", config.display_timezone.name());
        context.insert("year", &day.year());
        context.insert("month", &day.month());
        context.insert("month_name", &current_day.month());
        context.insert("day", &day.day());
        // TODO switch these to contexts
        context.insert(
            "events",
            &events
                .iter()
                .map(|e| e.context(config))
                .collect::<Vec<EventContext>>(),
        );

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
