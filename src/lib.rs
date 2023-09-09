//! # Statical
//!
//! `statical` is a calendar aggregator and generator that aims to make maintaining calendars on static websites easier and cheaper.
//!
//! It reads a collection of `*.ics` files or calendar feeds and creates a collection of `html` files containing all of the events found in the source files and feeds.

pub mod configuration;
pub mod model;
pub mod util;
pub mod views;

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::{assert_eq, assert_ne};

    use super::*;
}
