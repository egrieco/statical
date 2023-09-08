use color_eyre::eyre::Result;
use itertools::Itertools;
use log::debug;
use std::{collections::BTreeMap, path::PathBuf};

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

    fn config(&self) -> &Config {
        &self.calendars.config
    }

    /// Loops through all of the weeks in this view's collection.
    ///
    /// None values are prepended and appended to the list to properly handle the first and last intervals.
    ///
    /// # Errors
    ///
    /// This function will return an error if templates cannot be written.
    pub fn create_html_pages(&self, config: &Config) -> Result<()> {
        let mut index_written = false;

        // let week_windows = self.fun_name();

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
                    if next_week.start_datetime >= config.calendar_today_date {
                        index_written = true;
                        write_view_index = true;
                        // index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                        // write the main index as the week view
                        if config.default_calendar_view == CalendarView::Week {
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
                    if config.default_calendar_view == CalendarView::Week {
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
        let mut context = self.calendars.template_context();
        context.insert(
            "view_date",
            &current_week
                .format(&self.config().week_view_format)
                .to_string(),
        );
        context.insert("year", &year);
        // TODO add month numbers
        context.insert("month_name", &current_week.month());
        context.insert("week", &week);
        context.insert("week_dates", &week_dates);

        // create the main file path
        let binding = self.output_dir.join(PathBuf::from(&file_name));
        // the first item in this tuple is a flag indicating whether to prepend the view path
        let mut file_paths = vec![(true, binding)];

        if write_view_index {
            file_paths.push((true, self.output_dir.join(PathBuf::from("index.html"))));
        }
        if write_main_index {
            file_paths.push((
                true,
                self.config().output_dir.join(PathBuf::from("index.html")),
            ));
        }

        // write the template to all specified paths
        debug!("{} file paths to write", file_paths.len());
        for (prepend_view_path, file_path) in file_paths {
            // if the path matches the root path, prepend the default view to the next and previous links

            let mut base_url_path: unix_path::PathBuf =
                self.calendars.config.base_url_path.path_buf().clone();

            // TODO: need to clean up the prepending logic, we're always taking the same code path at the moment
            // TODO: need to decide whether to support relative paths where that logic was necessary

            // we're prepending the view path here
            if prepend_view_path {
                base_url_path.push("week")
            };

            let prepend_base_path = true;

            // and we're prepending the base path here
            let previous_file_path = match (&previous_file_name, prepend_base_path) {
                (Some(prev_file), true) => base_url_path
                    .as_path()
                    .join(prev_file)
                    .to_str()
                    .map(String::from),
                (Some(prev_file), false) => Some(prev_file).cloned(),
                (None, _) => None,
            };
            let next_file_path = match (&next_file_name, prepend_base_path) {
                (Some(next_file), true) => base_url_path
                    .as_path()
                    .join(next_file)
                    .to_str()
                    .map(String::from),
                (Some(next_file), false) => Some(next_file).cloned(),
                (None, _) => None,
            };
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
            write_template(&self.calendars.tera, "week.html", &context, &file_path)?;
        }

        Ok(())
    }
}
