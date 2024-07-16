use graphics::{Image, Size};

use crate::{color_mode::ColorMode, layer::Layer};

mod constants;

/// A Photoshop document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The number of channels in the image, including any alpha channels. Supported range is 1 to 56.
    pub number_of_channels: u16,
    /// The size of the image in pixels. Supported range is 1 to 30,000 for width and height.
    pub size: Size<u32>,
    /// The number of bits per channel. Supported values are 1, 8, 16 and 32.
    pub bits_per_channel: u16,
    /// The colour mode of the file.
    pub color_mode: ColorMode,
    /// The preview image for the whole document.
    pub preview_image: Option<Image>,
    /// The document’s layers.
    pub layers: Vec<Layer>,
}

// MARK: Creation

impl Document {
    /// Creates an empty photoshop document.
    pub fn new() -> Self {
        Self {
            number_of_channels: 4,
            size: Size::zero(),
            bits_per_channel: 1,
            color_mode: ColorMode::Bitmap,
            preview_image: None,
            layers: Vec::new(),
        }
    }
}
