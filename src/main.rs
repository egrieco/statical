use clap::{CommandFactory, Parser};
use color_eyre::eyre::{self, bail, Context};
use flexi_logger::Logger;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::exit,
};

use statical::{
    configuration::{config::Config, options::Opt},
    model::calendar_collection::CalendarCollection,
    util,
};

const DEFAULT_CONFIG_PATH: &str = "statical.toml";

fn main() -> eyre::Result<()> {
    let mut args = Opt::parse();
    color_eyre::install()?;

    if args.create_default_config {
        // TODO: figure out how to pre-populate the calendar sources with example data
        // TODO: maybe allow the user to set a specific path for the config file
        if Path::new(DEFAULT_CONFIG_PATH).exists() {
            bail!("config file already exists at: statical.toml");
        } else {
            File::create(DEFAULT_CONFIG_PATH)
                .wrap_err("could not create default config file")?
                .write_all(doku::to_toml::<Config>().as_bytes())
                .wrap_err("could not write config to default config file")?;
        }

        exit(0);
    }

    // setup logging
    Logger::try_with_env_or_str("debug")?.start()?;

    // if no config file is provided, check for one in the current directory
    if args.config_file.is_empty() {
        if PathBuf::from(DEFAULT_CONFIG_PATH).exists() {
            args.config_file.push(String::from(DEFAULT_CONFIG_PATH))
        } else {
            // TODO: print help text here if no config files are provided or found
            println!(
                "
Statical needs a configuration file to run.

Please specify a config file or run: statical --create-default-config

Full Help Text
--------------
"
            );
            // bail!("no config file provided or found in the local directory")
            Opt::command().print_long_help()?;
            exit(1);
        }
    }

    // run statical for every config file specified
    // TODO: may want to deduplicate the config files
    for config_path in &args.config_file {
        let config = Config::new(config_path, &args)?;

        if args.restore_missing_templates {
            let template_path = &config.base_dir.join(&config.template_path);
            eprintln!("template_path: {:?}", template_path);
            util::restore_missing_templates(template_path)?;
        }

        if args.restore_missing_assets {
            let assets_path = &config.base_dir.join(&config.assets_path);
            eprintln!("template_path: {:?}", assets_path);
            util::restore_missing_assets(assets_path)?;
        }

        if args.restore_missing_templates || args.restore_missing_assets {
            // skip the rest of the normal statical actions
            continue;
        }

        log::info!("creating calendar collection...");
        let calendar_collection = CalendarCollection::new(config)?;

        log::info!("writing html pages");
        calendar_collection.create_view_files()?;

        log::info!("final debug output");
        calendar_collection.print_unparsed_properties();
    }

    Ok(())
}
