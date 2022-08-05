use clap::StructOpt;
use color_eyre::eyre::{self};
use statical::{model::calendar_collection::CalendarCollection, options::Opt};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

mod options;

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    let config = if let Ok(mut config_file) = std::fs::File::open("statical.toml") {
        let mut config_raw = String::new();
        config_file.read_to_string(&mut config_raw)?;
        toml_edit::easy::from_str(&config_raw)?
    } else {
        let config = Default::default();
        if let Ok(mut config_file) = std::fs::File::create("statical.toml") {
            if let Ok(config_raw) = toml_edit::easy::to_string_pretty(&config) {
                config_file.write_all(config_raw.as_bytes()).ok();
            }
        }
        config
    };

    let calendar_collection = CalendarCollection::new(args, &config)?;

    if config.render_month {
        calendar_collection.create_month_pages(&PathBuf::from(&config.output_dir).join("month"))?;
    }

    if config.render_week {
        calendar_collection.create_week_pages(&PathBuf::from(&config.output_dir).join("week"))?;
    }

    if config.render_day {
        calendar_collection.create_day_pages(&PathBuf::from(&config.output_dir).join("day"))?;
    }

    if config.render_agenda {
        calendar_collection
            .create_agenda_pages(&PathBuf::from(&config.output_dir).join("agenda"))?;
    }

    Ok(())
}
