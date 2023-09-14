use chrono_tz::Tz;
use color_eyre::eyre::Context;
use color_eyre::eyre::{eyre, Result};
use doku::Document;
use figment::providers::{Format, Serialized, Toml};
use figment::Figment;
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{
    calendar_source_config::CalendarSourceConfig,
    options::Opt,
    types::{calendar_view::CalendarView, config_time_zone::ConfigTimeZone, config_url::ConfigUrl},
};

const DEFAULT_STYLESHEET_PATH: &str = "assets/statical.css";
const DEFAULT_TEMPLATE_PATH: &str = "templates";
const DEFAULT_ASSETS_PATH: &str = "assets";

#[derive(Debug, Deserialize, Serialize, Document)]
pub struct Config {
    /// The base directory against which all other paths are resolved
    ///
    /// This is normally automatically derived from the directory in which the config file resides
    #[doku(example = ".")]
    pub base_dir: PathBuf,

    /// The date that is considered "today" on the rendered calendar
    /// (defaults to today if left empty)
    ///
    /// This corresponds to page 0 on the Agenda view
    // TODO: need to add a more forgiving parser for start dates that can take human strings like "now", or "today"
    // TODO: should this be Local or Tz?
    #[doku(example = "today")]
    pub calendar_today_date: String,

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
    #[doku(example = "assets/statical.css")]
    pub copy_stylesheet_from: PathBuf,

    /// The path for template files
    #[doku(example = "templates")]
    // it'd be great to make this a RelativePathBuf but Doku doesn't support that
    pub template_path: PathBuf,

    /// The path for template files
    #[doku(example = "assets")]
    // it'd be great to make this a RelativePathBuf but Doku doesn't support that
    pub assets_path: PathBuf,

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

    /// Whether to render the calendar feed.
    pub render_feed: bool,

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

/// Sane default values for the config struct.
impl Default for Config {
    fn default() -> Self {
        Self {
            base_dir: ".".into(),
            calendar_today_date: "today".into(),
            display_timezone: ConfigTimeZone(Tz::America__Phoenix),
            calendar_sources: Vec::new(),
            output_dir: "output".into(),
            no_delete: false,
            base_url_path: "/".into(),
            stylesheet_path: "/styles/style.css".into(),
            copy_stylesheet_to_output: true,
            copy_stylesheet_from: DEFAULT_STYLESHEET_PATH.into(),
            template_path: DEFAULT_TEMPLATE_PATH.into(),
            assets_path: DEFAULT_ASSETS_PATH.into(),
            default_calendar_view: CalendarView::Month,
            render_month: true,
            render_week: true,
            render_day: true,
            render_agenda: true,
            render_feed: true,
            month_view_format: "%B %Y".into(),
            week_view_format: "%B %Y".into(),
            day_view_format: "%A, %B %-d, %Y".into(),
            agenda_view_format_start: "%B %-d, %Y".into(),
            agenda_view_format_end: "%B %-d, %Y".into(),
            agenda_events_per_page: 10,
            event_start_format: "%I:%M%P".into(),
            event_end_format: "%I:%M%P".into(),
        }
    }
}

impl Config {
    pub fn new(config_path: &str, args: &Opt) -> Result<Config> {
        // ensure that output_dir is relative to the config file
        let config_file = PathBuf::from(config_path)
            .canonicalize()
            .wrap_err("could not canonicalize config file path")?;
        // TODO: also look into RelativePathBuf in figment::value::magic https://docs.rs/figment/0.10.10/figment/value/magic/struct.RelativePathBuf.html
        let config_dir = config_file
            .parent()
            .ok_or(eyre!("could not get parent directory of the config file"))?;

        debug!("reading configuration...");
        let figment: Figment = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(config_path))
            .admerge(Serialized::defaults(args));

        let base_dir = figment
            .find_value("base_dir")?
            .as_str()
            // join should either append the path from the config, or replace it if the specified path is absolute
            .map(|d| config_dir.join(d))
            .unwrap_or(config_dir.into())
            .canonicalize()
            .wrap_err("could not canonicalize base dir")?;

        debug!("base directory is set to: {:?}", base_dir);

        let config = figment
            .merge(Figment::new().join(("base_dir", base_dir)))
            .extract()?;

        // TODO: make this into a log statement or remove it
        eprint!("config is: {:#?}", config);

        Ok(config)
    }
}
