#![allow(unused_imports)]

use color_eyre::eyre::{self, WrapErr};

/// The string literal `"hello, world!"`
/// ```
/// use statical::hello;
/// assert_eq!(hello(), "hello, world!");
/// ```
pub fn hello() -> &'static str {
    "hello, world!"
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::{assert_eq, assert_ne};

    use super::*;

    #[test]
    fn hello_test() {
        assert_eq!(hello(), "hello, world!");
    }
}
