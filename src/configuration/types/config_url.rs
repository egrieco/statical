use serde::{Deserialize, Serialize};
use std::ops::Deref;
use unix_path::{Path as UnixPath, PathBuf as UnixPathBuf};

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
