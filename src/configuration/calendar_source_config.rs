use doku::Document;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fmt::{self},
};

use super::types::config_color::ConfigColor;

/// A Config item representing a calendar source
#[derive(Debug, Deserialize, Serialize, Document, PartialEq, Eq)]
pub struct CalendarSourceConfig {
    /// The url or file path of the calendar
    ///
    /// NOTE: File paths are relative to the config file
    #[doku(
        example = "calendars/mycalendar_file.ics",
        example = "https://example.com/my/calendar/url/ical/"
    )]
    pub source: String,

    /// The name or internal identifier of the calendar
    ///
    /// Because this is to be used internally, there are a few restrictions
    ///
    /// 1. No spaces
    /// 2. Dash separated a.k.a. kebab-case
    /// 3. Must be unique per config file
    pub name: String,

    /// The user facing title for the calendar
    ///
    /// This will be pulled from the calendar if it contains a `X-WR-CALNAME` property.
    /// If you provide a title here, it will override any calendar provided title.
    pub title: Option<String>,

    /// Any valid CSS color notation
    pub(crate) color: ConfigColor,
}

// TODO: need to update this function for new fields
impl fmt::Display for CalendarSourceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source,)
    }
}

// TODO: need to update this function for new fields
impl<'a> From<&'a CalendarSourceConfig> for &'a str {
    fn from(value: &'a CalendarSourceConfig) -> &str {
        &value.source
    }
}

// TODO: need to update this function for new fields
impl From<&CalendarSourceConfig> for String {
    fn from(value: &CalendarSourceConfig) -> Self {
        value.source.clone()
    }
}

// TODO: need to update this function for new fields
impl AsRef<OsStr> for CalendarSourceConfig {
    fn as_ref(&self) -> &std::ffi::OsStr {
        OsStr::new(&self.source)
    }
}
