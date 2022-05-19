use serde::Serialize;

use super::event::EventContext;

#[derive(Debug, Serialize)]
pub struct DayContext {
    pub(crate) date: String,
    pub(crate) events: Vec<EventContext>,
}
