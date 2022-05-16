use color_eyre::eyre::{self, Result, WrapErr};
use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use std::io::BufRead;

use crate::model::event::Event;

#[derive(Debug)]
pub struct Calendar {
    name: Option<String>,
    description: Option<String>,
    events: Vec<Event>,
}

impl Calendar {
    pub fn events(&self) -> &Vec<Event> {
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

    pub fn push(&mut self, event: Event) {
        self.events.push(event)
    }

    /// Parse calendar data from ICS
    ///
    /// The ICS data can be either a file or a url. Anything that implements BufRead such as a File or String::as_bytes().
    pub fn parse_calendars<B>(buf: B) -> Result<Vec<Calendar>>
    where
        B: BufRead,
    {
        let mut calendars = Vec::new();
        let reader = IcalParser::new(buf);
        for entry in reader {
            eprintln!("{:#?}", entry);
            if let Ok(calendar) = entry {
                let mut new_calendar = Calendar::new(&calendar)?;
                for event in calendar.events {
                    let new_event = Event::new(event)?;
                    eprintln!("{}", new_event);
                    new_calendar.push(new_event);
                }
                calendars.push(new_calendar);
            }
        }
        Ok(calendars)
    }
}
