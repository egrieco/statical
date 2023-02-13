use chrono::{DateTime, Local, NaiveDateTime};
use chrono_tz::Tz;
use color_eyre::eyre::{bail, eyre, Context as EyreContext, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use toml_edit::{Array, Document};

use crate::model::calendar_source::CalendarSource;

/// A struct containing the configuration options.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub output_dir: String,
    /// Name of the timezone used to format time
    pub display_timezone: String,
    /// Number of events per page in agenda
    pub agenda_events_per_page: usize,
    /// Agenda page 0 starts at this `yyyy-mm-dd` date (or now if empty)
    pub agenda_start_date: String,
    /// The view (month, week, or day) to use for the main index page
    pub default_calendar_view: String,
    /// The path to add into the stylesheet link tag
    pub stylesheet_path: String,
    /// Whether to copy the referenced stylesheet into the output dir
    pub copy_stylesheet_to_output: bool,
    /// The stylesheet to copy to the output dir
    pub copy_stylesheet_from: String,
    /// The list of calendars to import (can be files and urls)
    pub calendar_sources: Vec<String>,
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
            display_timezone: "GMT".into(),
            agenda_events_per_page: 5,
            agenda_start_date: String::new(),
            default_calendar_view: "month".into(),
            stylesheet_path: "/styles/style.css".into(),
            copy_stylesheet_to_output: false,
            copy_stylesheet_from: "public/statical.css".into(),
            calendar_sources: Vec::new(),
        }
    }
}

impl From<&Document> for Config {
    fn from(doc: &Document) -> Self {
        Self {
            render_agenda: doc["render_agenda"].as_bool().unwrap_or(true),
            render_day: doc["render_day"].as_bool().unwrap_or(true),
            render_month: doc["render_month"].as_bool().unwrap_or(true),
            render_week: doc["render_week"].as_bool().unwrap_or(true),
            output_dir: doc["output_dir"].as_str().unwrap_or("output").into(),
            display_timezone: doc["display_timezone"].as_str().unwrap_or("GMT").into(),
            agenda_events_per_page: doc["agenda_events_per_page"].as_integer().unwrap_or(5)
                as usize,
            // TODO make this take human dates e.g. "today"
            agenda_start_date: doc["agenda_start_date"].as_str().unwrap_or("").into(),
            default_calendar_view: doc["default_calendar_view"]
                .as_str()
                .unwrap_or("month")
                .into(),
            stylesheet_path: doc["stylesheet_path"]
                .as_str()
                .unwrap_or("/styles/style.css")
                .into(),
            copy_stylesheet_to_output: doc["copy_stylesheet_to_output"].as_bool().unwrap_or(false),
            copy_stylesheet_from: doc["copy_stylesheet_from"]
                .as_str()
                .unwrap_or("public/statical.css")
                .into(),
            calendar_sources: doc["calendar_sources"]
                .as_array()
                .unwrap_or(&Array::new())
                .into_iter()
                .filter_map(|i| i.as_str())
                .map(|i| i.to_string())
                .collect(),
        }
    }
}

impl Config {
    pub fn parse(&self) -> Result<ParsedConfig> {
        let output_dir = PathBuf::from(&self.output_dir);
        let display_timezone: chrono_tz::Tz = self
            .display_timezone
            .parse::<chrono_tz::Tz>()
            .expect("could not parse display timezone");
        // TODO parse this into the config specified timezone
        let agenda_start_date = if self.agenda_start_date.is_empty() {
            Local::now()
        } else {
            // TODO need to add a more forgiving parser

            NaiveDateTime::parse_from_str(&self.agenda_start_date, "%Y-%m-%d")
                .context("invalid agenda start date in config")?
                .and_local_timezone(Local)
                .single()
                .ok_or(eyre!("ambiguous agenda start date"))?
        };
        let stylesheet_path = PathBuf::from(&self.stylesheet_path);
        let copy_stylesheet_from = PathBuf::from(&self.copy_stylesheet_from);
        let calendars = CalendarSource::from_strings(self.calendar_sources.clone());
        // TODO need to show calendar source errors to user

        Ok(ParsedConfig {
            render_agenda: self.render_agenda,
            render_day: self.render_day,
            render_month: self.render_month,
            render_week: self.render_week,
            output_dir,
            display_timezone,
            agenda_events_per_page: self.agenda_events_per_page,
            agenda_start_date,
            default_calendar_view: parse_calendar_view(&self.default_calendar_view)?,
            stylesheet_path,
            copy_stylesheet_to_output: self.copy_stylesheet_to_output,
            copy_stylesheet_from,
            calendar_sources: calendars.into_iter().filter_map(|c| c.ok()).collect(),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CalendarView {
    Month,
    Week,
    Day,
    Agenda,
}

// consider replacing with EnumString in strum_macros: https://docs.rs/strum_macros/latest/strum_macros/derive.EnumString.html
fn parse_calendar_view(view: &str) -> Result<CalendarView> {
    // TODO normalize case and strip whitespace
    match view {
        "month" => Ok(CalendarView::Month),
        "week" => Ok(CalendarView::Week),
        "day" => Ok(CalendarView::Day),
        "agenda" => Ok(CalendarView::Agenda),
        _ => bail!("could not parse calendar view"),
    }
}

#[derive(Debug)]
pub struct ParsedConfig {
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
    pub display_timezone: Tz,
    /// Number of events per page in agenda
    pub agenda_events_per_page: usize,
    /// Agenda page 0 starts at this `yyyy-mm-dd` date (or now if empty)
    pub agenda_start_date: DateTime<Local>,
    /// The view (month, week, or day) to use for the main index page
    pub default_calendar_view: CalendarView,
    /// The path to add into the stylesheet link tag
    pub stylesheet_path: PathBuf,
    /// Whether to copy the referenced stylesheet into the output dir
    pub copy_stylesheet_to_output: bool,
    /// The stylesheet to copy to the output dir
    pub copy_stylesheet_from: PathBuf,
    /// The list of calendars to import (can be files and urls)
    pub(crate) calendar_sources: Vec<CalendarSource>,
}
