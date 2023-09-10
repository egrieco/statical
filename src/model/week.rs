use crate::model::event::WeekNum;
use crate::model::event::Year;
use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::Month;
use chrono::{DateTime, Datelike, Days, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{eyre, Result};
use itertools::Itertools;

use super::calendar_collection::CalendarCollection;
use super::day::DayContext;

#[derive(Copy, Clone, Debug)]
pub struct Week<'a> {
    parent_collection: &'a CalendarCollection,
    pub(crate) start_datetime: DateTime<ChronoTz>,
    // TODO: switch this to use chrono::NaiveWeek
    pub(crate) start: NaiveDate,
    inner_iter: DateRule<NaiveDate>,
}

impl Week<'_> {
    pub fn new(
        start: DateTime<ChronoTz>,
        parent_collection: &CalendarCollection,
    ) -> Result<Week<'_>> {
        let start_naive = start
            .with_timezone(parent_collection.display_timezone())
            .date_naive();
        let aligned_week_start = start_naive
            .checked_sub_days(Days::new(
                start_naive.weekday().num_days_from_sunday().into(),
            ))
            .ok_or(eyre!("could not create the aligned week start"))?;

        Ok(Week {
            parent_collection,
            start_datetime: start,
            start: aligned_week_start,
            inner_iter: DateRule::daily(aligned_week_start).with_count(7),
        })
    }

    pub(crate) fn week_dates(&self) -> Vec<DayContext> {
        let mut week_dates = Vec::new();
        for day in self.days() {
            let events = self
                .parent_collection
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
                            .map(|e| e.context(&self.parent_collection.config))
                            .collect()
                    })
                    .unwrap_or_default(),
            ));
        }

        week_dates
    }

    pub(crate) fn week_switches_months(&self) -> bool {
        // TODO: we could also just check if the month of the first and last day are the same
        self.inner_iter
            .into_iter()
            .group_by(|d| d.month())
            .into_iter()
            .count()
            > 1
    }

    pub(crate) fn year(&self) -> Year {
        self.start.year()
    }

    pub(crate) fn week(&self) -> WeekNum {
        self.start.iso_week().week() as u8
    }

    pub(crate) fn month(&self) -> Month {
        let mut iter_copy = self.inner_iter;
        let first_date_of_week = iter_copy.next().unwrap();

        Month::try_from(first_date_of_week.month() as u8)
            .expect("month of week out of range, this should never happen")
    }

    /// Returns the month based on which month has the majority of days in this [`Week`].
    ///
    /// # Panics
    ///
    /// Panics if [`Month::try_from`] receives a number it cannot handle.
    pub(crate) fn month_by_majority(&self) -> Month {
        let mut iter_copy = self.inner_iter;
        let first_date_of_week = iter_copy.next().unwrap();
        let last_date_of_week = iter_copy.last().unwrap();
        if last_date_of_week.day() > 3 {
            Month::try_from(last_date_of_week.month() as u8)
                .expect("month of week out of range, this should never happen")
        } else {
            // TODO: do we want to return an error or just default to the below value?
            Month::try_from(first_date_of_week.month() as u8)
                .expect("month of week out of range, this should never happen")
        }
    }

    pub(crate) fn days(&self) -> impl Iterator<Item = NaiveDate> {
        DateRule::daily(self.start_datetime)
            .with_count(7)
            .map(|d| d.naive_local().date())
    }

    pub fn format<'a>(&'a self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'_>> {
        self.start.format(fmt)
    }
}

impl Iterator for Week<'_> {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        // start.checked_add_days
        self.inner_iter.next()
    }
}
