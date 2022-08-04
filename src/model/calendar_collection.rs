use color_eyre::eyre::{self, bail, Result, WrapErr};
use dedup_iter::DedupAdapter;
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{fs::File, io::BufReader};
use tera::{Context, Tera};
use time::ext::NumericalDuration;
use time::format_description::well_known::Rfc2822;
use time::util::days_in_year_month;
use time::OffsetDateTime;
use time::{macros::format_description, Date, Month as MonthName};
use time_tz::timezones::{self, find_by_name};
use time_tz::Tz;

use super::event::{Event, UnparsedProperties};
use crate::model::calendar::Calendar;
use crate::model::day::DayContext;
use crate::model::event::{WeekNum, Year};
use crate::options::Opt;

const AGENDA_EVENTS_PER_PAGE: usize = 5;

/// Type alias representing a specific month in time
type Month = (Year, u8);
/// Type alias representing a specific week in time
type Week = (Year, WeekNum);
/// Type alias representing a specific day in time
type Day = Date;

/// A BTreeMap of Vecs grouped by specific months
type MonthMap = BTreeMap<Month, WeekMapList>;
type WeekMapList = BTreeMap<WeekNum, WeekMap>;
/// A BTreeMap of Vecs grouped by specific weeks
type WeekMap = BTreeMap<Week, Vec<Rc<Event>>>;
/// A BTreeMap of Vecs grouped by specific days
type DayMap = BTreeMap<Day, Vec<Rc<Event>>>;

type WeekDayMap = BTreeMap<u8, Vec<Rc<Event>>>;

pub struct CalendarCollection<'a> {
    calendars: Vec<Calendar>,
    display_tz: &'a Tz,
    months: MonthMap,
    weeks: WeekMap,
    days: DayMap,
    tera: Tera,
}

impl<'a> CalendarCollection<'a> {
    pub fn new(args: Opt) -> eyre::Result<CalendarCollection<'a>> {
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
        let cal_start: OffsetDateTime = calendars
            .iter()
            .map(|c| c.start())
            .reduce(|min_start, start| min_start.min(start))
            .unwrap_or(OffsetDateTime::now_utc());
        let cal_end = calendars
            .iter()
            .map(|c| c.end())
            .reduce(|max_end, end| max_end.max(end))
            // TODO consider a better approach to finding the correct number of days
            .unwrap_or(OffsetDateTime::now_utc() + 30.days());

        // add events to maps
        let mut months = MonthMap::new();
        let mut weeks = WeekMap::new();
        let mut days = DayMap::new();

        // expand recurring events
        for calendar in calendars.iter_mut() {
            calendar.expand_recurrences(cal_start, cal_end);
        }

