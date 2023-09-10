use crate::model::event::WeekNum;
use crate::model::event::Year;
use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::Month;
use chrono::NaiveWeek;
use chrono::Weekday;
use chrono::{DateTime, Datelike, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::Result;
use itertools::Itertools;

use super::calendar_collection::CalendarCollection;
use super::day::DayContext;

/// Represents a week and generates the week context for [crate::views::week_view::WeekView]
#[derive(Debug)]
pub struct Week<'a> {
    parent_collection: &'a CalendarCollection,
    pub(crate) week: NaiveWeek,
}

impl Week<'_> {
    pub fn new(
        start: DateTime<ChronoTz>,
        parent_collection: &CalendarCollection,
    ) -> Result<Week<'_>> {
        let week = start
            .with_timezone(parent_collection.display_timezone())
            .date_naive()
            .week(Weekday::Sun);

        Ok(Week {
            parent_collection,
            week,
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

    pub(crate) fn first_day(&self) -> NaiveDate {
        self.week.first_day()
    }

    pub(crate) fn last_day(&self) -> NaiveDate {
        self.week.last_day()
    }

    pub(crate) fn week_switches_months(&self) -> bool {
        let first_day = self.week.first_day();
        let last_day = self.week.last_day();
        first_day.month() != last_day.month()
    }

    pub(crate) fn week_switches_years(&self) -> bool {
        let first_day = self.week.first_day();
        let last_day = self.week.last_day();
        first_day.year() != last_day.year()
    }

    /// This function returns the first or last day of the week based on which month/year covers more of the week
    fn first_or_last_by_majority(&self) -> NaiveDate {
        let first_day = self.week.first_day();
        let last_day = self.week.last_day();

        if last_day.day() > 3 {
            last_day
        } else {
            first_day
        }
    }

    pub(crate) fn iso_week(&self) -> WeekNum {
        self.first_day().iso_week().week() as u8
    }

    /// Returns the month based on which month has the majority of days in this [`Week`].
    ///
    /// # Panics
    ///
    /// Panics if [`Month::try_from`] receives a number it cannot handle.
    pub(crate) fn month(&self) -> Month {
        Month::try_from(self.first_or_last_by_majority().month() as u8)
            .expect("month of week out of range, this should never happen")
    }

    pub(crate) fn month_start(&self) -> Month {
        Month::try_from(self.first_day().month() as u8)
            .expect("month of week out of range, this should never happen")
    }

    pub(crate) fn month_end(&self) -> Month {
        Month::try_from(self.last_day().month() as u8)
            .expect("month of week out of range, this should never happen")
    }

    pub(crate) fn year(&self) -> Year {
        self.first_or_last_by_majority().year()
    }

    pub(crate) fn year_start(&self) -> Year {
        self.first_day().year()
    }

    pub(crate) fn year_end(&self) -> Year {
        self.last_day().year()
    }

    /// Creates an iterator to cycle through the week
    // NOTE: we are using this instead of NaieveWeek::days() since that range doesn't seem to want to behave as an iterator
    pub(crate) fn days(&self) -> impl Iterator<Item = NaiveDate> {
        DateRule::daily(self.first_day()).with_count(7)
    }

    pub fn format<'a>(&'a self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'_>> {
        self.first_day().format(fmt)
    }

    pub(crate) fn file_name(&self) -> String {
        format!("{}-{}.html", self.year_start(), self.iso_week())
    }
}
