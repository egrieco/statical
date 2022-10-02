use color_eyre::eyre::{self, bail, Context as EyreContext, Result};
use std::collections::HashSet;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::{fs::File, io::BufReader};
use tera::Tera;
use time::ext::NumericalDuration;
use time::util::days_in_year_month;
use time::OffsetDateTime;
use time::{Date, Month as MonthName};
use time_tz::Tz;

use super::event::UnparsedProperties;
use crate::config::ParsedConfig;
use crate::model::calendar::Calendar;
use crate::model::day::DayContext;
use crate::model::event::Year;
use crate::options::Opt;
use crate::util::{self, create_subdir};
use crate::views::agenda_view::AgendaView;
use crate::views::day_view::DayView;
use crate::views::month_view::MonthView;
use crate::views::week_view::{WeekDayMap, WeekView};

#[derive(Debug)]
pub struct CalendarCollection<'a> {
    calendars: Vec<Calendar>,

    // these are represented as options since the user can choose to render them or not
    months: Option<MonthView>,
    weeks: Option<WeekView>,
    days: Option<DayView>,
    agenda: Option<AgendaView>,

    tera: Tera,
    config: ParsedConfig<'a>,
    unparsed_properties: UnparsedProperties,
}

impl<'a> CalendarCollection<'a> {
    pub fn new(args: Opt, config: ParsedConfig<'a>) -> eyre::Result<CalendarCollection<'a>> {
        let mut calendars = Vec::new();
        let mut unparsed_properties = HashSet::new();

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
            .unwrap_or_else(OffsetDateTime::now_utc);
        let cal_end = calendars
            .iter()
            .map(|c| c.end())
            .reduce(|max_end, end| max_end.max(end))
            // TODO consider a better approach to finding the correct number of days
            .unwrap_or_else(|| OffsetDateTime::now_utc() + 30.days());

        // add events to views
        let months = if config.render_month {
            Some(MonthView::new(
                create_subdir(&config.output_dir, "month")?,
                &calendars,
            ))
        } else {
            None
        };

        let weeks = if config.render_week {
            Some(WeekView::new(
                create_subdir(&config.output_dir, "week")?,
                &calendars,
            ))
        } else {
            None
        };

        let days = if config.render_day {
            Some(DayView::new(
                create_subdir(&config.output_dir, "day")?,
                &calendars,
            ))
        } else {
            None
        };

        let agenda = if config.render_agenda {
            Some(AgendaView::new(
                create_subdir(&config.output_dir, "agenda")?,
                &calendars,
            ))
        } else {
            None
        };

        // expand recurring events
        for calendar in calendars.iter_mut() {
            calendar.expand_recurrences(cal_start, cal_end);
        }

        Ok(CalendarCollection {
            calendars,
            months,
            weeks,
            days,
            agenda,
            tera: Tera::new("templates/**/*.html")?,
            config,
            unparsed_properties,
        })
    }

    pub fn print_unparsed_properties(&self) {
        println!(
            "The following {} properties were present but have not been parsed:",
            self.unparsed_properties.len()
        );
        for property in &self.unparsed_properties {
            println!("  {}", property);
        }
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

    pub fn setup_output_dir(&self) -> Result<()> {
        let output_dir = &PathBuf::from(&self.config.output_dir);

        // make the output dir if it doesn't exist
        fs::create_dir_all(output_dir)
            .context(format!("could not create output dir: {:?}", output_dir))?;

        let styles_dir = util::create_subdir(output_dir, "styles")?;

        if self.config.copy_stylesheet_to_output {
            let stylesheet_destination = styles_dir.join(PathBuf::from("style.css"));
            let source_stylesheet = &&self.config.copy_stylesheet_from;
            fs::copy(source_stylesheet, &stylesheet_destination).context(format!(
                "could not copy stylesheet {:?} to destination: {:?}",
                source_stylesheet, stylesheet_destination
            ))?;
        }

        Ok(())
    }

    pub fn create_html_pages(&self) -> Result<()> {
        self.setup_output_dir()?;

        if self.months.is_some() {
            self.months
                .as_ref()
                .unwrap()
                .create_html_pages(&self.config, &self.tera)?;
        }

        if self.weeks.is_some() {
            self.weeks
                .as_ref()
                .unwrap()
                .create_html_pages(&self.config, &self.tera)?;
        }

        if self.days.is_some() {
            self.days
                .as_ref()
                .unwrap()
                .create_html_pages(&self.config, &self.tera)?;
        }

        if self.agenda.is_some() {
            self.agenda
                .as_ref()
                .unwrap()
                .create_html_pages(&self.config, &self.tera)?;
        }

        Ok(())
    }
}

/// Return the range of iso weeks this month covers
pub(crate) fn iso_weeks_for_month_display(year: &i32, month: &u8) -> Result<Range<u8>> {
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
                        .unwrap_or_default(),
                )
            })
            .collect();
        Ok(week_dates)
    }
}

/// Generate DayContext Vecs for empty weeks
pub(crate) fn blank_context(year: &i32, week: &u8) -> Result<Vec<DayContext>> {
    let sunday = first_sunday_of_week(year, week)?;
    let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
        .iter()
        .map(|o| DayContext::new(sunday + (*o as i64).days(), Vec::new()))
        .collect();
    Ok(week_dates)
}

pub(crate) fn month_from_u8(value: u8) -> Result<time::Month> {
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
