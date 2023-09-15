use color_eyre::eyre::Result;
use log::debug;
use std::fs::create_dir_all;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};

use crate::configuration::types::calendar_view::CalendarView;
use crate::model::calendar_collection::CalendarCollection;
use crate::model::week::Week;
use crate::{configuration::config::Config, model::event::EventList};

/// A BTreeMap of Vecs grouped by specific weeks
pub type WeekMap<'a> = BTreeMap<Week<'a>, EventList>;

/// A BTreeMap of Vecs grouped by specific weekday
pub type WeekDayMap = BTreeMap<u8, EventList>;

/// A triple with the previous, current, and next weeks present
///
/// Note that the previous and next weeks may be None
pub type WeekSlice<'a> = &'a [Option<Week<'a>>];

const VIEW_PATH: &str = "week";

#[derive(Debug)]
pub struct WeekView<'a> {
    calendars: &'a CalendarCollection,
    output_dir: PathBuf,
}

impl WeekView<'_> {
    pub fn new(calendars: &CalendarCollection) -> WeekView<'_> {
        let output_dir = calendars
            .base_dir()
            .join(&calendars.config.output_dir)
            .join(VIEW_PATH);
        WeekView {
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

    /// Loops through all of the weeks in this view's collection.
    ///
    /// None values are prepended and appended to the list to properly handle the first and last intervals.
    ///
    /// # Errors
    ///
    /// This function will return an error if templates cannot be written.
    pub fn create_html_pages(&self) -> Result<()> {
        // create the subdirectory to hold the files
        create_dir_all(self.output_dir())?;

        let mut index_written = false;

        // iterate through all windows
        for window in self.calendars.weeks_to_show()?.windows(3) {
            let next_week_opt = &window[2];

            // let mut index_paths = vec![];

            // write the index view if the next item is greater than or equal to this one
            // write the main index view if the default calendar view is set to match this class
            // write the index view if the next item is None and we have not yet written the index

            let mut write_view_index = false;
            let mut write_main_index = false;

            // figure out which dates should be the index files and pass in an array of index file paths
            if !index_written {
                if let Some(next_week) = next_week_opt {
                    // write the index file if the next month is after the current date
                    if next_week.first_day() >= self.calendars.today_date() {
                        index_written = true;
                        write_view_index = true;
                        // index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                        // write the main index as the week view
                        if self.config().default_calendar_view == CalendarView::Week {
                            write_main_index = true;
                            // index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    // write the index if we get to the last week and still haven't met the condition
                    index_written = true;
                    write_view_index = true;
                    // index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                    // write the main index as the week view
                    if self.config().default_calendar_view == CalendarView::Week {
                        write_main_index = true;
                        // index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

            // write the actual files
            self.write_view(&window, write_view_index, write_main_index)?;
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
        week_slice: &WeekSlice,
        write_view_index: bool,
        write_main_index: bool,
    ) -> Result<()> {
        let previous_week = &week_slice[0].as_ref();
        let current_week = week_slice[1]
            .as_ref()
            .expect("Current week is None. This should never happen.");
        let next_week = &week_slice[2].as_ref();

        // setup file names
        let file_name = current_week.file_name();
        let previous_file_name = previous_week.map(|previous_week| previous_week.file_name());
        let next_file_name = next_week.map(|next_week| next_week.file_name());

        // setup the tera context
        let mut context = self.calendars.template_context();
        context.insert(
            "view_date",
            &current_week
                .format(&self.config().week_view_format)
                .to_string(),
        );
        context.insert("year", &current_week.year());
        context.insert("year_start", &current_week.year_start());
        context.insert("year_end", &current_week.year_end());
        context.insert("month", &current_week.month().number_from_month());
        context.insert("month_name", &current_week.month().name());
        context.insert(
            "month_start",
            &current_week.month_start().number_from_month(),
        );
        context.insert("month_start_name", &current_week.month_start().name());
        context.insert("month_end", &current_week.month_end().number_from_month());
        context.insert("month_end_name", &current_week.month_end().name());
        context.insert("iso_week", &current_week.iso_week());
        context.insert("week_dates", &current_week.week_dates());
        context.insert("week_switches_months", &current_week.week_switches_months());
        context.insert("week_switches_years", &current_week.week_switches_years());

        // create the main file path
        let current_file_name = self.output_dir().join(PathBuf::from(&file_name));
        // the first item in this tuple is a flag indicating whether to prepend the view path
        let mut file_paths = vec![current_file_name];

        if write_view_index {
            file_paths.push(self.output_dir().join(PathBuf::from("index.html")));
        }
        if write_main_index {
            file_paths.push(self.config().output_dir.join(PathBuf::from("index.html")));
        }

        // write the template to all specified paths
        debug!("{} file paths to write", file_paths.len());
        for file_path in file_paths {
            // we're cloning this since we're going to be modifying it directly
            let mut base_url_path: unix_path::PathBuf =
                self.calendars.config.base_url_path.path_buf().clone();

            // we're prepending the view path here
            base_url_path.push("week");

            // and we're prepending the base path here
            let previous_file_path = previous_file_name.as_ref().and_then(|prev_file| {
                base_url_path
                    .as_path()
                    .join(prev_file)
                    .to_str()
                    .map(String::from)
            });
            let next_file_path = next_file_name.as_ref().and_then(|next_file| {
                base_url_path
                    .as_path()
                    .join(next_file)
                    .to_str()
                    .map(String::from)
            });

            context.insert("previous_file_name", &previous_file_path);
            context.insert("next_file_name", &next_file_path);
            debug!("writing file path: {:?}", file_path);
            debug!("base_url_path is: {:?}", base_url_path);
            debug!("previous_file_name is: {:?}", previous_file_name);
            debug!("previous_file_path is: {:?}", previous_file_path);
            debug!("next_file_name is: {:?}", next_file_name);
            debug!("next_file_path is: {:?}", next_file_path);
            // } else {
            // context.insert("previous_file_name", &previous_file_name);
            // context.insert("next_file_name", &next_file_name);
            // }

            // write the actual template
            self.calendars
                .write_template("week.html", &context, &file_path)?;
        }

        Ok(())
    }
}
