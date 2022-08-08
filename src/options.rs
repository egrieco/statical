use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Opt {
    /// The config file to read
    #[clap(short, long, default_value_t = String::from("statical.toml"))]
    pub config: String,

    /// The calendar file to read
    #[clap(short, long)]
    pub file: Option<Vec<PathBuf>>,

    /// The calendar url to read
    #[clap(short, long)]
    pub url: Option<Vec<String>>,
}
