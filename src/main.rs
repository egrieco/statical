use clap::Parser;
use color_eyre::eyre::{self, Context};
use flexi_logger::Logger;
use std::io::{Read, Write};
use toml_edit::ser::to_string_pretty;
use toml_edit::Document;

use statical::{
    config::{Config, ParsedConfig},
    model::calendar_collection::CalendarCollection,
    options::Opt,
};

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    // setup logging
    Logger::try_with_env_or_str("debug")?.start()?;

    log::info!("reading configuration...");
    let config: ParsedConfig = if let Ok(mut config_file) = std::fs::File::open(&args.config) {
        let mut config_raw = String::new();
        config_file.read_to_string(&mut config_raw)?;
        let parsed_toml = &config_raw.parse::<Document>()?;
        <&toml_edit::Document as std::convert::Into<Config>>::into(parsed_toml)
    } else {
        let config: statical::config::Config = Default::default();
        if let Ok(mut config_file) = std::fs::File::create(&args.config) {
            if let Ok(config_raw) = to_string_pretty(&config) {
                config_file.write_all(config_raw.as_bytes()).ok();
            }
        }
        config
    }
    .parse()
    .wrap_err("could not parse config")?;

    log::info!("creating calendar collection...");
    let calendar_collection = CalendarCollection::new(args, config)?;

    log::info!("writing html pages");
    calendar_collection.create_html_pages()?;

    log::info!("final debug output");
    calendar_collection.print_unparsed_properties();

    Ok(())
}
