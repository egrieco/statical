#![allow(unused_imports)]

use color_eyre::eyre::{self, Result, WrapErr};
use ical::IcalParser;
use std::io::BufRead;

mod calendar;
mod event;
use crate::calendar::Calendar;
use crate::event::Event;

/// Parse calendar data from ICS
///
/// The ICS data can be either a file or a url. Anything that implements BufRead such as a File or String::as_bytes().
pub fn parse_calendar<B>(buf: B) -> Result<Vec<Calendar>>
where
    B: BufRead,
{
    let mut calendars = Vec::new();
    let mut events = Vec::new();
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
    Ok(events)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::{assert_eq, assert_ne};

    use super::*;
}
