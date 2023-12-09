use chrono::{
    DateTime, Datelike, Duration, IsoWeek, Month, NaiveDate, NaiveDateTime, Utc, Weekday,
};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use ical::parser::ical::component::IcalEvent;
use indent::indent_all_by;
use num_traits::FromPrimitive;
use regex::{Regex, RegexSet};
use rrule::RRuleSet;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::{collections::HashSet, fmt, rc::Rc, sync::atomic::AtomicUsize};
use unescaper::unescape;

use crate::configuration::{calendar_source_config::CalendarSourceConfig, config::Config};
use crate::views::{
    day_view,
    event_view::{self},
    month_view, week_view,
};

/// An enum to help us determine how to parse a given date based on the regex that matched
enum ParseType {
    ParseDateTime,
    ParseDate,
}

const MISSING_SUMMARY: &str = "None";
const MISSING_DESCRIPTION: &str = "None";

// const START_DATETIME_FORMAT = format_description!(
//     "[weekday] [month repr:long] [day], [year] at [hour repr:12]:[minute][period case:lower]"
// );
const START_DATETIME_FORMAT: &str = "%a %B %d, %Y at %H:%M%P";

// const END_DATETIME_FORMAT = format_description!(
//     "[hour repr:12]:[minute][period case:lower]"
// );
const END_DATETIME_FORMAT: &str = "%H:%M%P";

// const RRULE_DTSTART_PARSING_FORMAT = format_description!(
//     "[year][month][day]T[hour][minute][second]Z"
// );
const RRULE_DTSTART_PARSING_FORMAT: &str = "%Y%m%dT%H%M%SZ";

const EVENT_FILE_FORMAT: &str = "%Y-%m-%d";

pub type Year = i32;
pub type WeekNum = u8;

pub type UnparsedProperties = HashSet<String>;

/// A list of events
///
/// These are reference counted since they may appear in more than one list
pub type EventList = Vec<Rc<Event>>;

/// This mostly exists to ensure that there is something to use as a unique ID for events when creating file names
static EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    calendar_config: Rc<CalendarSourceConfig>,
    summary: Option<String>,
    description: Option<String>,
    start: DateTime<Utc>,
    duration: Duration,
    rrule: Option<String>,
    location: Option<String>,
    url: Option<String>,
    event_number: usize,
}

