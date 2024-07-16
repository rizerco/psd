/// The image compression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageCompression {
    /// Raw data.
    RawData,
    /// RLE compression.
    Rle,
    /// ZIP without prediction.
    ZipWithoutPrediction,
    /// ZIP with prediction.
    ZipWithPrediction,
}

impl ImageCompression {
    /// Creates a new image compression from a raw value.
    pub fn from_value(value: i16) -> Option<Self> {
        match value {
            0 => Some(ImageCompression::RawData),
            1 => Some(ImageCompression::Rle),
            2 => Some(ImageCompression::ZipWithoutPrediction),
            3 => Some(ImageCompression::ZipWithPrediction),
            _ => None,
        }
    }

    /// Returns the raw value for the image compression.
    pub fn raw_value(&self) -> i16 {
        match self {
            ImageCompression::RawData => 0,
            ImageCompression::Rle => 1,
            ImageCompression::ZipWithoutPrediction => 2,
            ImageCompression::ZipWithPrediction => 3,
        }
    }
}
