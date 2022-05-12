#![allow(unused_imports)]

use color_eyre::eyre::{self, WrapErr};
use ical::IcalParser;
use std::io::BufRead;

mod event;
use crate::event::Event;

/// Parse calendar data from ICS
///
/// The ICS data can be either a file or a url. Anything that implements BufRead such as a File or String::as_bytes().
pub fn parse_calendar<B>(buf: B) -> eyre::Result<()>
where
    B: BufRead,
{
    let reader = IcalParser::new(buf);
    for entry in reader {
        if let Ok(calendar) = entry {
            for event in calendar.events {
                println!("{}", Event::new(event)?);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::{assert_eq, assert_ne};

    use super::*;
}