        // add events to interval maps
        for calendar in &calendars {
            for event in calendar.events() {
                months
                    .entry((event.year(), event.start().month() as u8))
                    .or_insert(WeekMapList::new())
                    .entry(event.week())
                    .or_insert(WeekMap::new())
                    .entry((event.year(), event.week()))
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
            display_tz: timezones::db::america::PHOENIX,
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
        while let Some(((year, month), weeks)) = months_iter.next() {
            println!("month: {}", month);
            let mut week_list = Vec::new();

            // create all weeks in this month
            let weeks_for_display = iso_weeks_for_month_display(year, month)?;
            println!("From week {:?}", weeks_for_display);
            for week_num in weeks_for_display {
                match weeks.get(&week_num) {
                    Some(week_map) => {
                        println!("  Creating week {}, {} {}", week_num, month, year);
                        for ((y, w), events) in week_map {
                            let mut week_day_map: WeekDayMap = BTreeMap::new();

                            for event in events {
                                println!(
                                    "    event: ({} {} {}) {} {}",
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

                            // create week days
                            let week_dates =
                                week_day_map.context(year, &week_num, self.display_tz())?;
                            week_list.push(week_dates);
                        }
                    }
                    None => {
                        println!("  Inserting blank week {}, {} {}", week_num, month, year);
                        week_list.push(blank_context(year, &week_num)?);
                    }
                }
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
            context.insert("weeks", &week_list);
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
            let week_dates = week_day_map.context(year, week, self.display_tz())?;

            let mut context = Context::new();
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

    pub fn create_agenda_pages(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.is_dir() {
            bail!("Agenda pages path does not exist: {:?}", output_dir)
        }

        let start = OffsetDateTime::now_utc().date();

        let past_events = self
            .days
            .range(..start)
            .flat_map(|(day, events)| {
                events.iter().map(move |event| {
                    (
                        DayContext::new(*day, vec![event.context(self.display_tz)]),
                        event,
                    )
                })
            })
            .collect::<Vec<_>>();
        let mut past_events_iter = past_events
            .rchunks(AGENDA_EVENTS_PER_PAGE)
            .zip(1_isize..)
            .peekable();
        while let Some((events, page)) = past_events_iter.next() {
            println!("page: {}", page);
            for (_day, event) in events {
                println!(
                    "  event: ({} {} {}) {} {}",
                    event.start().weekday(),
                    event.year(),
                    event.week(),
                    event.summary(),
                    event.start(),
                );
            }
            let file_name = format!("{}.html", -page);
            let previous_file_name = past_events_iter
                .peek()
                .map(|(_, previous_page)| format!("{}.html", -previous_page));
            let next_file_name = format!("{}.html", 1 - page);

            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("page", &page);
            context.insert("events", events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("agenda.html", &context, File::create(template_out_file)?)?;
        }

        let future_events = self
            .days
            .range(start..)
            .flat_map(|(day, events)| {
                events.iter().map(move |event| {
                    (
                        DayContext::new(*day, vec![event.context(self.display_tz)]),
                        event,
                    )
                })
            })
            .collect::<Vec<_>>();
        if future_events.is_empty() {
            println!("page: 0");
            let previous_file_name = if past_events.is_empty() {
                None
            } else {
                Some("-1.html")
            };

            let mut template_out_file = PathBuf::new();
            template_out_file.push(output_dir);
            template_out_file.push(PathBuf::from("0.html"));

            let mut context = Context::new();
            context.insert("page", &0);
            context.insert("events", &future_events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &None::<&str>);
            println!("Writing template to file: {:?}", template_out_file);
            self.render_to("agenda.html", &context, File::create(template_out_file)?)?;
        } else {
            let mut future_events_iter = future_events
                .rchunks(AGENDA_EVENTS_PER_PAGE)
                .zip(0..)
                .peekable();
            while let Some((events, page)) = future_events_iter.next() {
                println!("page: {}", page);
                for (_day, event) in events {
                    println!(
                        "  event: ({} {} {}) {} {}",
                        event.start().weekday(),
                        event.year(),
                        event.week(),
                        event.summary(),
                        event.start(),
                    );
                }
                let file_name = format!("{}.html", page);
                let previous_file_name = if page == 0 && past_events.is_empty() {
                    None
                } else {
                    Some(format!("{}.html", page - 1))
                };
                let next_file_name = future_events_iter
                    .peek()
                    .map(|(_, next_page)| format!("{}.html", next_page));

                let mut template_out_file = PathBuf::new();
                template_out_file.push(output_dir);
                template_out_file.push(PathBuf::from(&file_name));

                let mut context = Context::new();
                context.insert("page", &page);
                context.insert("events", events);
                context.insert("previous_file_name", &previous_file_name);
                context.insert("next_file_name", &next_file_name);
                println!("Writing template to file: {:?}", template_out_file);
                self.render_to("agenda.html", &context, File::create(template_out_file)?)?;
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn display_tz(&self) -> &Tz {
        self.display_tz
    }
}

/// Return the range of iso weeks this month covers
fn iso_weeks_for_month_display(year: &i32, month: &u8) -> Result<Range<u8>> {
    let first_day = first_sunday_of_view(*year, month_from_u8(*month)?)?;
    let first_week = first_day.iso_week();
    let days_in_month = days_in_year_month(*year, month_from_u8(*month)?);
    let last_day = Date::from_calendar_date(*year, month_from_u8(*month)?, days_in_month)?;
    let last_week = last_day.iso_week();
    Ok(first_week..last_week)
}

/// Return the first Sunday that should appear in a calendar view, even if that date is in the previous month
fn first_sunday_of_view(year: Year, month: MonthName) -> Result<Date> {
    let first_day_of_month = Date::from_calendar_date(year, month, 1)?;
    let days_from_sunday = first_day_of_month.weekday().number_days_from_sunday();
    let first_day_of_view = first_day_of_month - (days_from_sunday as i64).days();
    Ok(first_day_of_view)
}

/// Return the first Sunday of the week, even if that week is in the previous month
fn first_sunday_of_week(year: &i32, week: &u8) -> Result<Date, color_eyre::Report> {
    let first_sunday_of_month = Date::from_iso_week_date(*year, *week, time::Weekday::Sunday)?;
    let first_sunday_of_view = first_sunday_of_view(*year, first_sunday_of_month.month())?;
    let sunday =
        if (first_sunday_of_month.to_julian_day() - first_sunday_of_view.to_julian_day()) >= 7 {
            first_sunday_of_month
        } else {
            first_sunday_of_view
        };
    Ok(sunday)
}

/// Generates context objects for the days of a week
///
/// Implementing this as a trait so we can call it on a typedef rather than creating a new struct.
pub trait WeekContext {
    fn context(&self, year: &i32, week: &u8, tz: &Tz) -> Result<Vec<DayContext>>;
}

impl WeekContext for WeekDayMap {
    fn context(&self, year: &i32, week: &u8, tz: &Tz) -> Result<Vec<DayContext>> {
        let sunday = first_sunday_of_week(year, week)?;
        let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
            .iter()
            .map(|o| {
                DayContext::new(
                    sunday + (*o as i64).days(),
                    self.get(o)
                        .map(|l| l.iter().map(|e| e.context(tz)).collect())
                        .unwrap_or(Vec::new()),
                )
            })
            .collect();
        Ok(week_dates)
    }
}

/// Generate DayContext Vecs for empty weeks
fn blank_context(year: &i32, week: &u8) -> Result<Vec<DayContext>> {
    let sunday = first_sunday_of_week(year, week)?;
    let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
        .iter()
        .map(|o| DayContext::new(sunday + (*o as i64).days(), Vec::new()))
        .collect();
    Ok(week_dates)
}

fn month_from_u8(value: u8) -> Result<time::Month> {
    match value {
        1 => Ok(time::Month::January),
        2 => Ok(time::Month::February),
        3 => Ok(time::Month::March),
        4 => Ok(time::Month::April),
        5 => Ok(time::Month::May),
        6 => Ok(time::Month::June),
        7 => Ok(time::Month::July),
        8 => Ok(time::Month::August),
        9 => Ok(time::Month::September),
        10 => Ok(time::Month::October),
        11 => Ok(time::Month::November),
        12 => Ok(time::Month::December),
        _ => bail!("can only convert numbers from 1-12 into months"),
    }
}
