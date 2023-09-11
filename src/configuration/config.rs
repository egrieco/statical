use chrono_tz::Tz;
use doku::Document;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fmt::{self},
    path::PathBuf,
};

use super::types::{CalendarView, ConfigDate, ConfigTimeZone, ConfigUrl};

#[derive(Debug, Deserialize, Serialize, Document)]
pub struct Config {
    /// The date that is considered "today" on the rendered calendar
    /// (defaults to today if left empty)
    ///
    /// This corresponds to page 0 on the Agenda view
    // TODO: need to add a more forgiving parser for start dates that can take human strings like "now", or "today"
    // TODO: should this be Local or Tz?
    #[doku(example = "today")]
    #[serde(deserialize_with = "super::types::deserialize_config_date")]
    pub calendar_today_date: ConfigDate,

    /// Name of the timezone in which to display rendered times
    ///
    /// See available timezones here: <https://docs.rs/chrono-tz/latest/chrono_tz/enum.Tz.html>
    #[doku(example = "America/Phoenix")]
    pub display_timezone: ConfigTimeZone,

    /// The list of calendars to import (can be files and urls)
    pub(crate) calendar_sources: Vec<CalendarSourceConfig>,

    /// The path to the output directory where files will be written.
    ///
    /// NOTE: This is relative to the config file
    #[doku(example = "output")]
    pub output_dir: PathBuf,

    /// Do not delete files in the output directory
    #[doku(example = "false")]
    pub no_delete: bool,

    /// The base url at which the site will be served
    #[doku(example = "/")]
    pub base_url_path: ConfigUrl,

    /// The path to add into the stylesheet link tag
    #[doku(example = "/styles/style.css")]
    pub stylesheet_path: ConfigUrl,

    /// Whether to copy the referenced stylesheet into the output dir
    pub copy_stylesheet_to_output: bool,

    /// The stylesheet to copy to the output dir
    ///
    /// This is mostly useful for local testing, unless you want to use a separate stylesheet for the calendar
    ///
    /// NOTE: This is relative to the config file
    #[doku(example = "public/statical.css")]
    pub copy_stylesheet_from: PathBuf,

    /// The view (Month, Week, or Day) to use for the main index page
    // TODO: consider making this case sensitive maybe with EnumString from strum_macros
    // strum_macros: https://docs.rs/strum_macros/latest/strum_macros/derive.EnumString.html
    #[doku(example = "Month")]
    pub(crate) default_calendar_view: CalendarView,

    /// Whether to render the month pages.
    pub render_month: bool,

    /// Whether to render the week pages.
    pub render_week: bool,

    /// Whether to render the day pages.
    pub render_day: bool,

    /// Whether to render the agenda pages.
    pub render_agenda: bool,

    /// The strftime format for the Month `view_date` template variable
    #[doku(example = "%B %Y")]
    pub month_view_format: String,

    /// The strftime format for the Week `view_date` template variable
    #[doku(example = "%B %Y")]
    pub week_view_format: String,

    /// The strftime format for the Day `view_date` template variable
    #[doku(example = "%A, %B %-d, %Y")]
    pub day_view_format: String,

    /// The strftime format for the Agenda `view_date_start` template variable
    #[doku(example = "%B %-d, %Y")]
    pub agenda_view_format_start: String,

    /// The strftime format for the Agenda `view_date_end` template variable
    #[doku(example = "%B %-d, %Y")]
    pub agenda_view_format_end: String,

    /// Number of events per page in agenda
    #[doku(example = "10")]
    pub agenda_events_per_page: usize,

    /// The format for the start date of calendar events
    ///
    /// Available format options: <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    // TODO: find a way to validate format strings: https://github.com/chronotope/chrono/issues/342
    #[doku(example = "%I:%M%P")]
    pub event_start_format: String,

    /// The format for the end date of calendar events
    ///
    /// Available format options: <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>
    // TODO: find a way to validate format strings: https://github.com/chronotope/chrono/issues/342
    #[doku(example = "%I:%M%P")]
    pub event_end_format: String,
}

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
            calendar_today_date: ConfigDate::now(),
            default_calendar_view: CalendarView::Month,
            stylesheet_path: "/styles/style.css".into(),
            copy_stylesheet_to_output: false,
            copy_stylesheet_from: "public/statical.css".into(),
            event_start_format: "%I:%M%P".into(),
            event_end_format: "%I:%M%P".into(),
            calendar_sources: Vec::new(),
            month_view_format: "%B %Y".into(),
            week_view_format: "%B %Y".into(),
            day_view_format: "%A, %B %-d, %Y".into(),
            agenda_view_format_start: "%B %-d, %Y".into(),
            agenda_view_format_end: "%B %-d, %Y".into(),
            base_url_path: "/".into(),
            no_delete: false,
        }
    }
}