#[derive(Debug, Serialize)]
pub struct EventContext {
    agenda_header: String,
    calendar_name: String,
    calendar_title: String,
    calendar_color: String,
    summary: String,
    description: String,
    start: String,
    start_timestamp: i64,
    end: String,
    end_timestamp: i64,
    duration: String,
    // NOTE: not sure if we want this in event context as well as day context
    iso_week: u8,
    url: String,
    file_path: String,
    day_view_path: String,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n  {} to {} for {}\n{}",
            self.summary.as_ref().unwrap_or(&"NO SUMMARY".to_string()),
            self.start.format(START_DATETIME_FORMAT),
            self.end().format(END_DATETIME_FORMAT),
            HumanTime::from(self.duration).to_text_en(Accuracy::Precise, Tense::Present),
            indent_all_by(
                2,
                self.description
                    .as_ref()
                    .unwrap_or(&"NO DESCRIPTION".to_string())
            )
        )
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.start.cmp(&other.start)
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl Event {
    /// Returns and EventContext suitable for providing values to Tera templates
    pub fn context(&self, config: &Config) -> EventContext {
        EventContext {
            // TODO: add an agenda_header_format to the config
            agenda_header: self.start.format("%a, %-d %B %Y").to_string(),
            calendar_name: self.calendar_config.name.clone(),
            // TODO: make sure that this is using the fallback from the calendar
            calendar_title: self
                .calendar_config
                .title
                .clone()
                .unwrap_or("No Title".to_owned()),
            calendar_color: if config.adjust_colors {
                self.calendar_config
                    .adjusted_color
                    .get()
                    .unwrap_or(&self.calendar_config.color.to_hex_string())
                    .clone()
            } else {
                self.calendar_config.color.to_hex_string()
            },
            summary: self.summary().into(),
            description: self
                .description
                .as_deref()
                .unwrap_or("NO DESCRIPTION")
                .into(),
            start: self
                .start()
                .with_timezone::<chrono_tz::Tz>(&config.display_timezone.into())
                .format(&config.event_start_format)
                .to_string(),
            start_timestamp: self
                .start()
                .with_timezone::<chrono_tz::Tz>(&config.display_timezone.into())
                .timestamp(),
            end: self
                .end()
                .with_timezone::<chrono_tz::Tz>(&config.display_timezone.into())
                .format(&config.event_end_format)
                .to_string(),
            end_timestamp: self
                .end()
                .with_timezone::<chrono_tz::Tz>(&config.display_timezone.into())
                .timestamp(),
            duration: HumanTime::from(self.duration).to_text_en(Accuracy::Precise, Tense::Present),
            iso_week: self.start.iso_week().week() as u8,
            url: self.url().to_owned(),
            file_path: self.file_path(),
            day_view_path: self.day_view_path(),
        }
    }

    pub(crate) fn summary_for_filename(&self) -> String {
        let replace_pattern =
            Regex::new("[^a-zA-Z0-9_-]+").expect("could not compile event summary replacer regex");
        replace_pattern
            .replace_all(
                self.summary
                    .as_ref()
                    .unwrap_or(&format!("event-{}", self.event_number)),
                "_",
            )
            .to_string()
    }

    pub fn file_name(&self) -> String {
        format!(
            "{}-{}.html",
            self.start().format(EVENT_FILE_FORMAT),
            self.summary_for_filename()
        )
    }

    pub fn file_path(&self) -> String {
        // TODO: need to add config.base_url_path
        PathBuf::from("/")
            .join(event_view::VIEW_PATH)
            .join(self.file_name())
            .to_string_lossy()
            .to_string()
    }

    pub fn day_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        PathBuf::from("/")
            .join(day_view::VIEW_PATH)
            .join(format!(
                "{}-{:02}-{:02}.html",
                self.year(),
                self.month_num(),
                self.day()
            ))
            .to_string_lossy()
            .to_string()
    }

    pub fn week_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        let week = self.iso_week();
        PathBuf::from("/")
            .join(week_view::VIEW_PATH)
            .join(format!("{}-{}.html", week.year(), week.week()))
            .to_string_lossy()
            .to_string()
    }

    pub fn month_view_path(&self) -> String {
        // TODO: need to add config.base_url_path
        PathBuf::from("/")
            .join(month_view::VIEW_PATH)
            .join(format!("{}-{}.html", self.year(), self.month_num()))
            .to_string_lossy()
            .to_string()
    }

    pub fn summary(&self) -> &str {
        self.summary.as_deref().unwrap_or(MISSING_SUMMARY)
    }

    pub fn description(&self) -> &str {
        self.description.as_deref().unwrap_or(MISSING_DESCRIPTION)
    }
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn start_with_timezone(&self, tz: &ChronoTz) -> DateTime<ChronoTz> {
        self.start.with_timezone(tz)
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.start + self.duration
    }

    pub fn end_with_timezone(&self, tz: &ChronoTz) -> DateTime<ChronoTz> {
        (self.start + self.duration).with_timezone(tz)
    }

    pub fn days_with_timezone(&self, tz: &ChronoTz) -> Vec<DateTime<ChronoTz>> {
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

    pub fn year_with_timezone(&self, tz: &ChronoTz) -> Year {
        self.start_with_timezone(tz).year()
    }

    pub fn month_num(&self) -> u32 {
        self.start.month()
    }

    pub fn month(&self) -> Option<Month> {
        Month::from_u32(self.month_num())
    }

    pub fn day(&self) -> u32 {
        self.start().day()
    }

    pub fn weekday(&self) -> Weekday {
        self.start().weekday()
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

    pub fn new(
        event: &IcalEvent,
        calendar_config: Rc<CalendarSourceConfig>,
    ) -> Result<(Event, UnparsedProperties)> {
        log::debug!("creating new Event...");

        // let calendar_config = Rc::new(calendar_config);
        let mut summary = None;
        let mut description = None;
        let mut start: Option<DateTime<Utc>> = None;
        let mut end: Option<DateTime<Utc>> = None;
        let mut rrule = None;
        let mut location = None;
        let mut url = None;

        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        for property in &event.properties {
            log::debug!("parsing property: {}: {:?}", property.name, property.value);
            match property.name.as_str() {
                "SUMMARY" => summary = property.value.clone(),
                // TODO: sanitize html, maybe expand markdown
                "DESCRIPTION" => {
                    description = property
                        .value
                        .clone()
                        // we have to strip out escaped commas so they don't trip up unescape
                        .map(|v| v.replace(r"\,", r","))
                        .map(|v| unescape(&v))
                        .transpose()?
                }
                // TODO use the user configured default timezone
                "DTSTART" => start = property_to_time(property, chrono_tz::UTC)?,
                // TODO use the user configured default timezone
                "DTEND" => end = property_to_time(property, chrono_tz::UTC)?,
                "RRULE" => rrule = property.value.clone(),
                "LOCATION" => location = property.value.clone(),
                "URL" => url = property.value.clone(),
                _ => {
                    log::trace!("adding unparsed property: {}", property.name);
                    unparsed_properties.insert(property.name.clone());
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
                calendar_config,
                summary,
                description,
                start: start.unwrap(),
                duration: end.unwrap() - start.unwrap(),
                rrule,
                location,
                url,
                event_number: EVENT_COUNT.fetch_add(1, Relaxed),
            },
            unparsed_properties,
        ))
    }

    /// Creates a duplicate event with a different start datetime.
    ///
    /// This is useful when we are creating events from rrule expansions.
    pub fn duplicate_with_date(&self, date: DateTime<ChronoTz>) -> Event {
        // TODO might want to link this event back to its parent event in some way, maybe even have a separate event class
        Event {
            calendar_config: self.calendar_config.clone(),
            summary: self.summary.clone(),
            description: self.description.clone(),
            start: date.with_timezone(&Utc),
            duration: self.duration,
            // we're un-setting the rrule to prevent recursion issues here
            rrule: None,
            location: self.location.clone(),
            url: self.url.clone(),
            event_number: EVENT_COUNT.fetch_add(1, Relaxed),
        }
    }
}

/// Given a time based ical property, parse it into a OffsetDateTime
fn property_to_time(
    property: &ical::property::Property,
    default_timezone: ChronoTz,
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
                    .map(|tz_name| {
                        tz_name
                            .parse::<ChronoTz>()
                            .expect("could not parse timezone")
                    }) {
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
