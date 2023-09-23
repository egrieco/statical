use color_eyre::eyre::Result;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    configuration::config::Config,
    model::{calendar_collection::CalendarCollection, event::Event},
};

/// A triple with the previous, current, and next events present
///
/// Note that the previous and next events may be None
pub type EventSlice<'a> = &'a [Option<Rc<Event>>];

pub(crate) const VIEW_PATH: &str = "event";
const PAGE_TITLE: &str = "Event Page";

#[derive(Debug)]
pub struct EventView<'a> {
    calendars: &'a CalendarCollection,
    output_dir: PathBuf,
}

impl EventView<'_> {
    pub fn new(calendars: &CalendarCollection) -> EventView<'_> {
        let output_dir = calendars
            .base_dir()
            .join(&calendars.config.output_dir)
            .join(VIEW_PATH);
        EventView {
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
        for window in self.calendars.events_to_show()?.windows(3) {
            let next_event_opt = &window[2];

            let mut index_paths = vec![];

            // write the index page for the current day
            if !index_written {
                if let Some(next_event) = next_event_opt {
                    // write the index file if the next day is after the current date
                    // TODO: do we need to adjust for timezone?
                    if next_event.start().date_naive() > self.calendars.today_date() {
                        index_written = true;
                        index_paths.push(self.output_dir().join(PathBuf::from("index.html")));
                    }
                } else {
                    // write the index if next_day is None and nothing has been written yet
                    index_written = true;
                    index_paths.push(self.output_dir().join(PathBuf::from("index.html")));
                }
            }

            self.write_view(&window, index_paths.as_slice())?;
        }

        Ok(())
    }

    fn write_view(&self, event_slice: &EventSlice, index_paths: &[PathBuf]) -> Result<()> {
        let previous_event = &event_slice[0];
        let current_event = &event_slice[1]
            .as_ref()
            .expect("Current week is None. This should never happen.");
        let next_event = &event_slice[2];

        println!("event: {}", current_event);
        println!(
            "  event: ({} {} {}) {} {}",
            current_event.weekday(),
            current_event.year(),
            current_event.week(),
            current_event.summary(),
            current_event.start(),
        );

        let file_name = current_event.file_name();
        // TODO should we raise the error on format() failing?
        let previous_file_name = previous_event.as_ref().map(|e| e.file_name());
        let next_file_name = next_event.as_ref().map(|e| e.file_name());

        let mut context = self.calendars.template_context();
        context.insert("current_view", VIEW_PATH);
        context.insert("page_title", PAGE_TITLE);
        // TODO: how do we want to handle events that span two days?
        context.insert(
            "view_date",
            &current_event
                .start()
                // TODO: maybe give this its own format config
                .format(&self.config().day_view_format)
                .to_string(),
        );
        context.insert("year", &current_event.year());
        context.insert("month", &current_event.month());
        context.insert(
            "month_name",
            &current_event
                .month()
                .map(|m| m.name())
                .unwrap_or("UNKNOWN MONTH"),
        );
        context.insert("day", &current_event.day());
        // TODO switch these to contexts
        context.insert("event", &current_event.context(self.config()));

        let base_url_path: unix_path::PathBuf =
            self.calendars.config.base_url_path.path_buf().clone();

        // create the main file path
        let binding = self.output_dir().join(PathBuf::from(&file_name));
        let mut file_paths = vec![&binding];
        // then add any additional index paths
        file_paths.extend(index_paths);

        // write the template to all specified paths
        for file_path in file_paths {
            let view_path = base_url_path.join(VIEW_PATH);
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
                .write_template("event.html", &context, file_path)?;
        }

        Ok(())
    }
}
