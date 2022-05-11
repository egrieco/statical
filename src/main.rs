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
    file: PathBuf,
}

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    println!("Arguments: {:#?}", args);
    println!("Provided path is: {:?}", args.file);
    if args.file.exists() {
        println!("  file exists");
        let buf = BufReader::new(File::open(args.file)?);
        let reader = IcalParser::new(buf);
        for entry in reader {
            println!("{:#?}", entry);
        }
    }

    Ok(())
}
