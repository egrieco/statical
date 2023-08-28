use chrono::{DateTime, Utc};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{self, Context as EyreContext, Result};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::{fs::File, io::BufReader};
use tera::Tera;

use super::calendar_source::CalendarSource;
use super::event::{EventList, UnparsedProperties};
use crate::config::ParsedConfig;
use crate::model::calendar::Calendar;
use crate::model::calendar_source::CalendarSource::*;
use crate::options::Opt;
use crate::util::{self, create_subdir};
use crate::views::agenda_view::AgendaView;
use crate::views::day_view::DayView;
use crate::views::month_view::MonthView;
use crate::views::week_view::WeekView;

/// Type alias representing a specific day in time
pub(crate) type LocalDay = DateTime<ChronoTz>;

pub(crate) type EventsByLocalDay = BTreeMap<LocalDay, EventList>;

#[derive(Debug)]
pub struct CalendarCollection {
    calendars: Vec<Calendar>,
    /// Events grouped by day in the display timezone
    pub(crate) events_by_day: EventsByLocalDay,

    tera: Tera,
    pub(crate) config: ParsedConfig,
    unparsed_properties: UnparsedProperties,
    pub(crate) cal_start: DateTime<ChronoTz>,
    pub(crate) cal_end: DateTime<ChronoTz>,
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

        let end_of_month_default =
            DateRule::monthly(Utc::now().with_timezone(&config.display_timezone))
                .with_rolling_day(31)
                .unwrap()
                .next()
                .unwrap();
        // .ok_or(eyre!("could not get end of month")?;

        // get start and end date for entire collection
        let cal_start = calendars
            .iter()
            .map(|c| c.start().with_timezone(&config.display_timezone))
            .reduce(|min_start, start| min_start.min(start))
            .unwrap_or_else(|| Utc::now().with_timezone(&config.display_timezone));
        let cal_end = calendars
            .iter()
            .map(|c| c.end().with_timezone(&config.display_timezone))
            .reduce(|max_end, end| max_end.max(end))
            // TODO consider a better approach to finding the correct number of days
            .unwrap_or(end_of_month_default);

        // expand recurring events
        log::debug!("expanding recurring events...");
        for calendar in calendars.iter_mut() {
            let pre_expansion_count = calendar.events().len();
            calendar.expand_recurrences(cal_start, cal_end, &config.display_timezone)?;
            log::debug!(
                "calendar events pre_expansion_count: {} post_expansion_count: {}",
                pre_expansion_count,
                calendar.events().len()
            );
        }

        // TODO might want to hand back a better event collection e.g. might want to de-duplicate them
        let mut events_by_day = EventsByLocalDay::new();

        for event in calendars.iter().flat_map(|c| c.events()) {
            // find out if event is longer than 1 day
            // find out if the event crosses a day boundary in this timezone
            // find out if this event ends on this day
            for day in event.days_with_timezone(&config.display_timezone) {
                events_by_day.entry(day).or_default().push(event.clone());
            }
        }

        Ok(CalendarCollection {
            calendars,
            events_by_day,
            tera: Tera::new("templates/**/*.html")?,
            config,
            unparsed_properties,
            cal_start,
            cal_end,
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

        // add events to views
        if self.config.render_month {
            MonthView::new(create_subdir(&self.config.output_dir, "month")?, self)
                .create_html_pages(&self.config, &self.tera)?;
        };

        if self.config.render_week {
            WeekView::new(
                create_subdir(&self.config.output_dir, "week")?,
                &self.calendars,
            )
            .create_html_pages(&self.config, &self.tera)?;
        };

        if self.config.render_day {
            DayView::new(
                create_subdir(&self.config.output_dir, "day")?,
                &self.calendars,
            )
            .create_html_pages(&self.config, &self.tera)?;
        };

        if self.config.render_agenda {
            AgendaView::new(
                create_subdir(&self.config.output_dir, "agenda")?,
                &self.calendars,
            )
            .create_html_pages(&self.config, &self.tera)?;
        };

        Ok(())
    }
}
