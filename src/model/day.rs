use serde::Serialize;
use time::{macros::format_description, Date};

use super::event::EventContext;

#[derive(Debug, Serialize)]
pub struct DayContext {
    pub(crate) date: String,
    pub(crate) day: u8,
    pub(crate) wday: String,
    pub(crate) month: String,
    pub(crate) events: Vec<EventContext>,
}

impl DayContext {
    pub fn new(date: Date, events: Vec<EventContext>) -> DayContext {
        DayContext {
            date: date
                .format(format_description!("[year]-[month]-[day]"))
                .unwrap_or_else(|_| "bad date".to_string()),
            day: date.day(),
            month: date.month().to_string(),
            wday: date.weekday().to_string(),
            events,
        }
    }
}
