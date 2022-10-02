use color_eyre::eyre::Result;
use std::{collections::BTreeMap, fs::File, path::PathBuf};
use tera::{Context, Tera};
use time::{macros::format_description, Date};
use time_tz::TimeZone;

use crate::{
    config::{CalendarView, ParsedConfig},
    model::{calendar::Calendar, event::EventList},
    util::render_to,
};

/// Type alias representing a specific day in time
type Day = Date;

#[derive(Debug)]
pub struct DayView {
    /// The output directory for day view files
    output_dir: PathBuf,
    /// A BTreeMap of Vecs grouped by specific days
    day_map: BTreeMap<Day, EventList>,
}

impl DayView {
    pub fn new(output_dir: PathBuf, calendars: &Vec<Calendar>) -> Self {
        let mut day_map = BTreeMap::new();

        // add events to the day_map
        for calendar in calendars {
            for event in calendar.events() {
                day_map
                    .entry(event.start().date())
                    .or_insert(Vec::new())
                    .push(event.clone());
            }
        }

        DayView {
            output_dir,
            day_map,
        }
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut previous_file_name: Option<String> = None;
        let mut index_written = false;

        let mut days_iter = self.day_map.iter().peekable();
        while let Some((day, events)) = days_iter.next() {
            println!("day: {}", day);
            for event in events {
                println!(
                    "  event: ({} {} {}) {} {}",
                    event.start().weekday(),
                    event.year(),
                    event.week(),
                    event.summary(),
                    event.start(),
                );
            }
            let file_name = format!(
                "{}.html",
                day.format(format_description!("[year]-[month]-[day]"))?
            );
            // TODO should we raise the error on format() failing?
            let next_day_opt = days_iter.peek();
            let next_file_name = next_day_opt.map(|(next_day, _events)| {
                next_day
                    .format(format_description!("[year]-[month]-[day]"))
                    .map(|file_root| format!("{}.html", file_root))
                    .ok()
            });

            let mut template_out_file = self.output_dir.join(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("stylesheet_path", &config.stylesheet_path);
            context.insert("timezone", config.display_timezone.name());
            context.insert("year", &day.year());
            context.insert("month", &day.month());
            context.insert("day", &day.day());
            context.insert("events", events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            render_to(
                tera,
                "day.html",
                &context,
                File::create(&template_out_file)?,
            )?;

            // write the index page for the current week
            // TODO might want to write the index if next_week is None and nothing has been written yet
            if let Some(next_week) = next_day_opt {
                if !index_written {
                    let next_day = next_week.0;
                    // write the index file if the next month is after the current date
                    // TODO make sure that the conditional tests are correct, maybe add some tests
                    if next_day > &config.agenda_start_date {
                        template_out_file.pop();
                        template_out_file.push(PathBuf::from("index.html"));

                        println!("Writing template to index file: {:?}", template_out_file);
                        render_to(
                            tera,
                            "day.html",
                            &context,
                            File::create(&template_out_file)?,
                        )?;
                        index_written = true;

                        // write the main index as the day view
                        if config.default_calendar_view == CalendarView::Day {
                            template_out_file.pop();
                            template_out_file.pop();
                            template_out_file.push(PathBuf::from("index.html"));
                            println!(
                                "Writing template to main index file: {:?}",
                                template_out_file
                            );
                            render_to(
                                tera,
                                "day.html",
                                &context,
                                File::create(template_out_file)?,
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