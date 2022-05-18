use color_eyre::eyre::{self, bail, Result, WrapErr};
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use tera::Context;

use crate::model::calendar_collection::CalendarCollection;
use crate::model::event::{WeekNum, Year};
use crate::model::{calendar::Calendar, event::Event};

type WeekMap<'a> = BTreeMap<(Year, WeekNum), Vec<&'a Event>>;

pub struct WeekCollection<'a> {
    weeks: WeekMap<'a>,
}

impl WeekCollection<'_> {
    pub fn new(calendar_collection: &CalendarCollection) -> Result<WeekCollection> {
        let mut weeks: WeekMap = BTreeMap::new();

        for calendar in calendar_collection.calendars() {
            for event in calendar.events() {
                weeks
                    .entry((event.year(), event.week()))
                    .or_insert(Vec::new())
                    .push(event);
            }
        }
        Ok(WeekCollection { weeks })
    }

    pub fn create_week_pages(
        &self,
        calendar_collection: &CalendarCollection,
        output_dir: &Path,
    ) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Week pages path does not exist: {:?}", output_dir)
        }

        for ((year, week), events) in &self.weeks {
            println!("week: {}", week);
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
            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(format!("{}-{}.html", year, week)));

            let mut context = Context::new();
            context.insert("events", events);
            println!("Writing template to file: {:?}", template_out_file);
            let template_out = calendar_collection.render_to(
                "week.html",
                &context,
                File::create(template_out_file)?,
            )?;
        }
        Ok(())
    }
}
