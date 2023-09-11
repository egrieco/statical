use chrono::{Local, NaiveDate};
use fuzzydate::parse;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::ops::Deref;

/// Wrapper type for RelativePathBuf so we can use doku to generate example config files
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigDate(NaiveDate);

impl ConfigDate {
    pub(crate) fn now() -> ConfigDate {
        ConfigDate(Local::now().date_naive())
    }

    pub(crate) fn date(&self) -> NaiveDate {
        self.0
    }
}

impl Deref for ConfigDate {
    type Target = NaiveDate;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl doku::Document for ConfigDate {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}

/// Deserialize [`ConfigDates`] in the `statical.toml` file
///
/// This function calls [`fuzzydate::parse`] so we can handle human readable dates like "today"
pub(crate) fn deserialize_config_date<'de, D>(deserializer: D) -> Result<ConfigDate, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;

    parse(buf)
        .map(|d| ConfigDate(d.date()))
        .map_err(Error::custom)
}
