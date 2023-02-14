use chrono::{Datelike, Month, NaiveDate};
use num_traits::FromPrimitive;
use serde::Serialize;

use super::event::EventContext;

const YMD_FORMAT: &str = "[year]-[month]-[day]";

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
