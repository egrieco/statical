use csscolorparser::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ConfigColor(pub(crate) Color);

impl doku::Document for ConfigColor {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}
