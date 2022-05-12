use color_eyre::eyre::{self, bail, ContextCompat, Result, WrapErr};
use ical::parser::ical::component::IcalEvent;
use time::{macros::format_description, Duration, OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones::get_by_name, PrimitiveDateTimeExt};

#[derive(Debug)]
pub struct Event {
    summary: Option<String>,
    description: Option<String>,
    start: OffsetDateTime,
    duration: Duration,
}

impl Event {
    pub fn new(event: IcalEvent) -> Result<Event> {
        let mut summary = None;
        let mut description = None;
        let mut start: Option<OffsetDateTime> = None;
        let mut end: Option<OffsetDateTime> = None;

        for property in event.properties {
            eprintln!("Parsing {}: {:?}", property.name, property.value);

            match property.name.as_str() {
                "SUMMARY" => summary = property.value,
                "DESCRIPTION" => description = property.value,
                "DTSTART" => start = property_to_time(&property)?,
                "DTEND" => end = property_to_time(&property)?,
                _ => {
                    eprintln!("Ignoring {}: {:?}", property.name, property.value);
                    if let Some(params) = property.params {
                        println!("{:#?}", params);
                    }
                }
            }
        }

        // bail if we don't have enough info
        if summary.is_none() {
            bail!("event has no summary")
        }
        if start.is_none() {
            bail!("event has no start time")
        }
        if end.is_none() {
            bail!("event has no end time")
        }

        Ok(Event {
            summary,
            description,
            start: start.unwrap(),
            duration: end.unwrap() - start.unwrap(),
        })
    }
}

/// Given a time based ical property, parse it into a OffsetDateTime
fn property_to_time(property: &ical::property::Property) -> Result<Option<OffsetDateTime>> {
    eprintln!("attempting to parse: {}", property.name);
    let timezone = if let Some(params) = &property.params {
        let (_, zones) = params
            .into_iter()
            .find(|(name, _zones)| name == "TZID")
            .unwrap();
        eprintln!("zones: {:#?}", zones);
        zones.first().and_then(|tz_name| get_by_name(tz_name))
    } else {
        // need to set a default timezone
        get_by_name("America/Phoenix")
    };
    let primitive_time = PrimitiveDateTime::parse(
        property
            .value
            .as_ref()
            .context("no value for this property")?,
        format_description!("[year][month][day]T[hour][minute][second]"),
    )
    .context("could not parse this time")?;
    Ok(Some(
        primitive_time
            .assume_timezone(timezone.expect("no timezone determined for start time"))
            .unwrap(),
    ))
}
