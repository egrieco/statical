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
    file: Option<PathBuf>,

    /// The calendar url to read
    #[clap(short, long)]
    url: Option<String>,
}

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    println!("Arguments: {:#?}", args);
    if let Some(file) = args.file {
        println!("Provided path is: {:?}", file);
        if file.exists() {
            println!("  file exists");
            let buf = BufReader::new(File::open(file)?);
            let reader = IcalParser::new(buf);
            for entry in reader {
                println!("{:#?}", entry);
            }
        }
    }

    if let Some(url) = args.url {
        println!("Provided url is: {:?}", url);
        let ics_string = ureq::get(&url).call()?.into_string()?;
        println!("URL exists");
        // let buf = BufReader::new(ics_string);
        let reader = IcalParser::new(ics_string.as_bytes());
        for entry in reader {
            println!("{:#?}", entry);
        }
    }

    Ok(())
}
