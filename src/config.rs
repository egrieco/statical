use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub render_agenda: bool,
    pub render_day: bool,
    pub render_month: bool,
    pub render_week: bool,
    pub output_dir: String,
    /// Name of the timezone used to format time
    pub display_timezone: String,
    /// Number of events per page in agenda
    pub agenda_events_per_page: usize,
    /// Agenda page 0 starts at this `yyyy-mm-dd` date (or now if empty)
    pub agenda_start_date: String,
}

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
