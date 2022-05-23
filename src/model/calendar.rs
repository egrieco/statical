use chrono::TimeZone;
use chrono_tz::UTC;
use color_eyre::eyre::{self, Result, WrapErr};
use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use rrule::DateFilter;
use std::collections::HashSet;
use std::io::BufRead;
use std::rc::Rc;
use time::ext::NumericalDuration;
use time::util::days_in_year_month;
use time::OffsetDateTime;

use crate::model::event::Event;

use super::event::UnparsedProperties;

#[derive(Debug)]
pub struct Calendar {
    name: Option<String>,
    description: Option<String>,
    start: OffsetDateTime,
    end: OffsetDateTime,
    events: Vec<Rc<Event>>,
    recurring_events: Vec<Rc<Event>>,
}

impl Calendar {
    pub fn new(calendar: &IcalCalendar) -> Result<Calendar> {
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
        let now = OffsetDateTime::now_utc();
        let year = now.year();
        let month = now.month();

        // build the new calendar
        Ok(Calendar {
            name,
            description,
            start: now,
            end: now.saturating_add((days_in_year_month(year, month) as i64).days()),
            events: Vec::new(),
            recurring_events: Vec::new(),
        })
    }

    pub fn push(&mut self, event: Rc<Event>) {
        // collect calendar start and end dates, we need this for rrule expansion
        self.start = self.start.min(event.start());
        self.end = self.end.max(event.end());

        if event.rrule().is_some() {
            // add event to recurring_events
            // TODO might want to look at any recurrence termination dates and set calendar end to that
            self.recurring_events.push(event)
        } else {
            // add event to calendar event list
            self.events.push(event)
        }
    }

    pub fn expand_recurrences(&mut self, cal_start: OffsetDateTime, cal_end: OffsetDateTime) {
        // we need to convert from the time-rs library to chrono for RRule's sake
        let repeat_start = UTC.timestamp(cal_start.unix_timestamp(), 0);
        let repeat_end = UTC.timestamp(cal_end.unix_timestamp(), 0);

        let mut new_events: Vec<Rc<Event>> = Vec::new();

        for event in self.recurring_events() {
            // TODO might want to make this a map based on UID
            println!("Event with rrule found: {:#?}", event);
            for recurrence_datetimes in event
                .rrule()
                .unwrap()
                // setting inclusive to true since we have moved recurring events into a separate vec
                .all_between(repeat_start, repeat_end, true)
            {
                // add event to groups
                println!("{:#?}", recurrence_datetimes);
                for recurrence_time in recurrence_datetimes {
                    // we have to convert the DateTime<Tz> back into an OffsetDateTime
                    let new_start =
                        OffsetDateTime::from_unix_timestamp(recurrence_time.timestamp())
                            .expect("could not build timestamp from recurrence time");
                    // TODO might want to push directly into the events vec and skip some of the checks in Calendar.push()
                    new_events.push(Rc::new(event.duplicate_with_date(new_start)));
                }
            }
        }

        // add new events to events in calendar
        // this extra step was necessary due to mutability rules in Rust and iterators
        self.events.extend(new_events);
    }

    /// Parse calendar data from ICS
    ///
    /// The ICS data can be either a file or a url. Anything that implements BufRead such as a File or String::as_bytes().
    pub fn parse_calendars<B>(buf: B) -> Result<(Vec<Calendar>, UnparsedProperties)>
    where
        B: BufRead,
    {
        let mut calendars = Vec::new();
        let reader = IcalParser::new(buf);
        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        for entry in reader {
            if let Ok(calendar) = entry {
                let mut new_calendar = Calendar::new(&calendar)?;
                for event in calendar.events {
                    let (new_event, event_unparsed_properties) = Event::new(event)?;
                    unparsed_properties.extend(event_unparsed_properties.into_iter());
                    let rc_event = Rc::new(new_event);
                    new_calendar.push(rc_event);
                }
                calendars.push(new_calendar);
            }
        }
        Ok((calendars, unparsed_properties))
    }

    #[must_use]
    pub fn start(&self) -> OffsetDateTime {
        self.start
    }

    #[must_use]
    pub fn end(&self) -> OffsetDateTime {
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
