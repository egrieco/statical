use chrono::{DateTime, Datelike, Duration, IsoWeek, NaiveDateTime, Utc};
use chrono_tz::Tz;
use color_eyre::eyre::{bail, ContextCompat, Result, WrapErr};
use const_format::concatcp;
use ical::parser::ical::component::IcalEvent;
use regex::Regex;
use rrule::RRuleSet;
use serde::Serialize;
use std::{collections::HashSet, fmt, rc::Rc};
use unescaper::unescape;

const MISSING_SUMMARY: &str = "None";
// const PROPERTY_DATETIME_FORMAT: &str = "[year][month][day]T[hour][minute][second]";
const PROPERTY_DATETIME_FORMAT: &str = "%y%m%dT%H%M%S";
// const START_DATETIME_FORMAT = format_description!(
//     "[weekday] [month repr:long] [day], [year] at [hour repr:12]:[minute][period case:lower]"
// );
const START_DATETIME_FORMAT: &str = "%a %B %d, %Y at %H:%M%P";
// const END_DATETIME_FORMAT = format_description!(
//     "[hour repr:12]:[minute][period case:lower]"
// );
const END_DATETIME_FORMAT: &str = "%H:%M%P";
// const CONTEXT_START_DATETIME_FORMAT: &str = format_description!(
//     "[hour repr:12 padding:none]:[minute][period case:lower]"
// );
const CONTEXT_START_DATETIME_FORMAT: &str = END_DATETIME_FORMAT;
// const CONTEXT_END_DATETIME_FORMAT: &str = format_description!(
//     "[hour repr:12 padding:none]:[minute][period case:lower]"
// );
const CONTEXT_END_DATETIME_FORMAT: &str = END_DATETIME_FORMAT;

// const RRULE_DTSTART_PARSING_FORMAT = format_description!(
//     "[year][month][day]T[hour][minute][second]Z"
// );
const RRULE_DTSTART_PARSING_FORMAT: &str = concatcp!(PROPERTY_DATETIME_FORMAT, "Z");

pub type Year = i32;
pub type WeekNum = u8;

pub type UnparsedProperties = HashSet<String>;

/// A list of events
///
/// These are reference counted since they may appear in more than one list
pub type EventList = Vec<Rc<Event>>;

#[derive(Debug)]
pub struct Event {
    summary: Option<String>,
    description: Option<String>,
    start: DateTime<Utc>,
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
            self.start.format(START_DATETIME_FORMAT),
            self.end().format(END_DATETIME_FORMAT),
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
                .with_timezone(tz)
                .format(CONTEXT_START_DATETIME_FORMAT)
                .to_string(),
            start_timestamp: self.start().with_timezone(tz).timestamp(),
            end: self
                .end()
                .with_timezone(tz)
                .format(CONTEXT_END_DATETIME_FORMAT)
                .to_string(),
            end_timestamp: self.end().with_timezone(tz).timestamp(),
            duration: self.duration.to_string(),
            url: self.url().to_owned(),
        }
    }

    pub fn summary(&self) -> &str {
        self.summary.as_deref().unwrap_or(MISSING_SUMMARY)
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn end(&self) -> DateTime<Utc> {
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
    pub fn iso_week(&self) -> IsoWeek {
        self.start.iso_week()
    }

    /// Returns the week number of the event
    ///
    /// This returns the ISO week (as opposed to the `sunday_based_week()` or `monday_based_week()` functions)
    /// since there is a `from_iso_week_date()` function we can use when rendering the week view.
    pub fn week(&self) -> u8 {
        self.start.iso_week().week() as u8
    }

    pub fn rrule(&self) -> Result<Option<RRuleSet>> {
        println!("Attempting to parse rrule: {:?}", self.rrule);

        // ensure that DTSTART is provided in UTC
        let start_time = self.start().format(RRULE_DTSTART_PARSING_FORMAT);

        if let Some(rrule_str) = &self.rrule {
            let rrule = format!("DTSTART:{}\n{}", start_time, rrule_str).parse()?;
            Ok(Some(rrule))
        } else {
            Ok(None)
        }
    }

    pub fn new(event: IcalEvent) -> Result<(Event, UnparsedProperties)> {
        let mut summary = None;
        let mut description = None;
        let mut start: Option<DateTime<Utc>> = None;
        let mut end: Option<DateTime<Utc>> = None;
        let mut rrule = None;
        let mut location = None;
        let mut url = None;

        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        for property in event.properties {
            match property.name.as_str() {
                "SUMMARY" => summary = property.value,
                "DESCRIPTION" => {
                    description = property
                        .value
                        // we have to strip out escaped commas so they don't trip up unescape
                        .map(|v| v.replace(r"\,", r","))
                        .map(|v| unescape(&v))
                        .transpose()?
                }
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
    pub fn duplicate_with_date(&self, date: DateTime<Utc>) -> Event {
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
fn property_to_time(property: &ical::property::Property) -> Result<Option<DateTime<Utc>>> {
    let date_format = Regex::new("^(\\d+T\\d+)(Z)?$")?;
    let date_captures = date_format
        .captures(
            property
                .value
                .as_ref()
                .context("no value for this property")?,
        )
        .expect("could not get captures");

    let timezone: chrono_tz::Tz = if date_captures.get(2).map(|c| c.as_str()) == Some("Z") {
        "UTC".parse().expect("could not parse timezone")
    } else {
        // if necessary, parse the primitive time and zone separately
        if let Some(params) = &property.params {
            let (_, zones) = params.iter().find(|(name, _zones)| name == "TZID").unwrap();
            zones
                .first()
                .map(|tz_name| tz_name.parse::<Tz>().expect("could not parse timezone"))
                .expect("could not get timezone from property")
        } else {
            // need to set a default timezone
            "America/Phoenix".parse().expect("could not parse timezone")
        }
    };

    // parse the time without zone information
    let primitive_time: DateTime<Utc> = match NaiveDateTime::parse_from_str(
        date_captures
            .get(1)
            .map(|c| c.as_str())
            .expect("could not get capture"),
        PROPERTY_DATETIME_FORMAT,
    )
    .wrap_err("could not parse this time")?
    .and_local_timezone(timezone)
    {
        chrono::LocalResult::None => bail!("no sensible time for given value"),
        chrono::LocalResult::Single(time) => time.with_timezone(&Utc),
        // TODO handle cases where we actually want the second time
        chrono::LocalResult::Ambiguous(time, _second_time) => time.with_timezone(&Utc),
    };

    // adjust the timezone
    Ok(Some(primitive_time))
}
