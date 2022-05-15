use color_eyre::eyre::{self, Result, WrapErr};
use ical::parser::ical::component::IcalCalendar;

use crate::event::Event;

#[derive(Debug)]
pub struct Calendar {
    name: Option<String>,
    description: Option<String>,
    events: Vec<Event>,
}

impl Calendar {
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
}
