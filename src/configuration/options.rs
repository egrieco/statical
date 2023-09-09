use clap::Parser;
use serde::{Deserialize, Serialize};

/// Command line options
#[derive(Parser, Debug, Serialize, Deserialize)]
#[clap(author, version, about)]
pub struct Opt {
    /// The config file to read
    ///
    /// The base_dir for the CalendarCollection is also set from this file
    /// so that all files mentioned in the config are relative to the directory containing the config file.
    // TODO: make this a vec so we can run multiple sites at once
    #[clap(short, long, default_value_t = String::from("statical.toml"))]
    pub config: String,

    /// The calendar sources to read (can be URLs or file paths)
    #[clap(short, long)]
    pub source: Option<Vec<String>>,

    /// Generate the example config template
    #[clap(long, default_value_t = false)]
    pub generate_default_config: bool,

    /// Do not delete files in the output directory
    #[clap(long, default_value_t = false)]
    pub no_delete: bool,
}
