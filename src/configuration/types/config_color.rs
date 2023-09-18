use csscolorparser::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub(crate) struct ConfigColor(pub(crate) Color);

impl ConfigColor {
    pub(crate) fn to_hex_string(&self) -> String {
        self.0.to_hex_string()
    }
}

impl Eq for ConfigColor {}

impl doku::Document for ConfigColor {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}
