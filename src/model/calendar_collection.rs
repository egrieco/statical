use color_eyre::eyre::{self, bail, Result, WrapErr};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs::File, io::BufReader};
use tera::{Context, Tera};
use time::{format_description, Date, Month as MonthEnum};

use super::event::Event;
use crate::model::calendar::Calendar;
use crate::model::event::{WeekNum, Year};
use crate::options::Opt;

/// Type alias representing a specific month in time
type Month = (Year, u8);
/// Type alias representing a specific week in time
type Week = (Year, WeekNum);
/// Type alias representing a specific day in time
type Day = Date;

/// A BTreeMap of Vecs grouped by specific months
type MonthMap = BTreeMap<Month, Vec<Rc<Event>>>;
/// A BTreeMap of Vecs grouped by specific weeks
type WeekMap = BTreeMap<Week, Vec<Rc<Event>>>;
/// A BTreeMap of Vecs grouped by specific days
type DayMap = BTreeMap<Day, Vec<Rc<Event>>>;

pub struct CalendarCollection {
    calendars: Vec<Calendar>,
    months: MonthMap,
    weeks: WeekMap,
    days: DayMap,
    tera: Tera,
}

impl CalendarCollection {
    pub fn new(args: Opt) -> eyre::Result<CalendarCollection> {
        let mut calendars = Vec::new();

        if let Some(files) = args.file {
            for file in files {
                println!("  Provided path is: {:?}", file);
                if file.exists() {
                    println!("    File exists");
                    let buf = BufReader::new(File::open(file)?);
                    calendars.append(&mut Calendar::parse_calendars(buf)?);
                }
            }
        };

        if let Some(urls) = args.url {
            for url in urls {
                println!("  Provided url is: {:?}", url);
                let ics_string = ureq::get(&url).call()?.into_string()?;
                println!("    URL exists");
                calendars.append(&mut Calendar::parse_calendars(ics_string.as_bytes())?);
            }
        }

        // add events to maps
        let mut months = MonthMap::new();
        let mut weeks = WeekMap::new();
        let mut days = DayMap::new();

        for calendar in &calendars {
            for event in calendar.events() {
                months
                    .entry((event.year(), event.start().month() as u8))
                    .or_insert(Vec::new())
                    .push(event.clone());

                weeks
                    .entry((event.year(), event.week()))
                    .or_insert(Vec::new())
                    .push(event.clone());

                days.entry(event.start().date())
                    .or_insert(Vec::new())
                    .push(event.clone());
            }
        }

        Ok(CalendarCollection {
            calendars,
            months,
            weeks,
            days,
            tera: Tera::new("templates/**/*.html")?,
        })
    }

    /// Get a reference to the calendar collection's calendars.
    #[must_use]
    pub fn calendars(&self) -> &[Calendar] {
        self.calendars.as_ref()
    }

    /// Get a reference to the calendar collection's tera.
    #[must_use]
    pub fn tera(&self) -> &Tera {
        &self.tera
    }

    pub fn render(&self, template_name: &str, context: &tera::Context) -> eyre::Result<String> {
        Ok(self.tera.render(template_name, context)?)
    }

    pub fn render_to(
        &self,
        template_name: &str,
        context: &tera::Context,
        write: impl Write,
    ) -> eyre::Result<()> {
        Ok(self.tera.render_to(template_name, context, write)?)
    }

    pub fn create_week_pages(&self, output_dir: &Path) -> Result<()> {
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
            self.render_to("week.html", &context, File::create(template_out_file)?)?;
        }
        Ok(())
    }

    pub fn create_day_pages(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Day pages path does not exist: {:?}", output_dir)
        }

        for (day, events) in &self.days {
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
            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(format!(
                "{}.html",
                day.format(&format_description::parse("[year]-[month]-[day]")?)?
            )));

            let mut context = Context::new();
            context.insert("year", &day.year());
            context.insert("month", &day.month());
            context.insert("day", &day.day());
            context.insert("events", events);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("day.html", &context, File::create(template_out_file)?)?;
        }
        Ok(())
    }
}
