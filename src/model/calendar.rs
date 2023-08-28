use chrono::{DateTime, Months, TimeZone, Utc};
use chrono_tz::Tz as ChronoTz;
use color_eyre::eyre::{Context, Result};
use ical::parser::ical::component::IcalCalendar;
use ical::IcalParser;
use rrule::Tz as RruleTz;
use std::collections::HashSet;
use std::io::BufRead;
use std::rc::Rc;

use super::event::{EventList, UnparsedProperties};
use crate::model::event::Event;

#[derive(Debug)]
pub struct Calendar {
    name: Option<String>,
    description: Option<String>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    events: EventList,
    recurring_events: EventList,
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
        let now = Utc::now();

        // build the new calendar
        Ok(Calendar {
            name,
            description,
            start: now,
            end: now + Months::new(1),
            events: Vec::new(),
            recurring_events: Vec::new(),
        })
    }

    pub fn push(&mut self, event: Rc<Event>) -> Result<()> {
        // collect calendar start and end dates, we need this for rrule expansion
        self.start = self.start.min(event.start());
        self.end = self.end.max(event.end());

        if event.rrule()?.is_some() {
            // add event to recurring_events
            // TODO might want to look at any recurrence termination dates and set calendar end to that
            self.recurring_events.push(event)
        } else {
            // add event to calendar event list
            self.events.push(event)
        }
        Ok(())
    }

    pub fn expand_recurrences(
        &mut self,
        cal_start: DateTime<ChronoTz>,
        cal_end: DateTime<ChronoTz>,
        tz: &ChronoTz,
    ) -> Result<()> {
        log::debug!("expanding recurrences for calendar: {:?}", self.name);
        log::debug!("calendar_runs from '{}' to '{}'", cal_start, cal_end);

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
    pub fn parse_calendars<B>(buf: B) -> Result<(Vec<Calendar>, UnparsedProperties)>
    where
        B: BufRead,
    {
        log::debug!("parsing calendars...");
        let mut calendars = Vec::new();
        let reader = IcalParser::new(buf);
        let mut unparsed_properties: UnparsedProperties = HashSet::new();

        for calendar in reader.flatten() {
            let mut new_calendar = Calendar::new(&calendar)?;

            log::debug!("parsing calendar events...");
            for event in calendar.events {
                let (new_event, event_unparsed_properties) = Event::new(event)?;
                unparsed_properties.extend(event_unparsed_properties.into_iter());
                let rc_event = Rc::new(new_event);
                new_calendar
                    .push(rc_event)
                    .wrap_err("could not add event to calendar")?;
            }
            calendars.push(new_calendar);
        }
        Ok((calendars, unparsed_properties))
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
