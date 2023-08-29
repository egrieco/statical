use chrono::Weekday::Sun;
use chrono::{DateTime, Datelike, Days, Duration, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{eyre, Result, WrapErr};
use itertools::Itertools;
use num_traits::cast::FromPrimitive;
use std::{
    collections::BTreeMap,
    iter,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};

use super::week_view::WeekMap;
use crate::model::calendar_collection::LocalDay;
use crate::model::day::DayContext;
use crate::{
    config::{CalendarView, ParsedConfig},
    model::{calendar_collection::CalendarCollection, event::Year},
    util::write_template,
    views::week_view::WeekDayMap,
};
use crate::{model::event::EventList, views::week_view::Week};

type InternalDate = NaiveDate;

/// Type alias representing a specific month in time
type Month = (Year, u8);

/// A BTreeMap of Vecs grouped by specific months
pub type MonthMap = BTreeMap<Month, WeekMap>;

/// A triple with the previous, current, and next weeks present
///
/// Note that the previous and next weeks may be None
pub type MonthSlice<'a> = &'a [Option<DateTime<ChronoTz>>];

#[derive(Debug)]
pub struct MonthView<'a> {
    /// The output directory for month view files
    output_dir: PathBuf,
    calendars: &'a CalendarCollection,
}

impl MonthView<'_> {
    pub fn new(output_dir: PathBuf, calendars: &CalendarCollection) -> MonthView<'_> {
        MonthView {
            output_dir,
            calendars,
        }
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // this might be better for sparse traversal
        // get list of months to pull (sparse for now)
        // let month_list = self
        //     .calendars
        //     .iterator()
        //     .group_by(|i| (i.year(), i.month()));

        let mut month_map: BTreeMap<Month, BTreeMap<Week, EventList>> = BTreeMap::new();

        // add events to the month_map
        for events in self.calendars.events_by_day.values() {
            for event in events {
                month_map
                    .entry((event.year(), event.start().month() as u8))
                    .or_default()
                    .entry((event.year(), event.week()))
                    .or_default()
                    .push(event.clone());
            }
        }

        // months to iterate through
        // convert start date to beginning of month
        let aligned_month_start = self
            .calendars
            .cal_start
            .with_day(1)
            .ok_or(eyre!("could not get aligned start of month"))?;
        // let aligned_month_end = calendars.end
        let aligned_month_end = DateRule::monthly(self.calendars.cal_end)
            .with_rolling_day(31)
            .unwrap()
            .next()
            .ok_or(eyre!("could not get end of month"))?;

        // DateRule::monthly(aligned_month_start).with_end(calenda)
        // this will work for non-sparse calendar traversal
        let months_to_show = DateRule::monthly(aligned_month_start)
            .with_end(aligned_month_end)
            .with_rolling_day(1)
            .map_err(|e| eyre!(e))
            .wrap_err("could not create month iterator")?;

        // chain a None to the list of weeks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(months_to_show.into_iter().map(Some))
            .chain(iter::once(None));
        let month_windows = &chained_iter.collect::<Vec<Option<DateTime<ChronoTz>>>>();

        // iterate through all windows
        for window in month_windows.windows(3) {
            let next_month_opt = window[2];

            let mut index_paths = vec![];

            // write the index page for the current month
            if !index_written {
                if let Some(next_month) = next_month_opt {
                    // write the index file if the next month is after the current date
                    if next_month
                        > config.agenda_start_date.with_day(1).ok_or(eyre!(
                            "could not convert agenda start date to beginning of month"
                        ))?
                    {
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
                    .get(&day.with_timezone(&config.display_timezone).date_naive());
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

        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", &config.display_timezone.name());
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
    fn context(&self, year: &i32, week: &u8, config: &ParsedConfig) -> Result<Vec<DayContext>>;
}

impl WeekContext for WeekDayMap {
    fn context(&self, year: &i32, week: &u8, config: &ParsedConfig) -> Result<Vec<DayContext>> {
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
