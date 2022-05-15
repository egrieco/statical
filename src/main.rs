#![allow(unused_imports)]

extern crate ical;

use clap::{Args, Parser};
use color_eyre::eyre::{self, WrapErr};
use ical::IcalParser;
use std::{fs::File, io::BufReader, path::PathBuf};

use statical::*;

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
    let mut events = Vec::new();

    if let Some(files) = args.file {
        for file in files {
            println!("  Provided path is: {:?}", file);
            if file.exists() {
                println!("    File exists");
                let buf = BufReader::new(File::open(file)?);
                events.append(&mut parse_calendar(buf)?);
            }
        }
    }

    if let Some(urls) = args.url {
        for url in urls {
            println!("  Provided url is: {:?}", url);
            let ics_string = ureq::get(&url).call()?.into_string()?;
            println!("    URL exists");
            events.append(&mut parse_calendar(ics_string.as_bytes())?);
        }
    }

    for event in events {
        println!("{:?}", event);
    }
    Ok(())
}
