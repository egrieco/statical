extern crate ical;
extern crate serde_json;

use clap::StructOpt;
use color_eyre::eyre::{self};
use statical::{model::calendar_collection::CalendarCollection, options::Opt};
use std::path::PathBuf;

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    let calendar_collection = CalendarCollection::new(args)?;
    calendar_collection
        // TODO take the output path from the config
        .create_week_pages(&PathBuf::from("output/week"))?;

    Ok(())
}
