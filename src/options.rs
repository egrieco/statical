use clap::Parser;

/// Command line options
#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Opt {
    /// The config file to read
    #[clap(short, long, default_value_t = String::from("statical.toml"))]
    pub config: String,

    /// The calendar sources to read (can be URLs or file paths)
    #[clap(short, long)]
    pub source: Option<Vec<String>>,
}
