use chrono::Datelike;
use color_eyre::eyre::Result;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use crate::{
    configuration::{config::Config, types::calendar_view::CalendarView},
    model::{
        agenda::{Agenda, AgendaSlice},
        calendar_collection::CalendarCollection,
        event::EventContext,
    },
};

const VIEW_PATH: &str = "agenda";
const PAGE_TITLE: &str = "Agenda Page";

#[derive(Debug)]
pub(crate) struct AgendaView<'a> {
    calendars: &'a CalendarCollection,
    output_dir: PathBuf,
}

impl AgendaView<'_> {
    pub fn new(calendars: &CalendarCollection) -> AgendaView<'_> {
        let output_dir = calendars
            .base_dir()
            .join(&calendars.config.output_dir)
            .join(VIEW_PATH);
        AgendaView {
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

        let events = Agenda::new(self.calendars);

        // iterate through all windows
        for window in events.pages().windows(3) {
            let next_page_opt = window[2];

            let mut index_paths = vec![];

            // write the index page for the current week
            // TODO might want to write the index if next_week is None and nothing has been written yet
            if !index_written {
                if let Some(next_page) = next_page_opt {
                    let (page, _events) = next_page;
                    // write the index file if the next month is after the current date
                    // TODO make sure that the conditional tests are correct, maybe add some tests
                    // TODO handle the case when there is no page 1 (when there are less than agenda_events_per_page past current)
                    if page == &1_isize {
                        index_written = true;
                        index_paths.push(self.output_dir().join(PathBuf::from("index.html")));

                        // write the main index as the week view
                        if self.config().default_calendar_view == CalendarView::Agenda {
                            index_paths
                                .push(self.config().output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    index_written = true;
                    index_paths.push(self.output_dir().join(PathBuf::from("index.html")));

                    // write the main index as the week view
                    if self.config().default_calendar_view == CalendarView::Agenda {
                        index_paths
                            .push(self.config().output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

            self.write_view(&window, index_paths.as_slice())?;
        }

        Ok(())
    }

    /// Takes a `AgendaSlice` and writes the corresponding file
    ///
    /// # Panics
    ///
    /// Panics if the current_page (in the middle of the slice) is ever None. This should never happen.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be written to disk.
    fn write_view(&self, agenda_slice: &AgendaSlice, index_paths: &[PathBuf]) -> Result<()> {
        let previous_page = agenda_slice[0];
        let current_page =
            agenda_slice[1].expect("Current agenda page is None. This should never happen.");
        let next_page = agenda_slice[2];

        let (page, events) = current_page;

        println!("page {:?}", page);
        for event in events.iter() {
            println!(
                "  event: ({} {} {}) {} {}",
                event.start().weekday(),
                event.year(),
                event.week(),
                event.summary(),
                event.start(),
            );
        }

        let event_contexts: Vec<_> = events.iter().map(|e| e.context(self.config())).collect();

        let file_name = format!("{}.html", page);
        let previous_file_name = previous_page.map(|(page_num, _)| format!("{}.html", page_num));
        let next_file_name = next_page.map(|(page_num, _)| format!("{}.html", page_num));

        println!(
            "  {:?} {:?} {:?}",
            previous_file_name, file_name, next_file_name
        );

        let mut context = self.calendars.template_context();

        let first_event = events.first().expect("could not get first event for page");
        // let base_url_path: unix_path::PathBuf = self.config.base_url_path.path_buf().clone();
        context.insert("month_view_path", &first_event.month_view_path());
        context.insert("week_view_path", &first_event.week_view_path());
        context.insert("day_view_path", &first_event.day_view_path());
        context.insert("event_view_path", &first_event.file_path());
        // context.insert("agenda_view_path", &base_url_path.join("agenda"));

        context.insert("current_view", VIEW_PATH);
        context.insert("page_title", PAGE_TITLE);
        // TODO: we need to refactor the way agenda pages are created before we can enable the below
        // context.insert(
        //     "view_date_start",
        //     &current_page
        //         .format(&config.agenda_view_format_start)
        //         .to_string(),
        // );
        // context.insert(
        //     "view_date_end",
        //     &current_page
        //         .format(&config.agenda_view_format_end)
        //         .to_string(),
        // );
        context.insert("page", &page);
        context.insert("events", &event_contexts);

        // event groups are created by the template and whatever format is specified for headers
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
            let view_path = base_url_path.join("agenda");
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
                .write_template("agenda.html", &context, file_path)?;
        }

        Ok(())
    }
}
