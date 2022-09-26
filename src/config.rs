use color_eyre::eyre::{bail, eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::{Date, OffsetDateTime};
use time_tz::Tz;

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
        }
    }
}

impl Config {
    pub fn parse(&self) -> Result<ParsedConfig> {
        let output_dir = PathBuf::from(&self.output_dir);
        let display_timezone = time_tz::timezones::get_by_name(&self.display_timezone)
            .ok_or(eyre!("unknown timezone"))?;
        let agenda_start_date = if self.agenda_start_date.is_empty() {
            OffsetDateTime::now_utc().date()
        } else {
            Date::parse(
                &self.agenda_start_date,
                &time::format_description::parse("[year]-[month]-[day]")
                    .expect("could not parse time format description"),
            )
            .context("invalid agenda start date in config")?
        };
        let stylesheet_path = PathBuf::from(&self.stylesheet_path);
        let copy_stylesheet_from = PathBuf::from(&self.copy_stylesheet_from);

        Ok(ParsedConfig {
            output_dir,
            display_timezone,
            agenda_start_date,
            stylesheet_path,
            copy_stylesheet_from,
            default_calendar_view: parse_calendar_view(&self.default_calendar_view)?,
            render_agenda: self.render_agenda,
            render_day: self.render_day,
            render_month: self.render_month,
            render_week: self.render_week,
            agenda_events_per_page: self.agenda_events_per_page,
            copy_stylesheet_to_output: self.copy_stylesheet_to_output,
        })
    }
}

pub enum CalendarView {
    Month,
    Week,
    Day,
    Agenda,
}

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

pub struct ParsedConfig<'a> {
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
    pub display_timezone: &'a Tz,
    /// Number of events per page in agenda
    pub agenda_events_per_page: usize,
    /// Agenda page 0 starts at this `yyyy-mm-dd` date (or now if empty)
    pub agenda_start_date: Date,
    /// The view (month, week, or day) to use for the main index page
    pub default_calendar_view: CalendarView,
    /// The path to add into the stylesheet link tag
    pub stylesheet_path: PathBuf,
    /// Whether to copy the referenced stylesheet into the output dir
    pub copy_stylesheet_to_output: bool,
    /// The stylesheet to copy to the output dir
    pub copy_stylesheet_from: PathBuf,
}
