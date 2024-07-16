/// The different types of colour channel.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorChannelType {
    /// The red channel.
    Red,
    /// The green channel.
    Green,
    /// The blue channel.
    Blue,
    /// The alpha channel, or ‘transparency mask’ in the documentation.
    Alpha,
    /// The user supplied layer mask.
    UserSuppliedLayerMask,
    /// The real user supplied layer mask (when both a user mask and a vector mask are present).
    /// What does this mean? It’s straight from the docs.
    RealUserSuppliedLayerMask,
    /// An unknown channel type, used in parsing for anything that isn’t documented.
    Unknown,
}

impl ColorChannelType {
    /// Creates a new color channel type from a raw value.
    pub fn from_value(value: i16) -> Option<Self> {
        match value {
            0 => Some(ColorChannelType::Red),
            1 => Some(ColorChannelType::Green),
            2 => Some(ColorChannelType::Blue),
            -1 => Some(ColorChannelType::Alpha),
            -2 => Some(ColorChannelType::UserSuppliedLayerMask),
            -3 => Some(ColorChannelType::RealUserSuppliedLayerMask),
            9999 => Some(ColorChannelType::Unknown),
            _ => None,
        }
    }

    /// Returns the raw value for the colour channel type.
    pub fn raw_value(&self) -> i16 {
        match self {
            ColorChannelType::Red => 0,
            ColorChannelType::Green => 1,
            ColorChannelType::Blue => 2,
            ColorChannelType::Alpha => -1,
            ColorChannelType::UserSuppliedLayerMask => -2,
            ColorChannelType::RealUserSuppliedLayerMask => -3,
            ColorChannelType::Unknown => 9999,
        }
    }
}
