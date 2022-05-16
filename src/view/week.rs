use color_eyre::eyre::{self, Result, WrapErr};
use std::collections::BTreeMap;

use crate::model::{calendar::Calendar, event::Event};

pub struct WeekCollection {
    weeks: BTreeMap<(u32, u8), Vec<Event>>,
}

impl WeekCollection {
    pub fn new(calendars: &Vec<Calendar>) -> Result<WeekCollection> {
        for calendar in calendars {
            for event in calendar.events() {
                println!(
                    "Event: {} ({} {}) {}",
                    event.summary(),
                    event.year(),
                    event.week(),
                    event.start(),
                );
            }
        }
        todo!()
    }
}
