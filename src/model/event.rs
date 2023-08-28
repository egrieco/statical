use chrono::{DateTime, Datelike, Duration, IsoWeek, NaiveDate, NaiveDateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use chrono_tz::Tz;
use chronoutil::DateRule;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use ical::parser::ical::component::IcalEvent;
use regex::RegexSet;
use rrule::RRuleSet;
use serde::Serialize;
use std::{collections::HashSet, fmt, rc::Rc};
use unescaper::unescape;

/// An enum to help us determine how to parse a given date based on the regex that matched
enum ParseType {
    ParseDateTime,
    ParseDate,
}

const MISSING_SUMMARY: &str = "None";

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
const RRULE_DTSTART_PARSING_FORMAT: &str = "%Y%m%dT%H%M%SZ";

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
            duration: HumanTime::from(self.duration).to_text_en(Accuracy::Precise, Tense::Present),
            url: self.url().to_owned(),
        }
    }

    pub fn summary(&self) -> &str {
        self.summary.as_deref().unwrap_or(MISSING_SUMMARY)
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn start_with_timezone(&self, tz: &Tz) -> DateTime<Tz> {
        self.start.with_timezone(tz)
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.start + self.duration
    }

    pub fn end_with_timezone(&self, tz: &Tz) -> DateTime<Tz> {
        (self.start + self.duration).with_timezone(tz)
    }

    pub fn days_with_timezone(&self, tz: &Tz) -> Vec<DateTime<Tz>> {
        // adjust by config.display_timezone
        let start = self.start_with_timezone(tz);
        let end = self.end_with_timezone(tz);

        // TODO don't forget to handle events that end on the day as well
        // TODO don't forget to handle multi-day events (events with RRules should already be handled)
        DateRule::daily(start).with_end(end).into_iter().collect()
    }

    pub fn url(&self) -> &str {
        self.url.as_deref().unwrap_or_default()
    }

    pub fn year(&self) -> Year {
        self.start.year()
    }

    pub fn year_with_timezone(&self, tz: &Tz) -> Year {
        self.start_with_timezone(tz).year()
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
        log::debug!("creating new Event...");

        let mut summary = None;
        let mut description = None;
        let mut start: Option<DateTime<Utc>> = None;
        let mut end: Option<DateTime<Utc>> = None;
        let mut rrule = None;
        let mut location = None;
        let mut url = None;

        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        for property in event.properties {
            log::debug!("parsing property: {}: {:?}", property.name, property.value);
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
                // TODO use the user configured default timezone
                "DTSTART" => start = property_to_time(&property, chrono_tz::UTC)?,
                // TODO use the user configured default timezone
                "DTEND" => end = property_to_time(&property, chrono_tz::UTC)?,
                "RRULE" => rrule = property.value,
                "LOCATION" => location = property.value,
                "URL" => url = property.value,
                _ => {
                    log::trace!("adding unparsed property: {}", property.name);
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
fn property_to_time(
    property: &ical::property::Property,
    default_timezone: chrono_tz::Tz,
) -> Result<Option<DateTime<Utc>>> {
    // this map holds the patterns to match, the corresponding format strings for parsing, and the type of parsing method
    // TODO use lazy_static! here
    let regex_fmt_map = vec![
        (r"^(\d+T\d+)Z$", "%Y%m%dT%H%M%SZ", ParseType::ParseDateTime),
        (r"^(\d+T\d+)$", "%Y%m%dT%H%M%S", ParseType::ParseDateTime),
        (r"^(\d+)$", "%Y%m%d", ParseType::ParseDate),
    ];
    let set = RegexSet::new(regex_fmt_map.iter().map(|r| r.0))?;

    let prop_value = &property
        .value
        .as_ref()
        .ok_or(eyre!("no value for this property"))?;
    log::debug!("prop_value: {}", prop_value);

    let matches: Vec<_> = set.matches(prop_value).into_iter().collect();
    log::debug!("matches: {:?}", matches);

    // TODO clean up timezone logic, looks like there are inefficiencies and bugs
    // let timezone: chrono_tz::Tz = UTC;
    let timezone: chrono_tz::Tz = if let Some(params) = &property.params {
        log::debug!("property has parameters, searching for TZID...");
        // if necessary, parse the primitive time and zone separately
        match params.iter().find(|(name, _zones)| name == "TZID") {
            Some((_, zones)) => {
                log::debug!("found TZID, zones: {:?}", zones);
                match zones
                    .first()
                    // TODO replace expect calls with proper error handling
                    .map(|tz_name| tz_name.parse::<Tz>().expect("could not parse timezone"))
                {
                    Some(tz) => {
                        log::debug!("returning timezone: {}", tz);
                        tz
                    }
                    None => {
                        log::debug!("returning default timezone");
                        default_timezone
                    }
                }
            }
            None => {
                log::debug!("returning default timezone");
                default_timezone
            }
        }
    } else {
        // set a default timezone
        log::debug!("returning default timezone");
        default_timezone
    };

    let first_match = matches.first().expect("no matches found");

    // parse the time without zone information
    let fmt = regex_fmt_map[*first_match].1;
    log::debug!("parsing '{}' with '{}'", prop_value, fmt);

    let primitive_time: DateTime<Utc> = match regex_fmt_map[*first_match].2 {
        ParseType::ParseDateTime => {
            match NaiveDateTime::parse_from_str(prop_value, fmt)
                .wrap_err("could not parse this time")?
                .and_local_timezone(timezone)
            {
                chrono::LocalResult::None => bail!("no sensible time for given value"),
                chrono::LocalResult::Single(time) => time.with_timezone(&Utc),
                // TODO handle cases where we actually want the second time
                chrono::LocalResult::Ambiguous(time, _second_time) => time.with_timezone(&Utc),
            }
        }
        ParseType::ParseDate => match NaiveDate::parse_from_str(prop_value, fmt)
            .wrap_err("could not parse this date")?
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
        {
            chrono::LocalResult::None => unreachable!(),
            chrono::LocalResult::Single(time) => time,
            chrono::LocalResult::Ambiguous(time, _second_time) => time,
        },
    };

    // adjust the timezone
    Ok(Some(primitive_time))
}
