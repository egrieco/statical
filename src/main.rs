extern crate ical;

use clap::StructOpt;
use color_eyre::eyre::{self, WrapErr};
use statical::model::calendar::Calendar;
use statical::view::week::WeekCollection;
use std::{fs::File, io::BufReader};

mod options;

use crate::options::Opt;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    println!("  Arguments: {:#?}", args);
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
    }

    if let Some(urls) = args.url {
        for url in urls {
            println!("  Provided url is: {:?}", url);
            let ics_string = ureq::get(&url).call()?.into_string()?;
            println!("    URL exists");
            calendars.append(&mut Calendar::parse_calendars(ics_string.as_bytes())?);
        }
    }

    WeekCollection::new(&calendars).unwrap().create_week_pages();

    Ok(())
}
