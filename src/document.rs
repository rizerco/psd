use std::mem;

use file_stream::write::FileStreamWriter;
use graphics::{Image, Point, Rect, Size};

use crate::{color_mode::ColorMode, data, layer::Layer, layer_container::LayerContainer};

pub(crate) mod constants;

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

// MARK: Export

impl Document {
    /// Return the data for the file.
    pub fn file_data(&mut self) -> anyhow::Result<Vec<u8>> {
        // TODO: Create a file stream on disk to avoid
        // potentially running out of RAM.
        let mut file_stream = FileStreamWriter::new();

        // HEADER SECTION
        file_stream.write_bytes(&constants::FILE_SIGNATURE)?;
        file_stream.write_be(&1i16)?;

        // Six bytes of padding.
        file_stream.write_zeros(6)?;

        // The number of channels — always 4 for RGBA.
        file_stream.write_be(&self.number_of_channels)?;

        // The size of the image.
        file_stream.write_be(&self.size.height)?;
        file_stream.write_be(&self.size.width)?;

        // The colour depth.
        file_stream.write_be(&8i16)?;

        // The colour mode.
        file_stream.write_be(&ColorMode::Rgb.raw_value())?;

        // The colour mode data (come back to this when we have indexed documents).
        file_stream.write_be(&0u32)?;

        // IMAGE RESOURCES SECTION
        // Kind of a second header, with meta-information.
        let mut image_resources_file_stream = FileStreamWriter::new();
        image_resources_file_stream.write_bytes(&constants::RESOURCE_SIGNATURE)?;

        // The resolution info.
        image_resources_file_stream
            .write_be(&constants::resource_identifiers::RESOLUTION_INFORMATION)?;
        // Write null for the name.
        image_resources_file_stream.write_be(&0i16)?;
        // We don’t have the definition for this, so the bytes are hard coded.
        let resolution_information_data = vec![
            0x00, 0x48, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x48, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01,
        ];
        image_resources_file_stream.write_be(&(resolution_information_data.len() as u32))?;
        image_resources_file_stream.write_bytes(&resolution_information_data)?;

        // Selected layer (set to zero).
        image_resources_file_stream.write_bytes(&constants::RESOURCE_SIGNATURE)?;
        image_resources_file_stream.write_be(&constants::resource_identifiers::LAYER_STATE)?;

        // Write null for the name.
        image_resources_file_stream.write_be(&0i16)?;
        // The size is 2 bytes.
        image_resources_file_stream.write_be(&2u32)?;
        // Write 0 for the actual data.
        image_resources_file_stream.write_be(&0u16)?;

        // The layers group information.
        image_resources_file_stream.write_bytes(&constants::RESOURCE_SIGNATURE)?;
        image_resources_file_stream
            .write_be(&constants::resource_identifiers::LAYERS_GROUP_INFORMATION)?;
        // Write null for the name.
        image_resources_file_stream.write_be(&0i16)?;

        // Write the size of the group IDs data.
        let layers_group_identifiers_size =
            self.number_of_layers() as u32 * mem::size_of::<u16>() as u32;
        dbg!(layers_group_identifiers_size);
        image_resources_file_stream.write_be(&layers_group_identifiers_size)?;
        // image_resources_file_stream.write_be(&255u8)?;
        // For each layer (including groups), output the group ID.
        for _ in 0..self.number_of_layers() {
            image_resources_file_stream.write_be(&0i16)?;
        }

        // Write the images resourcces section.
        file_stream.write_be(&(image_resources_file_stream.data().len() as u32))?;
        file_stream.write_bytes(&image_resources_file_stream.data())?;

        // LAYER AND MASK INFORMATION SECTION
        let mut layer_and_mask_info_file_stream = FileStreamWriter::new();

        let mut layer_info_file_stream = FileStreamWriter::new();
        layer_info_file_stream.write_be(&((self.number_of_layers() as i16) * -1))?;

        // Layer records.
        for layer in self.layers.iter_mut() {
            // Procreate can’t handle zero width and height.
            if layer.bounds == Rect::zero() {
                layer.bounds = Rect {
                    origin: Point::zero(),
                    size: self.size.into(),
                };
            }
            layer_info_file_stream.write_bytes(&(layer.layer_record_data()?))?;
        }

        // Layer images.
        for layer in self.layers.iter_mut() {
            layer_info_file_stream.write_bytes(&(layer.encoded_image()?))?;
        }

        // Write the layer info to the layer and mask info file stream.
        let mut layer_info_data = layer_info_file_stream.data().to_vec();
        data::pad(&mut layer_info_data, 2);
        layer_and_mask_info_file_stream.write_be(&(layer_info_data.len() as u32))?;
        layer_and_mask_info_file_stream.write_bytes(&layer_info_data)?;

        // The global layer mask info.
        layer_and_mask_info_file_stream.write_be(&0u32)?;

        // Write the layer info to the global file stream.
        file_stream.write_be(&(layer_and_mask_info_file_stream.data().len() as u32))?;
        file_stream.write_bytes(layer_and_mask_info_file_stream.data())?;

        // IMAGE DATA SECTION
        // A flattened preview image.
        if let Some(preview_image) = &self.preview_image {
            // let preview_image_data = preview_image
        }

        Ok(file_stream.data().to_vec())
    }
}

// MARK: Layer container metods

impl LayerContainer for Document {
    fn layers(&self) -> Vec<&Layer> {
        self.layers.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use graphics::Color;

    use super::*;

    #[test]
    fn file_data() {
        let mut document = Document::new();
        document.size = Size {
            width: 32,
            height: 16,
        };

        let layer_0_bounds = Rect::new(2, 1, 14, 17);
        let mut layer_0 = Layer::new(layer_0_bounds);
        layer_0.name = Some("Yellow".to_string());
        let yellow_image = Image::color(&Color::YELLOW, layer_0_bounds.size.into());
        layer_0.image = Some(yellow_image);

        document.layers = vec![layer_0];

        let data = document.file_data().unwrap();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/yellow.psd");
        let expected_data = std::fs::read(path).unwrap();

        std::fs::write("/tmp/yellow.psd", &data).unwrap();
        assert_eq!(data, expected_data);
    }
}
