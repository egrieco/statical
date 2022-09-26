use color_eyre::eyre::Result;
use std::{
    collections::{btree_map, BTreeMap},
    fs::File,
    ops::RangeBounds,
    path::PathBuf,
    rc::Rc,
};
use tera::{Context, Tera};
use time::{macros::format_description, Date};
use time_tz::TimeZone;

use crate::{
    config::{CalendarView, ParsedConfig},
    model::event::{Event, EventList},
    util::{self, render_to},
};

/// Type alias representing a specific day in time
type Day = Date;

#[derive(Debug)]
pub struct DayView {
    /// A BTreeMap of Vecs grouped by specific days
    day_map: BTreeMap<Day, EventList>,
}

impl Default for DayView {
    fn default() -> Self {
        Self::new()
    }
}

impl DayView {
    pub fn new() -> Self {
        let day_map = BTreeMap::new();
        DayView { day_map }
    }

    pub fn add_event(&mut self, event: &Rc<Event>) {
        self.day_map
            .entry(event.start().date())
            .or_insert(Vec::new())
            .push(event.clone());
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let output_dir = util::create_subdir(&config.output_dir, "day")?;

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

            let mut template_out_file = output_dir.join(PathBuf::from(&file_name));

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

    pub(crate) fn range<R>(&self, range: R) -> btree_map::Range<Day, EventList>
    where
        R: RangeBounds<Day>,
    {
        self.day_map.range(range)
    }
}
