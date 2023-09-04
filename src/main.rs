use clap::Parser;
use color_eyre::eyre::{self};
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use flexi_logger::Logger;

use statical::{config::Config, model::calendar_collection::CalendarCollection, options::Opt};

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    // setup logging
    Logger::try_with_env_or_str("debug")?.start()?;

    log::info!("reading configuration...");
    let config: Config = Figment::from(Serialized::defaults(Config::default()))
        .merge(Toml::file("statical.toml"))
        .admerge(Serialized::defaults(args))
        .extract()?;

    println!("{:#?}", config);

    log::info!("creating calendar collection...");
    let calendar_collection = CalendarCollection::new(config)?;

    log::info!("writing html pages");
    calendar_collection.create_html_pages()?;

    log::info!("final debug output");
    calendar_collection.print_unparsed_properties();

    Ok(())
}
