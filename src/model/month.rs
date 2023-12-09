use super::{calendar_collection::CalendarCollection, event::Event};
use crate::views::{day_view, week_view};
use chrono::{Datelike, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use color_eyre::eyre::{eyre, Result};
use std::ops::Bound::Included;
use std::{cmp::Ordering, path::PathBuf, rc::Rc};

/// Represents a month
#[derive(Debug, Clone, Copy)]
pub struct Month<'a> {
    parent_collection: &'a CalendarCollection,
    pub(crate) year: i32,
    pub(crate) month: u8,
}

impl Month<'_> {
    pub fn new(parent_collection: &CalendarCollection, start: NaiveDate) -> Month<'_> {
        Month {
            parent_collection,
            year: start.year(),
            month: start.month0() as u8,
        }
    }

    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn month0(&self) -> u8 {
        self.month
    }

    pub fn month(&self) -> u8 {
        self.month + 1
    }

    pub(crate) fn timezone(&self) -> ChronoTz {
        *self.parent_collection.display_timezone()
    }

    pub(crate) fn naive_date(&self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month().into(), 1)
    }

    // based on: https://stackoverflow.com/questions/73796125/how-to-get-the-start-and-the-end-of-date-for-each-month-with-naivedate-rust
    pub(crate) fn last_day(&self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year, self.month() as u32 + 1, 1)
            .unwrap_or(NaiveDate::from_ymd_opt(self.year + 1, 1, 1)?)
            .pred_opt()
    }

    pub fn week_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        let week = self
            .naive_date()
            .expect("could not get naive date")
            .iso_week();
        PathBuf::from("/")
            .join(week_view::VIEW_PATH)
            .join(format!("{}-{}.html", week.year(), week.week()))
            .to_string_lossy()
            .to_string()
    }

    pub fn day_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        PathBuf::from("/")
            .join(day_view::VIEW_PATH)
            .join(format!(
                "{}-{:02}-{:02}.html",
                self.year(),
                self.month(),
                // TODO: need to get the same day of this week, not day of the month
                self.parent_collection.today_date().day()
            ))
            .to_string_lossy()
            .to_string()
    }

    /// Returns the first event present in this month
    pub(crate) fn first_event(&self) -> Result<Option<&Rc<Event>>> {
        let start_day = self.naive_date().ok_or(eyre!("could not get start_day"))?;
        let end_day = self.last_day().ok_or(eyre!("could not get end_day"))?;

        let first_event = self
            .parent_collection
            .events_by_day
            // TODO: I doubt that we need to adjust the timezone here, probably remove it
            .range((Included(start_day), Included(end_day)))
            .into_iter()
            .next()
            .map(|(_first_date, events)| events.first())
            .flatten();

        Ok(first_event)
    }
}

impl PartialEq<NaiveDate> for Month<'_> {
    fn eq(&self, other: &NaiveDate) -> bool {
        self.year == other.year() && self.month == other.month0() as u8
    }
}

impl PartialOrd<NaiveDate> for Month<'_> {
    fn partial_cmp(&self, other: &NaiveDate) -> Option<Ordering> {
        // compare year
        match self.year.partial_cmp(&other.year()) {
            Some(order) => match order {
                Ordering::Equal => {
                    // compare month
                    self.month.partial_cmp(&(other.month0() as u8))
                }
                Ordering::Less => Some(Ordering::Less),
                Ordering::Greater => Some(Ordering::Greater),
            },
            None => None,
        }
    }
}
