use chrono::Weekday::Sun;
use chrono::{Datelike, Days, Duration, Month, NaiveDate, Utc};
use chrono_tz::Tz;
use chronoutil::DateRule;
use color_eyre::eyre::{self, bail, eyre, Context as EyreContext, Result};
use std::collections::HashSet;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::{fs::File, io::BufReader};
use tera::Tera;

use super::calendar_source::CalendarSource;
use super::event::UnparsedProperties;
use crate::config::ParsedConfig;
use crate::model::calendar::Calendar;
use crate::model::calendar_source::CalendarSource::*;
use crate::model::day::DayContext;
use crate::model::event::Year;
use crate::options::Opt;
use crate::util::{self, create_subdir};
use crate::views::agenda_view::AgendaView;
use crate::views::day_view::DayView;
use crate::views::month_view::MonthView;
use crate::views::week_view::{WeekDayMap, WeekView};

type InternalDate = NaiveDate;

#[derive(Debug)]
pub struct CalendarCollection {
    calendars: Vec<Calendar>,

    // these are represented as options since the user can choose to render them or not
    months: Option<MonthView>,
    weeks: Option<WeekView>,
    days: Option<DayView>,
    agenda: Option<AgendaView>,

    tera: Tera,
    config: ParsedConfig,
    unparsed_properties: UnparsedProperties,
}

impl CalendarCollection {
    pub fn new(args: Opt, config: ParsedConfig) -> eyre::Result<CalendarCollection> {
        let mut calendars = Vec::new();
        let mut unparsed_properties = HashSet::new();

        // add sources from config file
        let calendar_sources = &config.calendar_sources;
        log::debug!("config calendar sources: {:?}", calendar_sources);

        // read calendar sources from cli options
        let cli_sources: Vec<CalendarSource> = if let Some(arg_sources) = args.source {
            CalendarSource::from_strings(arg_sources)?
        } else {
            Vec::new()
        };
        log::debug!("cli arg calendar sources: {:?}", cli_sources);

        // parse calendars from all sources
        for source in calendar_sources.iter().chain(cli_sources.iter()) {
            match source {
                CalendarFile(file) => {
                    log::info!("reading calendar file: {:?}", file);
                    let buf = BufReader::new(File::open(file)?);
                    let (parsed_calendars, calendar_unparsed_properties) =
                        &mut Calendar::parse_calendars(buf)?;
                    unparsed_properties.extend(calendar_unparsed_properties.clone().into_iter());
                    calendars.append(parsed_calendars);
                }
                CalendarUrl(url) => {
                    log::info!("reading calendar url: {}", url);
                    let ics_string = ureq::get(url.as_ref()).call()?.into_string()?;
                    let (parsed_calendars, calendar_unparsed_properties) =
                        &mut Calendar::parse_calendars(ics_string.as_bytes())?;
                    unparsed_properties.extend(calendar_unparsed_properties.clone().into_iter());
                    calendars.append(parsed_calendars);
                }
            }
        }

        let end_of_month_default = DateRule::monthly(Utc::now())
            .with_rolling_day(31)
            .unwrap()
            .next()
            .unwrap();
        // .ok_or(eyre!("could not get end of month")?;

        // get start and end date for entire collection
        let cal_start = calendars
            .iter()
            .map(|c| c.start())
            .reduce(|min_start, start| min_start.min(start))
            .unwrap_or_else(Utc::now);
        let cal_end = calendars
            .iter()
            .map(|c| c.end())
            .reduce(|max_end, end| max_end.max(end))
            // TODO consider a better approach to finding the correct number of days
            .unwrap_or(end_of_month_default);

        // expand recurring events
        log::debug!("expanding recurring events...");
        for calendar in calendars.iter_mut() {
            let pre_expansion_count = calendar.events().len();
            calendar.expand_recurrences(cal_start, cal_end)?;
            log::debug!(
                "calendar events pre_expansion_count: {} post_expansion_count: {}",
                pre_expansion_count,
                calendar.events().len()
            );
        }

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
    // let days_in_month = days_in_year_month(*year, month_from_u8(*month)?);
    // let last_day = NaiveDate::from_calendar_date(*year, month_from_u8(*month)?, days_in_month)?;
    let first_day =
        NaiveDate::from_ymd_opt(*year, *month as u32, 1).expect("could not get first day of month");
    let last_day = DateRule::monthly(first_day)
        .with_rolling_day(31)
        .expect("could not create rolling day rule")
        .next()
        .expect("could not get last day of month");
    let last_week = match last_day.weekday() == Sun {
        // iso weeks start on Monday
        true => last_day
            .succ_opt()
            .ok_or(eyre!("could not get successive day"))?,
        false => last_day,
    }
    .iso_week();

    Ok((first_week.week() as u8)..(last_week.week() as u8))
}

/// Return the first Sunday that should appear in a calendar view, even if that date is in the previous month
fn first_sunday_of_view(year: Year, month: Month) -> Result<InternalDate> {
    let first_day_of_month = NaiveDate::from_ymd_opt(year, month.number_from_month(), 1)
        .ok_or(eyre!("could not get date for year and month"))?;
    let days_from_sunday = first_day_of_month.weekday().num_days_from_sunday();
    let first_day_of_view = first_day_of_month - Days::new(days_from_sunday.into());
    Ok(first_day_of_view)
}

/// Return the first Sunday of the week, even if that week is in the previous month
fn first_sunday_of_week(year: &i32, week: &u32) -> Result<InternalDate, color_eyre::Report> {
    let first_sunday_of_month =
        NaiveDate::from_isoywd_opt(*year, *week, Sun).ok_or(eyre!("could not get iso week"))?;
    // let first_sunday_of_view = first_sunday_of_view(
    //     *year,
    //     Month::from_u32(first_sunday_of_month.month()).ok_or(eyre!("could not get month"))?,
    // )?;
    // let sunday =
    //     if (first_sunday_of_month.to_julian_day() - first_sunday_of_view.to_julian_day()) >= 7 {
    //         first_sunday_of_month
    //     } else {
    //         first_sunday_of_view
    //     };
    Ok(first_sunday_of_month)
}

/// Generates context objects for the days of a week
///
/// Implementing this as a trait so we can call it on a typedef rather than creating a new struct.
pub trait WeekContext {
    fn context(&self, year: &i32, week: &u8, tz: &Tz) -> Result<Vec<DayContext>>;
}

impl WeekContext for WeekDayMap {
    fn context(&self, year: &i32, week: &u8, tz: &Tz) -> Result<Vec<DayContext>> {
        let sunday = first_sunday_of_week(year, &(*week as u32))?;
        let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
            .iter()
            .map(|o| {
                DayContext::new(
                    sunday + Duration::days(*o as i64),
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
    let sunday = first_sunday_of_week(year, &(*week as u32))?;
    let week_dates: Vec<DayContext> = [0_u8, 1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8]
        .iter()
        .map(|o| DayContext::new(sunday + Duration::days(*o as i64), Vec::new()))
        .collect();
    Ok(week_dates)
}

// TODO convert to use functions from chrono crate
pub(crate) fn month_from_u8(value: u8) -> Result<Month> {
    match value {
        1 => Ok(Month::January),
        2 => Ok(Month::February),
        3 => Ok(Month::March),
        4 => Ok(Month::April),
        5 => Ok(Month::May),
        6 => Ok(Month::June),
        7 => Ok(Month::July),
        8 => Ok(Month::August),
        9 => Ok(Month::September),
        10 => Ok(Month::October),
        11 => Ok(Month::November),
        12 => Ok(Month::December),
        _ => bail!("can only convert numbers from 1-12 into months"),
    }
}
