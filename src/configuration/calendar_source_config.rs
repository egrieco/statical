use doku::Document;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fmt::{self},
};

/// A Config item representing a calendar source
#[derive(Debug, Deserialize, Serialize, Document)]
pub struct CalendarSourceConfig {
    /// The url or file path of the calendar
    ///
    /// NOTE: File paths are relative to the config file
    #[doku(
        example = "calendars/mycalendar_file.ics",
        example = "https://example.com/my/calendar/url/ical/"
    )]
    pub source: String,
}

impl fmt::Display for CalendarSourceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source,)
    }
}

impl<'a> From<&'a CalendarSourceConfig> for &'a str {
    fn from(value: &'a CalendarSourceConfig) -> &str {
        &value.source
    }
}

impl From<&CalendarSourceConfig> for String {
    fn from(value: &CalendarSourceConfig) -> Self {
        value.source.clone()
    }
}

impl AsRef<OsStr> for CalendarSourceConfig {
    fn as_ref(&self) -> &std::ffi::OsStr {
        OsStr::new(&self.source)
    }
}
