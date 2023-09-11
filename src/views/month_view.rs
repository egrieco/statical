use chrono::Weekday::Sun;
use chrono::{DateTime, Datelike, Days, Duration, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{eyre, Result, WrapErr};
use itertools::Itertools;
use num_traits::cast::FromPrimitive;
use std::fs::create_dir_all;
use std::path::Path;
use std::{collections::BTreeMap, iter, path::PathBuf};

use super::week_view::WeekMap;
use crate::configuration::types::CalendarView;
use crate::{
    configuration::config::Config,
    model::{
        calendar_collection::{CalendarCollection, LocalDay},
        day::DayContext,
        event::Year,
    },
    util::write_template,
    views::week_view::WeekDayMap,
};

type InternalDate = NaiveDate;

/// Type alias representing a specific month in time
type Month = (Year, u8);

/// A BTreeMap of Vecs grouped by specific months
pub type MonthMap<'a> = BTreeMap<Month, WeekMap<'a>>;

/// A triple with the previous, current, and next weeks present
///
/// Note that the previous and next weeks may be None
pub type MonthSlice<'a> = &'a [Option<DateTime<ChronoTz>>];

const VIEW_PATH: &str = "month";

#[derive(Debug)]
pub struct MonthView<'a> {
    calendars: &'a CalendarCollection,
    output_dir: PathBuf,
}

