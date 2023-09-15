use clap::Parser;
use serde::{Deserialize, Serialize};

/// Command line options
#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(name = "statical", author, version, about)]
pub struct Opt {
    /// The config file to read
    ///
    /// The base_dir for the CalendarCollection is also set from this file
    /// so that all files mentioned in the config are relative to the directory containing the config file.
    // TODO: make this a vec so we can run multiple sites at once
    pub config_file: Vec<String>,

    /// Create the example config file in the current directory
    #[clap(long, default_value_t = false)]
    pub create_default_config: bool,

    /// Restore the missing default templates to the templates path specified in the config file
    #[clap(long, default_value_t = false)]
    pub restore_missing_templates: bool,

    /// Restore the missing assets to the assets path specified in the config file
    #[clap(long, default_value_t = false)]
    pub restore_missing_assets: bool,

    /// Do not delete files in the output directory
    #[clap(long, default_value_t = false)]
    pub no_delete: bool,
}
