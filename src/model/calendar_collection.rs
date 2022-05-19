use color_eyre::eyre::{self, bail, Result, WrapErr};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs::File, io::BufReader};
use tera::{Context, Tera};
use time::ext::NumericalDuration;
use time::{macros::format_description, Date, Month as MonthEnum};

use super::event::{Event, UnparsedProperties};
use crate::model::calendar::Calendar;
use crate::model::day::DayContext;
use crate::model::event::{EventContext, WeekNum, Year};
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

type WeekDayMap = BTreeMap<u8, Vec<Rc<Event>>>;

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
        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        if let Some(files) = args.file {
            for file in files {
                println!("  Provided path is: {:?}", file);
                if file.exists() {
                    println!("    File exists");
                    let buf = BufReader::new(File::open(file)?);
                    let (parsed_calendars, calendar_unparsed_properties) =
                        &mut Calendar::parse_calendars(buf)?;
                    unparsed_properties.extend(calendar_unparsed_properties.clone().into_iter());
                    calendars.append(parsed_calendars);
                }
            }
        };

        if let Some(urls) = args.url {
            for url in urls {
                println!("  Provided url is: {:?}", url);
                let ics_string = ureq::get(&url).call()?.into_string()?;
                println!("    URL exists");
                let (parsed_calendars, calendar_unparsed_properties) =
                    &mut Calendar::parse_calendars(ics_string.as_bytes())?;
                unparsed_properties.extend(calendar_unparsed_properties.clone().into_iter());
                calendars.append(parsed_calendars);
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

        // print unparsed properties
        // TODO should probably put this behind a flag
        println!(
            "The following {} properties were present but have not been parsed:",
            unparsed_properties.len()
        );
        for property in unparsed_properties {
            println!("  {}", property);
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

    pub fn create_month_pages(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Month pages path does not exist: {:?}", output_dir)
        }

        for ((year, month), events) in &self.months {
            println!("month: {}", month);
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
            template_out_file.push(PathBuf::from(format!("{}-{}.html", year, month)));

            let mut context = Context::new();
            context.insert("year", &year);
            context.insert("month", &month);
            context.insert("events", events);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("month.html", &context, File::create(template_out_file)?)?;
        }
        Ok(())
    }

    pub fn create_week_pages(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Week pages path does not exist: {:?}", output_dir)
        }

        for ((year, week), events) in &self.weeks {
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
            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(format!("{}-{}.html", year, week)));

            // create week days
            let sunday = Date::from_iso_week_date(*year, *week, time::Weekday::Sunday)?;
            let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
                .iter()
                .map(|o| {
                    DayContext::new(
                        sunday + (*o as i64).days(),
                        week_day_map
                            .get(o)
                            .map(|l| l.iter().map(|e| e.context()).collect())
                            .unwrap_or(Vec::new()),
                    )
                })
                .collect();

            let mut context = Context::new();
            context.insert("year", &year);
            // TODO handle weeks where the month changes
            context.insert("month", &sunday.month());
            context.insert("week", &week);
            context.insert("week_dates", &week_dates);
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
                day.format(format_description!("[year]-[month]-[day]"))?
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
