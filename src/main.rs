use clap::Parser;
use color_eyre::eyre::{self};
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use flexi_logger::Logger;
use std::process::exit;

use statical::{config::Config, model::calendar_collection::CalendarCollection, options::Opt};

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

    log::info!("reading configuration...");
    let config: Config = Figment::from(Serialized::defaults(Config::default()))
        .merge(Toml::file("statical.toml"))
        .admerge(Serialized::defaults(args))
        .extract()?;

    log::info!("creating calendar collection...");
    let calendar_collection = CalendarCollection::new(config)?;

    log::info!("writing html pages");
    calendar_collection.create_html_pages()?;

    log::info!("final debug output");
    calendar_collection.print_unparsed_properties();

    Ok(())
}
