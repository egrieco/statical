#![allow(unused_imports)]

use clap::{Args, Parser};
use color_eyre::eyre::{self, WrapErr};

use statical::*;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Opt {
    /// Tracing filter.
    ///
    /// Can be any of "error", "warn", "info", "debug", or
    /// "trace". Supports more granular filtering, as well; see documentation for
    /// [`tracing_subscriber::EnvFilter`][EnvFilter].
    ///
    /// [EnvFilter]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/struct.EnvFilter.html
    #[clap(long, default_value = "info")]
    tracing_filter: String,
}

fn main() -> eyre::Result<()> {
    let args = Opt::parse();
    color_eyre::install()?;

    println!("Arguments: {:#?}", args);

    Ok(())
}
