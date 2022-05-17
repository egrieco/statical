use color_eyre::eyre::{self, Result, WrapErr};
use std::{fs::File, io::BufReader};

use crate::model::calendar::Calendar;
use crate::options::Opt;
use crate::view::week::WeekCollection;

pub struct CalendarCollection {
    calendars: Vec<Calendar>,
}

impl CalendarCollection {
    pub fn new(args: Opt) -> eyre::Result<CalendarCollection> {
        let mut calendars = Vec::new();

        if let Some(files) = args.file {
            for file in files {
                println!("  Provided path is: {:?}", file);
                if file.exists() {
                    println!("    File exists");
                    let buf = BufReader::new(File::open(file)?);
                    calendars.append(&mut Calendar::parse_calendars(buf)?);
                }
            }
        };

        if let Some(urls) = args.url {
            for url in urls {
                println!("  Provided url is: {:?}", url);
                let ics_string = ureq::get(&url).call()?.into_string()?;
                println!("    URL exists");
                calendars.append(&mut Calendar::parse_calendars(ics_string.as_bytes())?);
            }
        }

        Ok(CalendarCollection { calendars })
    }

    pub fn week_collection(&self) -> Result<WeekCollection> {
        WeekCollection::new(&self.calendars)
    }
}
