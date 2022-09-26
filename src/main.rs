use clap::StructOpt;
use color_eyre::eyre::{self};
use statical::{model::calendar_collection::CalendarCollection, options::Opt};
use std::io::{Read, Write};

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    let config = if let Ok(mut config_file) = std::fs::File::open(&args.config) {
        let mut config_raw = String::new();
        config_file.read_to_string(&mut config_raw)?;
        toml_edit::easy::from_str(&config_raw)?
    } else {
        let config: statical::config::Config = Default::default();
        if let Ok(mut config_file) = std::fs::File::create(&args.config) {
            if let Ok(config_raw) = toml_edit::easy::to_string_pretty(&config) {
                config_file.write_all(config_raw.as_bytes()).ok();
            }
        }
        config
    };

    let calendar_collection = CalendarCollection::new(args, config.parse()?)?;

    calendar_collection.create_html_pages()?;

    if config.render_month {
        calendar_collection.create_month_pages()?;
    }

    if config.render_agenda {
        calendar_collection.create_agenda_pages()?;
    }

    Ok(())
}
