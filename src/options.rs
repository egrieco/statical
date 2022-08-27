use clap::Parser;
use std::path::PathBuf;

/// Command line options
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Opt {
    /// The config file to read
    #[clap(short, long, default_value_t = String::from("statical.toml"))]
    pub config: String,

    /// The calendar files to read
    #[clap(short, long)]
    pub file: Option<Vec<PathBuf>>,

    /// The calendar urls to read
    #[clap(short, long)]
    pub url: Option<Vec<String>>,
}
