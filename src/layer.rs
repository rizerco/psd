use std::mem;

use file_stream::write::FileStreamWriter;
use graphics::{Image, Rect};

use crate::blend_mode::BlendMode;
use crate::color_channel::{ColorChannel, ColorChannelType};
use crate::data;
use crate::document;
use crate::string;

use self::divider_type::DividerType;
use self::group::GroupInfo;

mod divider_type;
mod group;

/// A layer in a Photoshop document.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// The layer type.
    pub layer_type: LayerType,
    /// The bounds of the layer.
    pub bounds: Rect<i32>,
    /// The number of channels for the layer.
    pub number_of_channels: i16,
    /// The channels for the layer.
    pub channels: Vec<ColorChannel>,
    /// The blend mode for the layer.
    pub blend_mode: BlendMode,
    /// The opacity of the layer (from 0.0 to 1.0).
    pub opacity: u8,
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

/// The type of the layer.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerType {
    /// A standard layer that shows an image.
    Image,
    /// A group layer.
    Group(GroupInfo),
}

// MARK: Creation

impl Layer {
    /// Creates a new Photoshop document layer.
    pub fn new(bounds: Rect<i32>) -> Self {
        Self {
            layer_type: LayerType::Image,
            bounds,
            number_of_channels: 4,
            channels: Vec::new(),
            blend_mode: BlendMode::Normal,
            opacity: u8::MAX,
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
            self.image = Some(Image::empty(self.bounds.size.into()));
        }

        let Some(image) = self.image.as_ref() else {
            panic!("No image for layer.")
        };

        // Unlike some formats, this is never padded.
        let channel_data_length = (image.size.width * image.size.height) as usize;
        let mut red_channel = ColorChannel::new(ColorChannelType::Red, channel_data_length);
        let mut green_channel = ColorChannel::new(ColorChannelType::Green, channel_data_length);
        let mut blue_channel = ColorChannel::new(ColorChannelType::Blue, channel_data_length);
        let mut alpha_channel = ColorChannel::new(ColorChannelType::Alpha, channel_data_length);

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
    pub fn encoded_image(&mut self) -> anyhow::Result<Vec<u8>> {
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

    /// Returns the data for the layer record.
    pub fn layer_record_data(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut file_stream = FileStreamWriter::new();

        // The rectangle / bounds.
        let top = self.bounds.min_y();
        let left = self.bounds.min_x();
        let bottom = self.bounds.max_y();
        let right = self.bounds.max_x();
        file_stream.write_be(&top)?;
        file_stream.write_be(&left)?;
        file_stream.write_be(&bottom)?;
        file_stream.write_be(&right)?;

        // The number of channels.
        file_stream.write_be(&self.number_of_channels)?;

        if self.channels.is_empty() {
            self.update_channel_data();
        }

        // The channel information.
        for channel in self.channels.iter_mut() {
            file_stream.write_be(&channel.color_type.raw_value())?;

            // The size is the size of the data plus the compression type byte.
            let Ok(result) = channel.compressed_data(self.bounds.height() as u32) else {
                continue;
            };
            file_stream.write_be(&(result.data.len() as u32 + mem::size_of::<i16>() as u32))?;
        }

        file_stream.write_bytes(&document::constants::RESOURCE_SIGNATURE)?;
        file_stream.write_bytes(&self.blend_mode.as_str().as_bytes())?;

        file_stream.write_be(&self.opacity)?;

        // Clipping… still don’t know what it means.
        file_stream.write_be(&0u8)?;

        // The flags… we only care about the visible flag (opposite to the documentation).
        let flags: u8 = if self.is_hidden {
            0b00000010
        } else {
            0b00000000
        };
        file_stream.write_be(&flags)?;

        // Filler.
        file_stream.write_be(&0u8)?;

        let mut extra_data_file_stream = FileStreamWriter::new();
        // Layer mask data — there are no layer masks, so we just put
        // a zero for the size of this section.
        extra_data_file_stream.write_be(&0u32)?;

        // Layer blending ranges — can this be zero too?
        extra_data_file_stream.write_be(&0u32)?;

        let mut name_data = string::pascal::data_from_string(self.name.as_ref())?;
        data::pad(&mut name_data, 4);
        extra_data_file_stream.write_bytes(&name_data)?;

        let unicode_name_data = string::unicode::data_from_string(self.name.as_ref())?;
        extra_data_file_stream.write_bytes(&unicode_name_data)?;

        if let Some(layer_information) = &self.additional_layer_information {
            extra_data_file_stream.write_bytes(layer_information)?;
        }

        file_stream.write_be(&(extra_data_file_stream.data().len() as u32))?;
        file_stream.write_bytes(&extra_data_file_stream.data())?;

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
        let bounds = Rect::new(13, 12, 3, 2);
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
    fn channel_data() {
        let mut resources_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        resources_path.push("tests/resources");
        let alpha_data = std::fs::read(&resources_path.join("alphachannel.data")).unwrap();
        let red_data = std::fs::read(&resources_path.join("redchannel.data")).unwrap();
        let green_data = std::fs::read(&resources_path.join("greenchannel.data")).unwrap();
        let blue_data = std::fs::read(&resources_path.join("bluechannel.data")).unwrap();
        let image = Image::open(resources_path.join("layer.png")).unwrap();

        let mut layer = Layer::new(Rect::new(6, 1, 764, 114));
        layer.image = Some(image);
        layer.name = Some("Layer".to_string());

        // Need to call this to poulate the channels.
        layer.encoded_image().unwrap();

        let result_alpha = &layer
            .channels
            .iter()
            .find(|channel| channel.color_type == ColorChannelType::Alpha)
            .unwrap()
            .data;
        assert_eq!(result_alpha, &alpha_data);

        let result_red = &layer
            .channels
            .iter()
            .find(|channel| channel.color_type == ColorChannelType::Red)
            .unwrap()
            .data;
        assert_eq!(result_red, &red_data);

        let result_green = &layer
            .channels
            .iter()
            .find(|channel| channel.color_type == ColorChannelType::Green)
            .unwrap()
            .data;
        assert_eq!(result_green, &green_data);

        let result_blue = &layer
            .channels
            .iter()
            .find(|channel| channel.color_type == ColorChannelType::Blue)
            .unwrap()
            .data;
        assert_eq!(result_blue, &blue_data);
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

    #[test]
    fn clouds_layer_record_data() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/tiny-clouds.png");

        let source_image = Image::open(&path).unwrap();

        let bounds = Rect {
            origin: Point::zero(),
            size: source_image.size.into(),
        };
        let mut layer = Layer::new(bounds);
        layer.name = Some("L1".to_string());
        layer.image = Some(source_image);
        // 00000000 00000000 00000004 00000007 0004

        let result = layer.layer_record_data().unwrap();

        // Top
        assert_eq!(result[0..=3], [0x00, 0x00, 0x00, 0x00]);
        // Left
        assert_eq!(result[4..=7], [0x00, 0x00, 0x00, 0x00]);
        // Bottom
        assert_eq!(result[8..=11], [0x00, 0x00, 0x00, 0x04]);
        // Right
        assert_eq!(result[12..=15], [0x00, 0x00, 0x00, 0x07]);

        // Number of channels
        assert_eq!(result[16..=17], [0x00, 0x04]);

        // Channel type (red)
        assert_eq!(result[18..=19], [0x00, 0x00]);
        // Channel data length.
        assert_eq!(result[20..=23], [0x00, 0x00, 0x00, 0x1b]);

        // Channel type (green)
        assert_eq!(result[24..=25], [0x00, 0x01]);
        // Channel data length.
        assert_eq!(result[26..=29], [0x00, 0x00, 0x00, 0x1b]);

        // Channel type (blue)
        assert_eq!(result[30..=31], [0x00, 0x02]);
        // Channel data length.
        assert_eq!(result[32..=35], [0x00, 0x00, 0x00, 0x12]);

        // Channel type (alpha)
        assert_eq!(result[36..=37], [0xff, 0xff]);
        // Channel data length.
        assert_eq!(result[38..=41], [0x00, 0x00, 0x00, 0x12]);

        // Resource signature (8BIM).
        assert_eq!(result[42..=45], [0x38, 0x42, 0x49, 0x4d]);
        // Blend mode (norm).
        assert_eq!(result[46..=49], [0x6e, 0x6f, 0x72, 0x6d]);

        // Opacity
        assert_eq!(result[50], 0xff);
        // Clipping
        assert_eq!(result[51], 0x00);
        // Flags (including visibility).
        assert_eq!(result[52], 0x00);
        // Filler.
        assert_eq!(result[53], 0x00);

        // Length of extra data (12).
        assert_eq!(result[54..=57], [0x00, 0x00, 0x00, 0x20]);

        // Mask.
        assert_eq!(result[58..=61], [0x00, 0x00, 0x00, 0x00]);
        // Blending ranges.
        assert_eq!(result[62..=65], [0x00, 0x00, 0x00, 0x00]);

        // Pascal name.
        assert_eq!(result[66..=69], [0x02, 0x4c, 0x31, 0x00]);
    }
}
