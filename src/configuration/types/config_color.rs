use csscolorparser::Color;
use palette::{FromColor, Oklch, Srgb};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub(crate) struct ConfigColor(pub(crate) Color);

impl ConfigColor {
    pub(crate) fn to_hex_string(&self) -> String {
        self.0.to_hex_string()
    }

    // TODO: need to do this math once per color, and not on every call to this method
    pub(crate) fn to_adjusted_hex_string(&self) -> String {
        let (r, g, b, _a) = self.0.to_linear_rgba();
        // NOTE: its really important to use Oklch or another perceptually normalized color model here
        //       colors will look very strange if they are adjusted in other models
        let mut color = Oklch::from_color(Srgb::new(r, g, b));
        color.chroma = 0.15;
        color.l = 0.9;
        let (or, og, ob) = Srgb::from_color(color).into_components();
        csscolorparser::Color::from_linear_rgba(or, og, ob, 1.0).to_hex_string()
    }
}

impl Eq for ConfigColor {}

impl doku::Document for ConfigColor {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}
