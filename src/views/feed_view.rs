use color_eyre::eyre::{Context, Result};
use icalendar::{Calendar, Component, Event};
use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::model::calendar_collection::CalendarCollection;

pub(crate) const VIEW_PATH: &str = "feed";

#[derive(Debug)]
pub struct FeedView<'a> {
    calendars: &'a CalendarCollection,
    output_dir: PathBuf,
}

impl FeedView<'_> {
    pub fn new(calendars: &CalendarCollection) -> FeedView<'_> {
        let output_dir = calendars
            .base_dir
            .join(&calendars.config.output_dir)
            .join(VIEW_PATH);
        FeedView {
            calendars,
            output_dir,
        }
    }

    fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub(crate) fn create_view_files(&self) -> Result<()> {
        // create the subdirectory to hold the files
        create_dir_all(self.output_dir())?;

        // create a calendar
        let mut calendar = Calendar::new();

        // loop through all of the events (probably skip the expanded ones)
        // TODO: write original events with RRules rather than the expanded event recurrences
        for event in self.calendars.events() {
            let ical_event = Event::new()
                .summary(event.summary())
                .description(event.description())
                .done();

            // add the event to the calendar
            calendar.push(ical_event);
        }

        // write the calendar feed file to disk
        // TODO replace this with a debug or log message
        let file_path = self.output_dir().join("feed.ics");
        eprintln!("Writing calendar feed to file: {:?}", file_path);
        let mut output_file = File::create(file_path)?;
        output_file
            .write_all(format!("{}", calendar).as_bytes())
            .wrap_err("could not write calendar feed file")?;

        Ok(())
    }
}
