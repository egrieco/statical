use clap::Parser;
use color_eyre::eyre::{self};
use flexi_logger::Logger;
use std::process::exit;

use statical::{
    configuration::{config::Config, options::Opt},
    model::calendar_collection::CalendarCollection,
};

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    if args.generate_default_config {
        // TODO: figure out how to pre-populate the calendar sources with example data
        println!("{}", doku::to_toml::<Config>());

        exit(0);
    }

    // setup logging
    Logger::try_with_env_or_str("debug")?.start()?;

    log::info!("creating calendar collection...");
    let calendar_collection = CalendarCollection::new(args)?;

    log::info!("writing html pages");
    calendar_collection.create_html_pages()?;

    log::info!("final debug output");
    calendar_collection.print_unparsed_properties();

    Ok(())
}
