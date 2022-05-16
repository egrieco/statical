extern crate ical;

use clap::Parser;
use color_eyre::eyre::{self, WrapErr};
use statical::model::calendar::Calendar;
use statical::view::week::WeekCollection;

use std::{fs::File, io::BufReader, path::PathBuf};

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Opt {
    /// The calendar file to read
    #[clap(short, long)]
    file: Option<Vec<PathBuf>>,

    /// The calendar url to read
    #[clap(short, long)]
    url: Option<Vec<String>>,
}

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

    WeekCollection::new(&calendars);

    Ok(())
}
