use color_eyre::eyre::{self, Result, WrapErr};
use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use std::collections::HashSet;
use std::io::BufRead;
use std::rc::Rc;

use crate::model::event::Event;

use super::event::UnparsedProperties;

#[derive(Debug)]
pub struct Calendar {
    name: Option<String>,
    description: Option<String>,
    events: Vec<Rc<Event>>,
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
            eprintln!("{:#?}", property);
            match property.name.as_str() {
                "X-WR-CALNAME" => name = property.value.clone(),
                "X-WR-CALDESC" => description = property.value.clone(),
                _ => {
                    eprintln!("  Ignoring {}: {:?}", property.name, property.value);
                }
            }
        }
        Ok(Calendar {
            name,
            description,
            events: Vec::new(),
        })
    }

    pub fn push(&mut self, event: Rc<Event>) {
        self.events.push(event)
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
            eprintln!("{:#?}", entry);
            if let Ok(calendar) = entry {
                let mut new_calendar = Calendar::new(&calendar)?;
                for event in calendar.events {
                    let (new_event, event_unparsed_properties) = Event::new(event)?;
                    unparsed_properties.extend(event_unparsed_properties.into_iter());
                    let rc_event = Rc::new(new_event);
                    eprintln!("{}", rc_event);
                    new_calendar.push(rc_event);
                }
                calendars.push(new_calendar);
            }
        }
        Ok((calendars, unparsed_properties))
    }
}
