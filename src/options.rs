use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Opt {
    /// The calendar file to read
    #[clap(short, long)]
    pub file: Option<Vec<PathBuf>>,

    /// The calendar url to read
    #[clap(short, long)]
    pub url: Option<Vec<String>>,
}
