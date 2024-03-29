use chrono::Datelike;
use color_eyre::eyre::Result;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    configuration::{config::Config, types::calendar_view::CalendarView},
    model::{
        calendar_collection::CalendarCollection,
        day::Day,
        event::{Event, EventContext},
    },
};

pub(crate) const YMD_FORMAT: &str = "%Y-%m-%d";

/// A triple with the previous, current, and next days present
///
/// Note that the previous and next days may be None
pub type DaySlice<'a> = &'a [Option<Day>];

pub(crate) const VIEW_PATH: &str = "day";
const PAGE_TITLE: &str = "Day Page";

#[derive(Debug)]
pub struct DayView<'a> {
    calendars: &'a CalendarCollection,
    output_dir: PathBuf,
}

impl DayView<'_> {
    pub fn new(calendars: &CalendarCollection) -> DayView<'_> {
        let output_dir = calendars
            .base_dir()
            .join(&calendars.config.output_dir)
            .join(VIEW_PATH);
        DayView {
            calendars,
            output_dir,
        }
    }

    fn config(&self) -> &Config {
        &self.calendars.config
    }

    fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub fn create_html_pages(&self) -> Result<()> {
        // create the subdirectory to hold the files
        create_dir_all(self.output_dir())?;

        let mut index_written = false;

        // iterate through all windows
        for window in self.calendars.days_to_show()?.windows(3) {
            let next_day_opt = &window[2];

            let mut index_paths = vec![];

            // write the index page for the current day
            if !index_written {
                if let Some(next_day) = next_day_opt {
                    // write the index file if the next day is after the current date
                    if next_day.start_datetime.date_naive() > self.calendars.today_date() {
                        index_written = true;
                        index_paths.push(self.output_dir().join(PathBuf::from("index.html")));

                        // write the main index as the day view
                        if self.config().default_calendar_view == CalendarView::Day {
                            index_paths
                                .push(self.config().output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    // write the index if next_day is None and nothing has been written yet
                    index_written = true;
                    index_paths.push(self.output_dir().join(PathBuf::from("index.html")));

                    // write the main index as the day view
                    if self.config().default_calendar_view == CalendarView::Day {
                        index_paths
                            .push(self.config().output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

            self.write_view(&window, index_paths.as_slice())?;
        }

        Ok(())
    }

    fn write_view(&self, day_slice: &DaySlice, index_paths: &[PathBuf]) -> Result<()> {
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

        let file_name = format!("{}.html", day.format(YMD_FORMAT));
        // TODO should we raise the error on format() failing?
        let previous_file_name =
            previous_day.map(|previous_day| format!("{}.html", previous_day.format(YMD_FORMAT)));
        let next_file_name =
            next_day.map(|next_day| format!("{}.html", next_day.format(YMD_FORMAT)));

        let mut context = self.calendars.template_context();

        // let first_event = events.first().expect("could not get first event for page");
        // let base_url_path: unix_path::PathBuf = self.config.base_url_path.path_buf().clone();
        context.insert("month_view_path", &current_day.month_view_path());
        context.insert("week_view_path", &current_day.week_view_path());
        // context.insert("day_view_path", &current_day.day_view_path());
        context.insert("event_view_path", &events.first().map(|e| e.file_path()));
        // context.insert("agenda_view_path", &base_url_path.join("agenda"));

        context.insert("current_view", VIEW_PATH);
        context.insert("page_title", PAGE_TITLE);
        context.insert(
            "view_date",
            &current_day
                .format(&self.config().day_view_format)
                .to_string(),
        );
        context.insert("year", &day.year());
        context.insert("month", &day.month());
        context.insert("month_name", &current_day.month());
        context.insert("day", &day.day());
        // TODO switch these to contexts
        context.insert(
            "events",
            &events
                .iter()
                .map(|e| e.context(self.config()))
                .collect::<Vec<EventContext>>(),
        );

        let base_url_path: unix_path::PathBuf =
            self.calendars.config.base_url_path.path_buf().clone();

        // create the main file path
        let binding = self.output_dir().join(PathBuf::from(&file_name));
        let mut file_paths = vec![&binding];
        // then add any additional index paths
        file_paths.extend(index_paths);

        // write the template to all specified paths
        for file_path in file_paths {
            let view_path = base_url_path.join("day");
            context.insert(
                "previous_file_name",
                &previous_file_name.as_ref().map(|path| view_path.join(path)),
            );
            context.insert(
                "next_file_name",
                &next_file_name.as_ref().map(|path| view_path.join(path)),
            );

            // write the actual template
            self.calendars
                .write_template("day.html", &context, file_path)?;
        }

        Ok(())
    }
}
