use doku::Document;
use serde::{Deserialize, Serialize};

// TODO: might want to us the delegate crate for some of these types: https://crates.io/crates/delegate

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Document)]
pub(crate) enum CalendarView {
    Month,
    Week,
    Day,
    Event,
    Agenda,
}
