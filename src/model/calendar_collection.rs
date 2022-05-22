use chrono::TimeZone;
use chrono_tz::UTC;
use color_eyre::eyre::{self, bail, Result, WrapErr};
use rrule::DateFilter;
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs::File, io::BufReader};
use tera::{Context, Tera};
use time::ext::NumericalDuration;
use time::format_description::well_known::Rfc2822;
use time::OffsetDateTime;
use time::{macros::format_description, Date};

use super::event::{Event, UnparsedProperties};
use crate::model::calendar::Calendar;
use crate::model::day::DayContext;
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
                if file.exists() {
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
                let ics_string = ureq::get(&url).call()?.into_string()?;
                let (parsed_calendars, calendar_unparsed_properties) =
                    &mut Calendar::parse_calendars(ics_string.as_bytes())?;
                unparsed_properties.extend(calendar_unparsed_properties.clone().into_iter());
                calendars.append(parsed_calendars);
            }
        }

        // get start and end date for entire collection
        let cal_start: OffsetDateTime = dbg!(calendars
            .iter()
            .map(|c| c.start())
            .reduce(|min_start, start| min_start.min(start))
            .unwrap_or(OffsetDateTime::now_utc()));
        let cal_end = dbg!(calendars
            .iter()
            .map(|c| c.end())
            .reduce(|max_end, end| max_end.max(end))
            // TODO consider a better approach to finding the correct number of days
            .unwrap_or(OffsetDateTime::now_utc() + 30.days()));

        // add events to maps
        let mut months = MonthMap::new();
        let mut weeks = WeekMap::new();
        let mut days = DayMap::new();
        let mut repeating_events: Vec<Rc<Event>> = Vec::new();

        for calendar in &calendars {
            for event in calendar.events() {
                if let Some(rrule) = event.rrule() {
                    // TODO might want to make this a map based on UID
                    println!("Event with rrule found: {:#?}", event);
                    repeating_events.push(event.clone());
                }

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

        // add event recurrences
        println!(
            "{} recurring events from {:?} to {:?}",
            repeating_events.len(),
            cal_start.format(&Rfc2822),
            cal_end.format(&Rfc2822)
        );
        // we need to convert from the time-rs library to chrono for RRule's sake
        let repeat_start = UTC.timestamp(cal_start.unix_timestamp(), 0);
        let repeat_end = UTC.timestamp(cal_end.unix_timestamp(), 0);
        for event in repeating_events {
            for recuring_event in event
                .rrule()
                .unwrap()
                // setting inclusive to false to prevent injecting duplicate events
                .all_between(repeat_start, repeat_end, false)
            {
                // add event to groups
                println!("{:#?}", recuring_event);
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
        // expand events with RRules

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

        let mut previous_file_name: Option<String> = None;

        let mut months_iter = self.months.iter().peekable();
        while let Some(((year, month), events)) = months_iter.next() {
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
            let file_name = format!("{}-{}.html", year, month);
            let next_file_name = months_iter
                .peek()
                .map(|((next_year, next_month), _events)| {
                    format!("{}-{}.html", next_year, next_month)
                });
            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("year", &year);
            context.insert("month", &month);
            context.insert("events", events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("month.html", &context, File::create(template_out_file)?)?;
            previous_file_name = Some(file_name);
        }
        Ok(())
    }

    pub fn create_week_pages(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Week pages path does not exist: {:?}", output_dir)
        }

        let mut previous_file_name: Option<String> = None;

        let mut weeks_iter = self.weeks.iter().peekable();
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
            let next_file_name = weeks_iter.peek().map(|((next_year, next_week), _events)| {
                format!("{}-{}.html", next_year, next_week)
            });
            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(&file_name));

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
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("week.html", &context, File::create(template_out_file)?)?;
            previous_file_name = Some(file_name);
        }
        Ok(())
    }

    pub fn create_day_pages(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Day pages path does not exist: {:?}", output_dir)
        }

        let mut previous_file_name: Option<String> = None;

        let mut days_iter = self.days.iter().peekable();
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
            let next_file_name = days_iter.peek().map(|(next_day, _events)| {
                next_day
                    .format(format_description!("[year]-[month]-[day]"))
                    .map(|file_root| format!("{}.html", file_root))
                    .ok()
            });

            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("year", &day.year());
            context.insert("month", &day.month());
            context.insert("day", &day.day());
            context.insert("events", events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("day.html", &context, File::create(template_out_file)?)?;
            previous_file_name = Some(file_name);
        }
        Ok(())
    }
}
