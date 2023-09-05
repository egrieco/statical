use crate::model::event::WeekNum;
use crate::model::event::Year;
use chrono::format::{DelayedFormat, StrftimeItems};
use chrono::Month;
use chrono::{DateTime, Datelike, Days, NaiveDate};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{eyre, Result};

#[derive(Copy, Clone, Debug)]
pub struct Week {
    pub(crate) start_datetime: DateTime<ChronoTz>,
    pub(crate) start: NaiveDate,
    inner_iter: DateRule<NaiveDate>,
}

impl Week {
    pub fn new(start: DateTime<ChronoTz>, tz: &ChronoTz) -> Result<Self> {
        let start_naive = start.with_timezone(tz).date_naive();
        let aligned_week_start = start_naive
            .checked_sub_days(Days::new(
                start_naive.weekday().num_days_from_sunday().into(),
            ))
            .ok_or(eyre!("could not create the aligned week start"))?;

        Ok(Week {
            start_datetime: start,
            start: aligned_week_start,
            inner_iter: DateRule::daily(aligned_week_start).with_count(7),
        })
    }

    pub(crate) fn year(&self) -> Year {
        self.start.year()
    }

    pub(crate) fn week(&self) -> WeekNum {
        self.start.iso_week().week() as u8
    }

    pub(crate) fn month(&self) -> Month {
        Month::try_from(self.start.month() as u8)
            .expect("month of week out of range, this should never happen")
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

impl Iterator for Week {
    type Item = NaiveDate;

    fn next(&mut self) -> Option<Self::Item> {
        // start.checked_add_days
        self.inner_iter.next()
    }
}
