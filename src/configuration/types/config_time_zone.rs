use chrono_tz::Tz as ChronoTz;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Wrapper type for chrono_tz::Tz so we can use doku to generate example config files
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ConfigTimeZone(pub chrono_tz::Tz);

impl ConfigTimeZone {
    pub fn timezone(&self) -> ChronoTz {
        self.0
    }
}

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

impl doku::Document for ConfigTimeZone {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}
