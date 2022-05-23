use color_eyre::eyre::{self, Result, WrapErr};
use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
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
    pub fn events(&self) -> &Vec<Rc<Event>> {
        &self.events
    }

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
            self.recurring_events.push(event)
        } else {
            // add event to calendar event list
            self.events.push(event)
        }
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
}
