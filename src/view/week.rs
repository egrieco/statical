use color_eyre::eyre::{self, Result, WrapErr};
use std::collections::BTreeMap;

use crate::model::event::{Week, Year};
use crate::model::{calendar::Calendar, event::Event};

type WeekMap<'a> = BTreeMap<(Year, Week), Vec<&'a Event>>;

pub struct WeekCollection<'a> {
    weeks: WeekMap<'a>,
}

impl WeekCollection<'_> {
    pub fn new(calendars: &Vec<Calendar>) -> Result<WeekCollection> {
        let mut weeks: WeekMap = BTreeMap::new();

        for calendar in calendars {
            for event in calendar.events() {
                println!(
                    "Event: {} ({} {}) {}",
                    event.summary(),
                    event.year(),
                    event.week(),
                    event.start(),
                );
                weeks
                    .entry((event.year(), event.week()))
                    .or_insert(Vec::new())
                    .push(event);
            }
        }
        Ok(WeekCollection { weeks })
    }
}
