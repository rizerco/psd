/// Colour modes used in a Photoshop document.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    /// The bitmap colour mode.
    Bitmap,
    /// The grayscale colour mode.
    Grayscale,
    /// The indexed colour mode.
    Indexed,
    /// The RGB colour mode.
    Rgb,
    /// The CMYK colour mode.
    Cmyk,
    /// The multichannel colour mode.
    Multichannel,
    /// The duotone colour mode.
    Duotone,
    /// The Lab colour mode.
    Lab,
}

impl ColorMode {
    /// Creates a new colour mode from a raw value.
    pub fn from_value(value: i16) -> Option<Self> {
        match value {
            0 => Some(ColorMode::Bitmap),
            1 => Some(ColorMode::Grayscale),
            2 => Some(ColorMode::Indexed),
            3 => Some(ColorMode::Rgb),
            4 => Some(ColorMode::Cmyk),
            7 => Some(ColorMode::Multichannel),
            8 => Some(ColorMode::Duotone),
            9 => Some(ColorMode::Lab),
            _ => None,
        }
    }

    /// Returns the raw value for the colour mode.
    pub fn raw_value(&self) -> i16 {
        match self {
            ColorMode::Bitmap => 0,
            ColorMode::Grayscale => 1,
            ColorMode::Indexed => 2,
            ColorMode::Rgb => 3,
            ColorMode::Cmyk => 4,
            ColorMode::Multichannel => 7,
            ColorMode::Duotone => 8,
            ColorMode::Lab => 9,
        }
    }
}
