use serde::{Deserialize, Serialize};

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
        }
    }
}
