use chrono::{DateTime, Datelike, Month, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use num_traits::FromPrimitive;
use serde::Serialize;
use std::{fmt, path::PathBuf};

use crate::views::{day_view, month_view, week_view};

use super::event::EventContext;

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

    pub(crate) fn month_num(&self) -> u8 {
        self.start.month() as u8
    }

    pub(crate) fn month(&self) -> Month {
        Month::try_from(self.month_num())
            .expect("month of week out of range, this should never happen")
    }

    pub(crate) fn format(&self, fmt: &str) -> String {
        self.start.format(fmt).to_string()
    }

    pub fn week_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        let week = self.start.iso_week();
        PathBuf::from("/")
            .join(week_view::VIEW_PATH)
            .join(format!("{}-{}.html", week.year(), week.week0()))
            .to_string_lossy()
            .to_string()
    }

    pub(crate) fn month_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        PathBuf::from("/")
            .join(month_view::VIEW_PATH)
            .join(format!("{}-{}.html", self.start.year(), self.month_num()))
            .to_string_lossy()
            .to_string()
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
    pub(crate) link: String,
    pub(crate) wday: String,
    pub(crate) month: String,
    pub(crate) month_name: String,
    pub(crate) is_weekend: bool,
    pub(crate) events: Vec<EventContext>,
}

impl DayContext {
    pub fn new(date: NaiveDate, events: Vec<EventContext>) -> DayContext {
        let mut file_path = PathBuf::from("/")
            .join(day_view::VIEW_PATH)
            .join(date.format(day_view::YMD_FORMAT).to_string());
        file_path.set_extension("html");

        DayContext {
            date: date.format(YMD_FORMAT).to_string(),
            day: date.day() as u8,
            link: file_path.to_string_lossy().to_string(),
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
