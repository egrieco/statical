use std::fmt;

use color_eyre::eyre::{self, bail, ContextCompat, Result, WrapErr};
use ical::parser::ical::component::IcalEvent;
use regex::Regex;
use serde::Serialize;
use time::{macros::format_description, Duration, OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones::get_by_name, PrimitiveDateTimeExt};

const MISSING_SUMMARY: &str = "None";

pub type Year = i32;
pub type WeekNum = u8;

#[derive(Debug, Serialize)]
pub struct Event {
    summary: Option<String>,
    description: Option<String>,
    start: OffsetDateTime,
    duration: Duration,
}

#[derive(Debug, Serialize)]
pub struct EventContext {
    summary: String,
    description: String,
    start: String,
    end: String,
    duration: String,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} to {} for {})\n{}",
            self.summary.as_ref().unwrap_or(&"NO SUMMARY".to_string()),
            self.start
                .format(format_description!(
                    "[weekday] [month repr:long] [day], [year] at [hour repr:12]:[minute][period case:lower]"
                ))
                .expect("could not format start time"),
            self.end().format(format_description!("[hour repr:12]:[minute][period case:lower]")).expect("could not format end time"),
            self.duration,
            self.description
                .as_ref()
                .unwrap_or(&"NO DESCRIPTION".to_string())
        )
    }
}

impl Event {
    /// Returns and EventContext suitable for providing values to Tera templates
    pub fn context(&self) -> EventContext {
        EventContext {
            summary: self.summary().into(),
            description: self
                .description
                .as_deref()
                .unwrap_or("NO DESCRIPTION")
                .into(),
            start: self
                .start()
                .format(format_description!(
                    "[hour repr:12 padding:none]:[minute][period case:lower]"
                ))
                .unwrap_or("NO START TIME".to_string()),
            end: self
                .end()
                .format(format_description!(
                    "[hour repr:12 padding:none]:[minute][period case:lower]"
                ))
                .unwrap_or("NO END TIME".to_string()),
            duration: self.duration.to_string(),
        }
    }

    pub fn summary(&self) -> &str {
        self.summary.as_deref().unwrap_or(MISSING_SUMMARY)
    }

    pub fn start(&self) -> OffsetDateTime {
        self.start
    }

    pub fn end(&self) -> OffsetDateTime {
        self.start + self.duration
    }

    pub fn year(&self) -> Year {
        self.start.year()
    }

    /// Returns the week number of the event
    ///
    /// This returns the ISO week (as opposed to the `sunday_based_week()` or `monday_based_week()` functions)
    /// since there is a `from_iso_week_date()` function we can use when rendering the week view.
    pub fn week(&self) -> WeekNum {
        self.start.iso_week()
    }

    pub fn new(event: IcalEvent) -> Result<Event> {
        let mut summary = None;
        let mut description = None;
        let mut start: Option<OffsetDateTime> = None;
        let mut end: Option<OffsetDateTime> = None;

        for property in event.properties {
            eprintln!("  Parsing {}: {:?}", property.name, property.value);

            match property.name.as_str() {
                "SUMMARY" => summary = property.value,
                "DESCRIPTION" => description = property.value,
                "DTSTART" => start = property_to_time(&property)?,
                "DTEND" => end = property_to_time(&property)?,
                _ => {
                    eprintln!("  Ignoring {}: {:?}", property.name, property.value);
                    if let Some(params) = property.params {
                        println!("{:#?}", params);
                    }
                }
            }
        }

        // bail if we don't have enough info
        if summary.is_none() {
            bail!("event has no summary")
        }
        if start.is_none() {
            bail!("event has no start time")
        }
        if end.is_none() {
            bail!("event has no end time")
        }

        Ok(Event {
            summary,
            description,
            start: start.unwrap(),
            duration: end.unwrap() - start.unwrap(),
        })
    }
}

/// Given a time based ical property, parse it into a OffsetDateTime
fn property_to_time(property: &ical::property::Property) -> Result<Option<OffsetDateTime>> {
    eprintln!("  attempting to parse: {}", property.name);

    let date_format = Regex::new("^(\\d+T\\d+)(Z)?$")?;
    let date_captures = date_format
        .captures(
            property
                .value
                .as_ref()
                .context("no value for this property")?,
        )
        .expect("could not get captures");

    let timezone = if date_captures.get(2).map(|c| c.as_str()) == Some("Z") {
        get_by_name("UTC")
    } else {
        // if necessary, parse the primitive time and zone separately
        eprintln!(
            "  attempting to parse with separate time zone: {}",
            property.name
        );
        if let Some(params) = &property.params {
            let (_, zones) = params.iter().find(|(name, _zones)| name == "TZID").unwrap();
            eprintln!("zones: {:#?}", zones);
            zones.first().and_then(|tz_name| get_by_name(tz_name))
        } else {
            // need to set a default timezone
            get_by_name("America/Phoenix")
        }
    };

    // parse the time without zone information
    let primitive_time = PrimitiveDateTime::parse(
        date_captures
            .get(1)
            .map(|c| c.as_str())
            .expect("could not get capture"),
        format_description!("[year][month][day]T[hour][minute][second]"),
    )
    .context("could not parse this time")?;

    // adjust the timezone
    Ok(Some(
        primitive_time
            .assume_timezone(timezone.expect("no timezone determined for start time"))
            .unwrap(),
    ))
}
