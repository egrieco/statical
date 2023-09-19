use csscolorparser::Color;
use palette::{FromColor, Oklch, Srgb};
use serde::{Deserialize, Serialize};

use crate::configuration::config::Config;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub(crate) struct ConfigColor(pub(crate) Color);

impl ConfigColor {
    pub(crate) fn to_hex_string(&self) -> String {
        self.0.to_hex_string()
    }

    pub(crate) fn adjust_color(&self, config: &Config) -> String {
        let (r, g, b, _a) = self.0.to_linear_rgba();
        // NOTE: its really important to use Oklch or another perceptually normalized color model here
        //       colors will look very strange if they are adjusted in other models
        let mut color = Oklch::from_color(Srgb::new(r, g, b));
        color.chroma = config.adjusted_chroma;
        color.l = config.adjusted_lightness;
        let (or, og, ob) = Srgb::from_color(color).into_components();
        ConfigColor(csscolorparser::Color::from_linear_rgba(or, og, ob, 1.0)).to_hex_string()
    }
}

impl Eq for ConfigColor {}

impl doku::Document for ConfigColor {
    fn ty() -> doku::Type {
        doku::Type::from(doku::TypeKind::String)
    }
}
