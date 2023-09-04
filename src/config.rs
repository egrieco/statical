use chrono::{DateTime, Local};
use chrono_tz::Tz;
use doku::Document;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fmt::{self},
    ops::Deref,
    path::PathBuf,
};

/// Wrapper type for chrono_tz::Tz so we can use doku to generate example config files
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigTimeZone(chrono_tz::Tz);

impl Deref for ConfigTimeZone {
    type Target = chrono_tz::Tz;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ConfigTimeZone> for chrono_tz::Tz {
    fn from(value: ConfigTimeZone) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Document)]
pub enum CalendarView {
    Month,
    Week,
    Day,
    Agenda,
}

#[derive(Debug, Deserialize, Serialize, Document)]
pub struct Config {
    /// Flag to control rendering of the agenda pages.
    pub render_agenda: bool,

    /// Flag to control rendering of the day pages.
    pub render_day: bool,

    /// Flag to control rendering of the month pages.
    pub render_month: bool,

    /// Flag to control rendering of the week pages.
    pub render_week: bool,

    /// The path to the output directory where files will be written.
    pub output_dir: PathBuf,

    /// Name of the timezone used to format time
    pub display_timezone: ConfigTimeZone,

    /// Number of events per page in agenda
    pub agenda_events_per_page: usize,

    /// Agenda page 0 starts at this `yyyy-mm-dd` date (or now if empty)
    // TODO: need to add a more forgiving parser for start dates that can take human strings like "now", or "today"
    // TODO: should this be Local or Tz?
    pub agenda_start_date: DateTime<Local>,

    /// The view (month, week, or day) to use for the main index page
    // TODO: consider making this case sensitive maybe with EnumString from strum_macros
    // strum_macros: https://docs.rs/strum_macros/latest/strum_macros/derive.EnumString.html
    pub default_calendar_view: CalendarView,

    /// The path to add into the stylesheet link tag
    pub stylesheet_path: PathBuf,

    /// Whether to copy the referenced stylesheet into the output dir
    pub copy_stylesheet_to_output: bool,

    /// The stylesheet to copy to the output dir
    pub copy_stylesheet_from: PathBuf,

    /// The format for the start date of calendar events
    // TODO: find a way to validate format strings: https://github.com/chronotope/chrono/issues/342
    pub event_start_format: String,

    /// The format for the end date of calendar events
    // TODO: find a way to validate format strings: https://github.com/chronotope/chrono/issues/342
    pub event_end_format: String,

    /// The list of calendars to import (can be files and urls)
    pub(crate) calendar_sources: Vec<CalendarSourceConfig>,
}

/// A Config item representing a calendar source
#[derive(Debug, Deserialize, Serialize, Document)]
pub struct CalendarSourceConfig {
    /// The url or file path of the calendar
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

/// Sane default values for the config struct.
impl Default for Config {
    fn default() -> Self {
        Self {
            render_agenda: true,
            render_day: true,
            render_month: true,
            render_week: true,
            output_dir: "output".into(),
            display_timezone: ConfigTimeZone(Tz::GMT),
            agenda_events_per_page: 5,
            agenda_start_date: Local::now(),
            default_calendar_view: CalendarView::Month,
            stylesheet_path: "/styles/style.css".into(),
            copy_stylesheet_to_output: false,
            copy_stylesheet_from: "public/statical.css".into(),
            event_start_format: "%I:%M%P".into(),
            event_end_format: "%I:%M%P".into(),
            calendar_sources: Vec::new(),
        }
    }
}

impl doku::Document for ConfigTimeZone {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}
