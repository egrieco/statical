use doku::Document;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use unix_path::{Path as UnixPath, PathBuf as UnixPathBuf};

/// Wrapper type for chrono_tz::Tz so we can use doku to generate example config files
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigTimeZone(pub chrono_tz::Tz);

impl Deref for ConfigTimeZone {
    type Target = chrono_tz::Tz;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ConfigTimeZone> for chrono_tz::Tz {
    fn from(value: ConfigTimeZone) -> Self {
        value.0
    }
}

/// Wrapper type for RelativePathBuf so we can use doku to generate example config files
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigUrl(UnixPathBuf);

impl ConfigUrl {
    pub fn path_buf(&self) -> &UnixPathBuf {
        &self.0
    }
}

impl Deref for ConfigUrl {
    type Target = UnixPathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ConfigUrl> for UnixPathBuf {
    fn from(value: ConfigUrl) -> Self {
        value.0
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Document)]
pub(crate) enum CalendarView {
    Month,
    Week,
    Day,
    Agenda,
}

impl doku::Document for ConfigTimeZone {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}

impl doku::Document for ConfigUrl {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}

impl From<&str> for ConfigUrl {
    fn from(value: &str) -> Self {
        ConfigUrl(UnixPath::new(value).into())
    }
}
