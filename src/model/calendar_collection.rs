use color_eyre::eyre::{self, Result, WrapErr};
use std::io::Write;
use std::task::Context;
use std::{fs::File, io::BufReader};
use tera::Tera;

use crate::model::calendar::Calendar;
use crate::options::Opt;
use crate::view::week::WeekCollection;

pub struct CalendarCollection {
    calendars: Vec<Calendar>,
    tera: Tera,
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

        Ok(CalendarCollection {
            calendars,
            tera: Tera::new("templates/**/*.html")?,
        })
    }

    pub fn week_collection(&self) -> Result<WeekCollection> {
        WeekCollection::new(&self)
    }

    /// Get a reference to the calendar collection's calendars.
    #[must_use]
    pub fn calendars(&self) -> &[Calendar] {
        self.calendars.as_ref()
    }

    /// Get a reference to the calendar collection's tera.
    #[must_use]
    pub fn tera(&self) -> &Tera {
        &self.tera
    }

    pub fn render(&self, template_name: &str, context: &tera::Context) -> eyre::Result<String> {
        Ok(self.tera.render(template_name, context)?)
    }

    pub fn render_to(
        &self,
        template_name: &str,
        context: &tera::Context,
        write: impl Write,
    ) -> eyre::Result<()> {
        Ok(self.tera.render_to(template_name, context, write)?)
    }
}
