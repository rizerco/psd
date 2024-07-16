/// Colour spaces used in a Photoshop colour swatch.
pub enum ColorSpace {
    /// The RGB colour space.
    Rgb,
    /// The HSB colour space.
    Hsb,
    /// The CMYK colour space.
    Cmyk,
    /// The Lab colour space.
    Lab,
    /// The greyscale colour space.
    Grayscale,
}

impl ColorSpace {
    /// Creates a new colour space from a raw value.
    pub fn from_value(value: i16) -> Option<Self> {
        match value {
            0 => Some(ColorSpace::Rgb),
            1 => Some(ColorSpace::Hsb),
            2 => Some(ColorSpace::Cmyk),
            7 => Some(ColorSpace::Lab),
            8 => Some(ColorSpace::Grayscale),
            _ => None,
        }
    }

    /// Returns the raw value for the colour space.
    pub fn color_mode(&self) -> i16 {
        match self {
            ColorSpace::Rgb => 0,
            ColorSpace::Hsb => 1,
            ColorSpace::Cmyk => 2,
            ColorSpace::Lab => 7,
            ColorSpace::Grayscale => 8,
        }
    }
}
