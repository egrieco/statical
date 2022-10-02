use color_eyre::eyre::Result;
use dedup_iter::DedupAdapter;
use std::{collections::BTreeMap, path::PathBuf};
use tera::{Context, Tera};
use time_tz::TimeZone;

use crate::{
    config::{CalendarView, ParsedConfig},
    model::{
        calendar::Calendar,
        calendar_collection::WeekContext,
        event::{EventList, WeekNum, Year},
    },
    util::write_template,
};

/// Type alias representing a specific week in time
type Week = (Year, WeekNum);

/// A BTreeMap of Vecs grouped by specific weeks
pub type WeekMap = BTreeMap<Week, EventList>;

/// A BTreeMap of Vecs grouped by specific weekday
pub type WeekDayMap = BTreeMap<u8, EventList>;

#[derive(Debug)]
pub struct WeekView {
    /// The output directory for week view files
    output_dir: PathBuf,
    week_map: WeekMap,
}

impl WeekView {
    pub fn new(output_dir: PathBuf, calendars: &Vec<Calendar>) -> Self {
        let mut week_map = BTreeMap::new();

        // add events to the week_map
        for calendar in calendars {
            for event in calendar.events() {
                week_map
                    .entry((event.year(), event.week()))
                    .or_insert(Vec::new())
                    .push(event.clone());
            }
        }

        WeekView {
            output_dir,
            week_map,
        }
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut previous_file_name: Option<String> = None;
        let mut index_written = false;

        let mut weeks_iter = self.week_map.iter().peekable();
        while let Some(((year, week), events)) = weeks_iter.next() {
            println!("week: {}", week);

            let mut week_day_map: WeekDayMap = BTreeMap::new();

            for event in events {
                println!(
                    "  event: ({} {} {}) {} {}",
                    event.start().weekday(),
                    event.year(),
                    event.week(),
                    event.summary(),
                    event.start(),
                );
                let day_of_week = event.start().weekday().number_days_from_sunday();
                week_day_map
                    .entry(day_of_week)
                    .or_insert(Vec::new())
                    .push(event.clone());
            }
            let file_name = format!("{}-{}.html", year, week);
            let next_week_opt = weeks_iter.peek();
            let next_file_name = next_week_opt.map(|((next_year, next_week), _events)| {
                format!("{}-{}.html", next_year, next_week)
            });

            // create week days
            let week_dates = week_day_map.context(year, week, config.display_timezone)?;

            let mut context = Context::new();
            context.insert("stylesheet_path", &config.stylesheet_path);
            context.insert("timezone", &config.display_timezone.name());
            context.insert("year", &year);
            // handling weeks where the month changes
            context.insert(
                "month",
                &week_dates
                    .iter()
                    .map(|d| d.month.clone())
                    .dedup()
                    .collect::<Vec<String>>()
                    .join(" - "),
            );
            context.insert("week", &week);
            context.insert("week_dates", &week_dates);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);

            write_template(
                tera,
                "week.html",
                &context,
                &self.output_dir.join(PathBuf::from(&file_name)),
            )?;

            // write the index page for the current week
            // TODO might want to write the index if next_week is None and nothing has been written yet
            if let Some(next_week) = next_week_opt {
                if !index_written {
                    let (next_year, next_week) = next_week.0;
                    // write the index file if the next month is after the current date
                    // TODO make sure that the conditional tests are correct, maybe add some tests
                    if next_year >= &config.agenda_start_date.year()
                        && next_week >= &config.agenda_start_date.iso_week()
                    {
                        write_template(
                            tera,
                            "week.html",
                            &context,
                            &self.output_dir.join(PathBuf::from("index.html")),
                        )?;
                        index_written = true;

                        // write the main index as the week view
                        if config.default_calendar_view == CalendarView::Week {
                            write_template(
                                tera,
                                "week.html",
                                &context,
                                &config.output_dir.join(PathBuf::from("index.html")),
                            )?;
                        }
                    }
                }
            }

            previous_file_name = Some(file_name);
        }

        Ok(())
    }
}
