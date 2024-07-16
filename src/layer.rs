use file_stream::write::FileStreamWriter;
use graphics::{Image, Rect};

use crate::{
    blend_mode::BlendMode,
    color_channel::{ColorChannel, ColorChannelType},
};

use self::divider_type::DividerType;

mod divider_type;

/// A layer in a Photoshop document.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// The bounds of the layer.
    pub bounds: Rect<f32>,
    /// The number of channels for the layer.
    pub number_of_channels: u16,
    /// The channels for the layer.
    pub channels: Vec<ColorChannel>,
    /// The blend mode for the layer.
    pub blend_mode: BlendMode,
    /// The opacity of the layer (from 0.0 to 1.0).
    pub opacity: f32,
    /// Whether or not the layer is hidden.
    pub is_hidden: bool,
    /// The layer’s name.
    pub name: Option<String>,
    /// The layer image.
    pub image: Option<Image>,
    /// The data for the additional layer information.
    additional_layer_information: Option<Vec<u8>>,
    /// The type of divider this layer represents. Used for
    /// groups and group markers, and set to `other` for
    /// other types of layers.
    divider_type: DividerType,
}

// MARK: Creation

impl Layer {
    /// Creates a new Photoshop document layer.
    pub fn new(bounds: Rect<f32>) -> Self {
        Self {
            bounds,
            number_of_channels: 4,
            channels: Vec::new(),
            blend_mode: BlendMode::Normal,
            opacity: 1.0,
            is_hidden: false,
            name: None,
            image: None,
            additional_layer_information: None,
            divider_type: DividerType::Other,
        }
    }
}

// MARK: Updates

impl Layer {
    /// Updates the channel data for the image.
    fn update_channel_data(&mut self) {
        // Procreate can’t handle empty images, so we create a clear
        // image of the size of document.
        if self.image.is_none() && self.bounds != Rect::zero() {
            self.image = Some(Image::empty(self.bounds.size.rounded().into()));
        }

        let Some(image) = self.image.as_ref() else {
            panic!("No image for layer.")
        };

        // Unlike some formats, this is never padded.
        let channel_data_length = (image.size.width * image.size.height) as usize;
        let mut red_channel = ColorChannel::new(ColorChannelType::Red, channel_data_length);
        let mut green_channel = ColorChannel::new(ColorChannelType::Red, channel_data_length);
        let mut blue_channel = ColorChannel::new(ColorChannelType::Red, channel_data_length);
        let mut alpha_channel = ColorChannel::new(ColorChannelType::Red, channel_data_length);

        let width = image.size.width;
        let height = image.size.height;

        for y_position in 0..height {
            for x_position in 0..width {
                // For now we’re just assuming that the bytes
                // are in RGBA order.
                let target_index = (y_position * width + x_position) as usize;
                let source_index = ((y_position * image.bytes_per_row) + (x_position * 4)) as usize;
                red_channel.data[target_index] = image.data[source_index];
                green_channel.data[target_index] = image.data[source_index + 1];
                blue_channel.data[target_index] = image.data[source_index + 2];
                alpha_channel.data[target_index] = image.data[source_index + 3];
            }
        }

        self.channels = vec![red_channel, green_channel, blue_channel, alpha_channel];
    }
}

// MARK: Encoding

impl Layer {
    /// Returns the image encoded per channel.
    fn encoded_image(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut file_stream = FileStreamWriter::new();
        if self.channels.is_empty() {
            self.update_channel_data();
        }
        let height = self.bounds.size.height as u32;
        for channel in self.channels.iter_mut() {
            let Ok(compressed) = channel.compressed_data(height) else {
                continue;
            };
            file_stream.write_be(&compressed.compression.raw_value())?;
            file_stream.write_bytes(&compressed.data)?;
        }

        Ok(file_stream.data().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use graphics::{Color, Point, Size};

    use super::*;

    #[test]
    fn encoded_image() {
        let bounds = Rect::new(13.0, 12.0, 3.0, 2.0);
        let mut layer = Layer::new(bounds);
        layer.name = Some("Frowning".to_string());
        let image = Image::color(
            &Color {
                red: 128,
                green: 0,
                blue: 128,
                alpha: 255,
            },
            Size {
                width: 2,
                height: 2,
            },
        );
        layer.image = Some(image);

        let expected_data = vec![
            0x00, 0x01, // Compression type
            0x00, 0x04, 0x00, 0x04, 0x02, 0x80, 0x80, 0x00, 0x02, 0x80, 0x80, 0x00, // Red
            0x00, 0x01, // Compression type
            0x00, 0x02, 0x00, 0x02, 0xFE, 0x00, 0xFE, 0x00, // Green
            0x00, 0x01, // Compression type
            0x00, 0x04, 0x00, 0x04, 0x02, 0x80, 0x80, 0x00, 0x02, 0x80, 0x80, 0x00, // Blue
            0x00, 0x01, // Compression type
            0x00, 0x04, 0x00, 0x04, 0x02, 0xFF, 0xFF, 0x00, 0x02, 0xFF, 0xFF, 0x00, // Alpha
        ];

        let encoded_image = layer.encoded_image().unwrap();
        assert_eq!(encoded_image, expected_data);
    }

    #[test]
    fn rle_encoded_image() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/clouds2.png");
        let source_image = Image::open(&path).unwrap();

        let bounds = Rect {
            origin: Point::zero(),
            size: source_image.size.into(),
        };
        let mut layer = Layer::new(bounds);
        layer.image = Some(source_image);

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/clouds-encoded-image.data");
        let expected = std::fs::read(&path).unwrap();

        let result = layer.encoded_image().unwrap();

        assert_eq!(result, expected);
    }
}
