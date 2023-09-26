use chrono::{DateTime, Months, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use color_eyre::eyre::{Context, Result};
use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use indent::indent_all_by;
use log::debug;
use rrule::Tz as RruleTz;
use std::io::BufRead;
use std::rc::Rc;
use std::{collections::HashSet, fmt};

use super::event::{EventList, UnparsedProperties};
use crate::configuration::calendar_source_config::CalendarSourceConfig;
use crate::model::event::Event;

const START_DATETIME_FORMAT: &str = "%a %B %d, %Y";
const END_DATETIME_FORMAT: &str = "%a %B %d, %Y";

#[derive(Debug, PartialEq)]
pub struct Calendar {
    /// The internal name of the calendar
    name: Option<String>,
    /// The user visible name of the calendar
    title: String,

    source_config: Rc<CalendarSourceConfig>,

    description: Option<String>,
    pub(crate) start: DateTime<Utc>,
    pub(crate) end: DateTime<Utc>,
    events: EventList,
    recurring_events: EventList,
    unparsed_properties: UnparsedProperties,
}

impl Eq for Calendar {}

impl fmt::Display for Calendar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n    {} to {}\n    {} events, {} recurring events\n{}",
            self.name.as_ref().unwrap_or(&"NO NAME".to_string()),
            self.start.format(START_DATETIME_FORMAT),
            self.end().format(END_DATETIME_FORMAT),
            self.events.len(),
            self.recurring_events.len(),
            indent_all_by(
                4,
                self.description
                    .as_ref()
                    .unwrap_or(&"NO DESCRIPTION".to_string())
            )
        )
    }
}

impl Calendar {
    pub fn new(
        calendar: &IcalCalendar,
        source_config: Rc<CalendarSourceConfig>,
    ) -> Result<Calendar> {
        // eprintln!("Parsing calendar: {:#?}", calendar);
        let mut name = None;
        let mut description = None;

        for property in &calendar.properties {
            match property.name.as_str() {
                "X-WR-CALNAME" => name = property.value.clone(),
                "X-WR-CALDESC" => description = property.value.clone(),
                _ => {
                    // TODO collect the unparsed properties
                    // eprintln!("  Ignoring {}: {:?}", property.name, property.value);
                }
            }
        }
        // calculate calendar start and end dates
        // default to today and one month from today
        let now = Utc::now();

        let title = source_config
            .title
            .clone()
            .or(name.clone())
            .unwrap_or("No Calendar Name Found".to_owned());

        let mut unparsed_properties: UnparsedProperties = HashSet::new();
        let mut events: EventList = Vec::new();
        let mut recurring_events: EventList = Vec::new();

        // setup default start and end of calendar
        let mut start = now;
        let mut end = now + Months::new(1);

        log::debug!("parsing calendar events...");
        for event in &calendar.events {
            let (new_event, event_unparsed_properties) = Event::new(event, source_config.clone())?;
            unparsed_properties.extend(event_unparsed_properties.into_iter());

            // collect calendar start and end dates, we need this for rrule expansion
            start = start.min(new_event.start());
            end = end.max(new_event.end());

            // sort events into recurring and non-recurring
            match &new_event.rrule().wrap_err("could not parse rrule")? {
                Some(rrules) => {
                    // extend the calendar end to the until value of the RRule
                    if let Some(end_date) = rrules
                        .get_rrule()
                        .iter()
                        .filter_map(|r| r.get_until())
                        .reduce(|accum, date| accum.max(date))
                    {
                        end = end.max(end_date.with_timezone(&Utc));
                    }

                    // add event to recurring_events
                    // TODO might want to look at any recurrence termination dates and set calendar end to that
                    recurring_events.push(Rc::new(new_event))
                }
                None => {
                    // add event to calendar event list
                    events.push(Rc::new(new_event))
                }
            }
        }

        debug!("calendar {:?} runs from {} to {}", name, start, end);

        // build the new calendar
        Ok(Calendar {
            name,
            title,
            source_config,
            description,
            start,
            end,
            events,
            recurring_events,
            unparsed_properties,
        })
    }

    pub fn expand_recurrences(
        &mut self,
        cal_start: DateTime<ChronoTz>,
        cal_end: DateTime<ChronoTz>,
        tz: &ChronoTz,
    ) -> Result<()> {
        log::debug!("expanding recurrences for calendar: {:?}", self.name);
        log::debug!("calendar runs from '{}' to '{}'", cal_start, cal_end);

        // we need to convert from the time-rs library to chrono for RRule's sake
        let repeat_start: DateTime<RruleTz> =
            rrule::Tz::UTC.from_utc_datetime(&cal_start.naive_utc());
        // .ok_or(bail!("could not get local start time"))
        // .into();
        let repeat_end: DateTime<RruleTz> = rrule::Tz::UTC.from_utc_datetime(&cal_end.naive_utc());
        // .single()
        // .ok_or(bail!("could not get local end time"));

        let mut new_events: EventList = Vec::new();

        for event in self.recurring_events() {
            // TODO might want to make this a map based on UID
            println!("Event with rrule found: {:#?}", event);
            if let Ok(Some(rrule)) = event.rrule() {
                // add event to groups
                for recurrence_time in &rrule.after(repeat_start).before(repeat_end) {
                    log::debug!(
                        "adding duplicate event with recurrence_time: {}",
                        recurrence_time
                    );
                    // TODO might want to push directly into the events vec and skip some of the checks in Calendar.push()
                    new_events.push(Rc::new(
                        // TODO ensure that we want this to be UTC here
                        event.duplicate_with_date(recurrence_time.with_timezone(tz)),
                    ));
                }
            };
        }

        // add new events to events in calendar
        // this extra step was necessary due to mutability rules in Rust and iterators
        log::debug!("adding {} new_events to calendar events", new_events.len());
        self.events.extend(new_events);

        Ok(())
    }

    /// Parse calendar data from ICS
    ///
    /// The ICS data can be either a file or a url. Anything that implements BufRead such as a File or String::as_bytes().
    pub fn parse_calendars<B>(
        buf: B,
        source_config: Rc<CalendarSourceConfig>,
    ) -> Result<Vec<Calendar>>
    where
        B: BufRead,
    {
        log::debug!("parsing calendars...");
        let mut calendars = Vec::new();
        let reader = IcalParser::new(buf);

        for calendar in reader.flatten() {
            calendars.push(Calendar::new(&calendar, source_config.clone())?);
        }
        Ok(calendars)
    }

    #[must_use]
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    #[must_use]
    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }

    #[must_use]
    pub fn events(&self) -> &[Rc<Event>] {
        self.events.as_ref()
    }

    #[must_use]
    pub fn recurring_events(&self) -> &[Rc<Event>] {
        self.recurring_events.as_ref()
    }
}
