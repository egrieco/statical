use std::fmt;

use chrono::{DateTime, Datelike, Month, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use num_traits::FromPrimitive;
use serde::Serialize;

use super::event::{EventContext, WeekNum, Year};

const YMD_FORMAT: &str = "%Y-%m-%d";
const START_DATETIME_FORMAT: &str = "%a %B %d, %Y";

/// Type alias representing a specific day in time
// pub(crate) type Day = DateTime<Utc>;

#[derive(Debug)]
pub struct Day {
    pub(crate) start_datetime: DateTime<ChronoTz>,
    pub(crate) start: NaiveDate,
    inner_iter: DateRule<NaiveDate>,
}

impl Day {
    pub fn new(start: DateTime<ChronoTz>, tz: &ChronoTz) -> Self {
        let start_naive = start.with_timezone(tz).date_naive();

        Day {
            start_datetime: start,
            start: start_naive,
            inner_iter: DateRule::daily(start_naive).with_count(1),
        }
    }

    pub(crate) fn year(&self) -> Year {
        self.start.year()
    }

    pub(crate) fn month(&self) -> Month {
        Month::try_from(self.start.month() as u8)
            .expect("month of week out of range, this should never happen")
    }

    pub(crate) fn week(&self) -> WeekNum {
        self.start.iso_week().week() as u8
    }

    pub(crate) fn day(&self) -> u32 {
        self.start.day()
    }

    pub(crate) fn format(&self, fmt: &str) -> String {
        self.start.format(fmt).to_string()
    }
}

impl Iterator for Day {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        // start.checked_add_days
        self.inner_iter.next()
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Day: {}", self.start.format(START_DATETIME_FORMAT),)
    }
}

#[derive(Debug, Serialize)]
pub struct DayContext {
    pub(crate) date: String,
    pub(crate) day: u8,
    pub(crate) wday: String,
    pub(crate) month: String,
    pub(crate) month_name: String,
    pub(crate) is_weekend: bool,
    pub(crate) events: Vec<EventContext>,
}

impl DayContext {
    pub fn new(date: NaiveDate, events: Vec<EventContext>) -> DayContext {
        DayContext {
            date: date.format(YMD_FORMAT).to_string(),
            day: date.day() as u8,
            month: date.month().to_string(),
            month_name: Month::from_u32(date.month())
                .expect("invalid month")
                .name()
                .to_string(),
            wday: date.weekday().to_string(),
            is_weekend: date.weekday().number_from_monday() > 5,
            events,
        }
    }
}
