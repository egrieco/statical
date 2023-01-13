use color_eyre::eyre::Result;
use std::{
    collections::BTreeMap,
    iter,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};
use time_tz::TimeZone;

use super::week_view::WeekMap;
use crate::{
    config::{CalendarView, ParsedConfig},
    model::{
        calendar::Calendar,
        calendar_collection::{blank_context, iso_weeks_for_month_display, WeekContext},
        event::{WeekNum, Year},
    },
    util::write_template,
    views::week_view::WeekDayMap,
};
use crate::{model::event::EventList, views::week_view::Week};

/// Type alias representing a specific month in time
type Month = (Year, u8);

/// A BTreeMap of Vecs grouped by specific months
pub type MonthMap = BTreeMap<Month, WeekMapList>;
type WeekMapList = BTreeMap<WeekNum, WeekMap>;

/// A triple with the previous, current, and next weeks present
///
/// Note that the previous and next weeks may be None
pub type MonthSlice<'a> = &'a [Option<(&'a Month, &'a WeekMapList)>];

#[derive(Debug)]
pub struct MonthView {
    /// The output directory for month view files
    output_dir: PathBuf,
    month_map: MonthMap,
}

impl MonthView {
    pub fn new(output_dir: PathBuf, calendars: &Vec<Calendar>) -> Self {
        let mut month_map: BTreeMap<Week, BTreeMap<u8, BTreeMap<Week, EventList>>> =
            BTreeMap::new();

        // add events to the month_map
        for calendar in calendars {
            for event in calendar.events() {
                month_map
                    .entry((event.year(), event.start().month() as u8))
                    .or_default()
                    .entry(event.week())
                    .or_default()
                    .entry((event.year(), event.week()))
                    .or_default()
                    .push(event.clone());
            }
        }

        MonthView {
            output_dir,
            month_map,
        }
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // chain a None to the list of weeks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(self.month_map.iter().map(Some))
            .chain(iter::once(None).into_iter());
        let month_windows = &chained_iter.collect::<Vec<Option<(&Month, &WeekMapList)>>>();

        // iterate through all windows
        for window in month_windows.windows(3) {
            let next_month_opt = window[2];

            let mut index_paths = vec![];

            // write the index page for the current month
            if !index_written {
                if let Some(next_month) = next_month_opt {
                    let agenda_start_month = (
                        config.agenda_start_date.year(),
                        config.agenda_start_date.month() as u8,
                    );

                    // write the index file if the next month is after the current date
                    if next_month.0 > &agenda_start_month {
                        index_written = true;
                        index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                        // write the main index as the month view
                        if config.default_calendar_view == CalendarView::Month {
                            index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    index_written = true;
                    index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                    // write the main index as the month view
                    if config.default_calendar_view == CalendarView::Month {
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

    /// Takes a `MonthSlice` and writes the corresponding file
    ///
    /// # Panics
    ///
    /// Panics if the current_month (in the middle of the slice) is ever None. This should never happen.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be written to disk.
    fn write_view(
        &self,
        config: &ParsedConfig,
        tera: &Tera,
        month_slice: &MonthSlice,
        output_dir: &Path,
        index_paths: &[PathBuf],
    ) -> Result<()> {
        let previous_month = month_slice[0];
        let current_month =
            month_slice[1].expect("Current month is None. This should never happen.");
        let next_month = month_slice[2];

        let ((year, month), weeks) = current_month;

        println!("month: {}", month);
        let mut week_list = Vec::new();

        // create all weeks in this month
        let weeks_for_display = iso_weeks_for_month_display(year, month)?;
        println!("From week {:?}", weeks_for_display);
        for week_num in weeks_for_display {
            match weeks.get(&week_num) {
                Some(week_map) => {
                    println!("  Creating week {}, {} {}", week_num, month, year);
                    for ((_y, _w), events) in week_map {
                        let mut week_day_map: WeekDayMap = BTreeMap::new();

                        for event in events {
                            println!(
                                "    event: ({} {} {}) {} {}",
                                event.start().weekday(),
                                event.year(),
                                event.week(),
                                event.summary(),
                                event.start(),
                            );
                            let day_of_week = event.start().weekday().number_days_from_sunday();
                            week_day_map
                                .entry(day_of_week)
                                .or_default()
                                .push(event.clone());
                        }

                        // create week days
                        let week_dates =
                            week_day_map.context(year, &week_num, config.display_timezone)?;
                        week_list.push(week_dates);
                    }
                }
                None => {
                    println!("  Inserting blank week {}, {} {}", week_num, month, year);
                    week_list.push(blank_context(year, &week_num)?);
                }
            }
        }

        let file_name = format!("{}-{}.html", year, month);
        let previous_file_name =
            previous_month.map(|((previous_year, previous_month), _events)| {
                format!("{}-{}.html", previous_year, previous_month)
            });
        let next_file_name = next_month
            .map(|((next_year, next_month), _events)| format!("{}-{}.html", next_year, next_month));

        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", &config.display_timezone.name());
        context.insert("year", &year);
        context.insert("month", &month);
        context.insert("weeks", &week_list);

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
                        .map(|path| ["month", path].join("/")),
                );
                context.insert(
                    "next_file_name",
                    &next_file_name
                        .as_ref()
                        .map(|path| ["month", path].join("/")),
                );
            } else {
                context.insert("previous_file_name", &previous_file_name);
                context.insert("next_file_name", &next_file_name);
            }

            // write the actual template
            write_template(tera, "month.html", &context, file_path)?;
        }

        Ok(())
    }
}
