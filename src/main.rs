extern crate ical;
extern crate serde_json;

use std::path::PathBuf;

use clap::StructOpt;
use color_eyre::eyre::{self};
use statical::{model::calendar_collection::CalendarCollection, options::Opt};

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    println!("  Arguments: {:#?}", args);

    let calendar_collection = CalendarCollection::new(args)?;
    calendar_collection
        .week_collection()?
        // TODO take the output path from the config
        .create_week_pages(&calendar_collection, &PathBuf::from("output/week"))?;

    Ok(())
}
