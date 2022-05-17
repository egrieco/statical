extern crate ical;

use clap::StructOpt;
use color_eyre::eyre::{self};
use statical::{model::calendar_collection::CalendarCollection, options::Opt};

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    println!("  Arguments: {:#?}", args);

    CalendarCollection::new(args)?
        .week_collection()?
        .create_week_pages()?;

    Ok(())
}