impl MonthView<'_> {
    pub fn new(calendars: &CalendarCollection) -> MonthView<'_> {
        let output_dir = calendars
            .base_dir
            .join(&calendars.config.output_dir)
            .join(VIEW_PATH);
        MonthView {
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

    /// Returns the months to show of this [`MonthView`] with a `None` at the beginning and end.
    ///
    /// This makes it easier to iterate over all of the months in the view and place links to the previous and next months.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot construct the [`DateRule`] properly.
    // TODO: map the returned values to NaiveDate objects
    fn months_to_show(&self) -> Result<Vec<Option<DateTime<ChronoTz>>>, color_eyre::eyre::Error> {
        let aligned_month_start = self
            .calendars
            .cal_start
            .with_day(1)
            .ok_or(eyre!("could not get aligned start of month"))?;
        let aligned_month_end = DateRule::monthly(self.calendars.cal_end)
            .with_rolling_day(31)
            .map_err(|e| eyre!(e))
            .wrap_err("could not create an iterator with rolling day at end of month")?
            .next()
            .ok_or(eyre!("could not get end of month"))?;
        let months_to_show = DateRule::monthly(aligned_month_start)
            .with_end(aligned_month_end)
            .with_rolling_day(1)
            .map_err(|e| eyre!(e))
            .wrap_err("could not create month iterator")?;
        let chained_iter = iter::once(None)
            .chain(months_to_show.into_iter().map(Some))
            .chain(iter::once(None));
        let month_windows = chained_iter.collect::<Vec<Option<DateTime<ChronoTz>>>>();
        Ok(month_windows)
    }

    pub fn create_html_pages(&self) -> Result<()> {
        // create the subdirectory to hold the files
        create_dir_all(self.output_dir())?;

        let mut index_written = false;

        // iterate through all windows
        // TODO: for sparse traversal filter out the months with no events
        for window in self.months_to_show()?.windows(3) {
            let next_month_opt = window[2];

            let mut index_paths = vec![];

            // write the index page for the current month
            if !index_written {
                if let Some(next_month) = next_month_opt {
                    // write the index file if the next month is after the current date
                    if next_month.date_naive()
                        > self.config().calendar_today_date.with_day(1).ok_or(eyre!(
                            "could not convert agenda start date to beginning of month"
                        ))?
                    {
                        index_written = true;
                        index_paths.push(self.output_dir().join(PathBuf::from("index.html")));

                        // write the main index as the month view
                        if self.config().default_calendar_view == CalendarView::Month {
                            index_paths
                                .push(self.config().output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    index_written = true;
                    index_paths.push(self.output_dir().join(PathBuf::from("index.html")));

                    // write the main index as the month view
                    if self.config().default_calendar_view == CalendarView::Month {
                        index_paths
                            .push(self.config().output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

            // write the actual files
            self.write_view(&window, index_paths.as_slice())?;
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
    fn write_view(&self, month_slice: &MonthSlice, index_paths: &[PathBuf]) -> Result<()> {
        let previous_month = month_slice[0];
        let current_month =
            month_slice[1].expect("Current month is None. This should never happen.");
        let next_month = month_slice[2];

        println!("month: {}", current_month);
        let mut week_list = Vec::new();

        // create all weeks in this month
        let days_by_week = month_view_date_range(current_month)?.chunks(7);
        let weeks_for_display = days_by_week.into_iter();
        for (week_num, week) in weeks_for_display.enumerate() {
            println!("From week {}:", week_num);
            let mut week_dates = Vec::new();
            for day in week {
                let events = self
                    .calendars
                    .events_by_day
                    // TODO: I doubt that we need to adjust the timezone here, probably remove it
                    .get(
                        &day.with_timezone::<chrono_tz::Tz>(&self.config().display_timezone.into())
                            .date_naive(),
                    );
                println!(
                    "  For week {} day {}: there are {} events",
                    week_num,
                    day,
                    events.map(|e| e.len()).unwrap_or(0)
                );
                week_dates.push(DayContext::new(
                    day.naive_local().date(),
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
            week_list.push(week_dates);
        }

        let file_name = format!("{}-{}.html", current_month.year(), current_month.month());
        let previous_file_name = previous_month.map(|previous_month| {
            format!("{}-{}.html", previous_month.year(), previous_month.month())
        });
        let next_file_name = next_month
            .map(|next_month| format!("{}-{}.html", next_month.year(), next_month.month()));

        let mut context = self.calendars.template_context();
        context.insert(
            "view_date",
            &current_month
                .format(&self.config().month_view_format)
                .to_string(),
        );
        context.insert("year", &current_month.year());
        context.insert("month", &current_month.month());
        context.insert(
            "month_name",
            &chrono::Month::from_u8(current_month.month() as u8)
                .ok_or(eyre!("unknown month"))?
                .name(),
        );
        context.insert("weeks", &week_list);

        // create the main file path
        let binding = self.output_dir().join(PathBuf::from(&file_name));
        let mut file_paths = vec![&binding];
        // then add any additional index paths
        file_paths.extend(index_paths);

        let base_url_path: unix_path::PathBuf =
            self.calendars.config.base_url_path.path_buf().clone();

        // write the template to all specified paths
        for file_path in file_paths {
            let view_path = base_url_path.join("month");
            context.insert(
                "previous_file_name",
                &previous_file_name.as_ref().map(|path| view_path.join(path)),
            );
            context.insert(
                "next_file_name",
                &next_file_name.as_ref().map(|path| view_path.join(path)),
            );

            // write the actual template
            write_template(
                &self.calendars.tera,
                "month.html",
                &context,
                &self.calendars.base_dir.join(file_path),
            )?;
        }

        Ok(())
    }
}

/// Calendar date range
///
/// Provides the range of dates, timezone adjusted, to retrieve from the main event BTreeMap
///
/// We cannot simply sort events into a Month -> Week -> Day data structure, as in month views
/// the first and last week can contain days from the previous and next months respectively
fn month_view_date_range(month: LocalDay) -> Result<DateRule<LocalDay>> {
    // get the first day of the month
    let first_day_of_month = month
        .with_day(1)
        .ok_or(eyre!("could not get first day of month"))?;
    // get the last day of the month
    let last_day_of_month = DateRule::monthly(first_day_of_month)
        .with_rolling_day(31)
        .map_err(|e| eyre!(e))
        .wrap_err("could not create rolling day rule")?
        .next()
        .ok_or(eyre!("could not get last day of month"))?;

    // adjust the first day to the first Sunday, even if that is in the previous month
    let first_day_of_view =
        first_day_of_month - Days::new(first_day_of_month.weekday().num_days_from_sunday().into());
    // adjust the last day if that is not a Saturday, even if it is in the next month
    // TODO: double check the math for ensuring that the last day is sunday
    let last_day_of_view = last_day_of_month
        + Days::new(((7 - last_day_of_month.weekday().num_days_from_sunday()) % 7).into());

    Ok(DateRule::daily(first_day_of_view).with_end(last_day_of_view))
}

/// Return the first Sunday of the week, even if that week is in the previous month
fn first_sunday_of_week(year: &i32, week: &u32) -> Result<InternalDate, color_eyre::Report> {
    let first_sunday_of_month =
        NaiveDate::from_isoywd_opt(*year, *week, Sun).ok_or(eyre!("could not get iso week"))?;
    // let first_sunday_of_view = first_sunday_of_view(
    //     *year,
    //     Month::from_u32(first_sunday_of_month.month()).ok_or(eyre!("could not get month"))?,
    // )?;
    // let sunday =
    //     if (first_sunday_of_month.to_julian_day() - first_sunday_of_view.to_julian_day()) >= 7 {
    //         first_sunday_of_month
    //     } else {
    //         first_sunday_of_view
    //     };
    Ok(first_sunday_of_month)
}

/// Generates context objects for the days of a week
///
/// Implementing this as a trait so we can call it on a typedef rather than creating a new struct.
pub trait WeekContext {
    fn context(&self, year: &i32, week: &u8, config: &Config) -> Result<Vec<DayContext>>;
}

impl WeekContext for WeekDayMap {
    fn context(&self, year: &i32, week: &u8, config: &Config) -> Result<Vec<DayContext>> {
        let sunday = first_sunday_of_week(year, &(*week as u32))?;
        let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
            .iter()
            .map(|o| {
                DayContext::new(
                    sunday + Duration::days(*o as i64),
                    self.get(o)
                        .map(|l| l.iter().map(|e| e.context(config)).collect())
                        .unwrap_or_default(),
                )
            })
            .collect();
        Ok(week_dates)
    }
}
