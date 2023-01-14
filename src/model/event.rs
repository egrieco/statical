use color_eyre::eyre::{bail, ContextCompat, Result, WrapErr};
use ical::parser::ical::component::IcalEvent;
use regex::Regex;
use rrule::RRule;
use serde::Serialize;
use std::{collections::HashSet, fmt, rc::Rc};
use time::{
    macros::{format_description, offset},
    Duration, OffsetDateTime, PrimitiveDateTime,
};
use time_tz::{timezones::get_by_name, OffsetDateTimeExt, PrimitiveDateTimeExt, Tz};

const MISSING_SUMMARY: &str = "None";

pub type Year = i32;
pub type WeekNum = u8;

pub type UnparsedProperties = HashSet<String>;

/// A list of events
///
/// These are reference counted since they may appear in more than one list
pub type EventList = Vec<Rc<Event>>;

#[derive(Debug, Serialize)]
pub struct Event {
    summary: Option<String>,
    description: Option<String>,
    start: OffsetDateTime,
    duration: Duration,
    rrule: Option<String>,
    location: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventContext {
    summary: String,
    description: String,
    start: String,
    start_timestamp: i64,
    end: String,
    end_timestamp: i64,
    duration: String,
    url: String,
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
    pub fn context(&self, tz: &Tz) -> EventContext {
        EventContext {
            summary: self.summary().into(),
            description: self
                .description
                .as_deref()
                .unwrap_or("NO DESCRIPTION")
                .into(),
            start: self
                .start()
                .to_timezone(tz)
                .format(format_description!(
                    "[hour repr:12 padding:none]:[minute][period case:lower]"
                ))
                .unwrap_or_else(|_| "NO START TIME".to_string()),
            start_timestamp: self.start().to_timezone(tz).unix_timestamp(),
            end: self
                .end()
                .to_timezone(tz)
                .format(format_description!(
                    "[hour repr:12 padding:none]:[minute][period case:lower]"
                ))
                .unwrap_or_else(|_| "NO END TIME".to_string()),
            end_timestamp: self.end().to_timezone(tz).unix_timestamp(),
            duration: self.duration.to_string(),
            url: self.url().to_owned(),
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

    pub fn url(&self) -> &str {
        self.url.as_deref().unwrap_or_default()
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

    pub fn rrule(&self) -> Option<RRule> {
        println!("Attempting to parse: {:?}", self.rrule);
        if let Some(rrule_str) = &self.rrule {
            match format!(
                "DTSTART:{}\n{}",
                self.start()
                    // ensure that DTSTART is provided in UTC
                    .to_offset(offset!(+0))
                    .format(format_description!(
                        "[year][month][day]T[hour][minute][second]Z"
                    ))
                    .unwrap(),
                rrule_str
            )
            .parse()
            {
                Ok(rrule) => Some(rrule),
                Err(e) => {
                    println!("Could not parse rrule: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn new(event: IcalEvent) -> Result<(Event, UnparsedProperties)> {
        let mut summary = None;
        let mut description = None;
        let mut start: Option<OffsetDateTime> = None;
        let mut end: Option<OffsetDateTime> = None;
        let mut rrule = None;
        let mut location = None;
        let mut url = None;

        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        for property in event.properties {
            match property.name.as_str() {
                "SUMMARY" => summary = property.value,
                "DESCRIPTION" => description = property.value,
                "DTSTART" => start = property_to_time(&property)?,
                "DTEND" => end = property_to_time(&property)?,
                "RRULE" => rrule = property.value,
                "LOCATION" => location = property.value,
                "URL" => url = property.value,
                _ => {
                    unparsed_properties.insert(property.name);
                    // TODO collect unparsed params as well
                    // if let Some(params) = property.params {
                    //     println!("{:#?}", params);
                    // }
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

        // TODO parse the rrule here, store None if it does not parse
        Ok((
            Event {
                summary,
                description,
                start: start.unwrap(),
                duration: end.unwrap() - start.unwrap(),
                rrule,
                location,
                url,
            },
            unparsed_properties,
        ))
    }

    /// Creates a duplicate event with a different start datetime.
    ///
    /// This is useful when we are creating events from rrule expansions.
    pub fn duplicate_with_date(&self, date: OffsetDateTime) -> Event {
        // TODO might want to link this event back to its parent event in some way, maybe even have a separate event class
        Event {
            summary: self.summary.clone(),
            description: self.description.clone(),
            start: date,
            duration: self.duration,
            // we're un-setting the rrule to prevent recursion issues here
            rrule: None,
            location: self.location.clone(),
            url: self.url.clone(),
        }
    }
}

/// Given a time based ical property, parse it into a OffsetDateTime
fn property_to_time(property: &ical::property::Property) -> Result<Option<OffsetDateTime>> {
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
        if let Some(params) = &property.params {
            let (_, zones) = params.iter().find(|(name, _zones)| name == "TZID").unwrap();
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
