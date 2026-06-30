use std::f32::NEG_INFINITY;
use std::mem;
use std::path::Path;

use file_stream::read::FileStreamReader;
use file_stream::write::FileStreamWriter;
use graphics::image::ImageConstraints;
use graphics::{Image, Point, Rect, Size};

use crate::color_mode::ColorMode;
use crate::divider_type::DividerType;
use crate::error::ReadError;
use crate::image_compression::ImageCompression;
use crate::layer::Layer;
use crate::layer_container::LayerContainer;
use crate::{LayerType, data, image};

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
    /// Creates an empty Photoshop document.
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

    /// Open a Photoshop document from disk.
    pub fn open<P: AsRef<Path>>(file_path: P) -> anyhow::Result<Self> {
        Self::open_with_constraints(file_path, None, None)
    }

    /// Open a Photoshop document from disk, with the option to apply
    /// constraints on the maximum image size and number of layers.
    pub fn open_with_constraints<P: AsRef<Path>>(
        file_path: P,
        size_constraints: Option<ImageConstraints>,
        maximum_layers: Option<u32>,
    ) -> anyhow::Result<Self> {
        let mut file_stream = FileStreamReader::open(file_path)?;

        let mut output = Document::new();

        //
        // HEADER SECTION
        //

        // Check that the file signature is correct.
        if file_stream.read_bytes(4)? != constants::FILE_SIGNATURE {
            anyhow::bail!(ReadError::InvalidFileSignature);
        };

        // Check the version number
        if file_stream.read_be::<i16>()? != constants::VERSION_NUMBER {
            anyhow::bail!(ReadError::UnsupportedVersionNumber)
        }

        // Next 6 bytes are reserved.
        file_stream.skip_bytes(6)?;

        // Parse channels
        output.number_of_channels = file_stream.read_be()?;

        // Parse the size.
        output.size.height = file_stream.read_be()?;
        output.size.width = file_stream.read_be()?;

        if let Some(max_size) = size_constraints
            .as_ref()
            .and_then(|constraints| constraints.maximum_size)
        {
            if output.size.width > max_size.width || output.size.height > max_size.height {
                anyhow::bail!(ReadError::MaximumSizeExceeded)
            }
        }

        if let Some(max_resolution) = size_constraints
            .as_ref()
            .and_then(|constraints| constraints.maximum_resolution)
        {
            let resolution = output.size.width * output.size.height;
            if resolution > max_resolution {
                anyhow::bail!(ReadError::MaximumSizeExceeded)
            }
        }

        // Parse colour depth information.
        output.bits_per_channel = file_stream.read_be()?;

        if let Some(parsed_color_mode) = ColorMode::from_value(file_stream.read_be()?) {
            output.color_mode = parsed_color_mode;
        }

        // TODO: Throw an error if this colour mode isn’t supported.

        //
        // COLOUR MODE DATA SECTION
        //

        let color_mode_data_length: u32 = file_stream.read_be()?;
        // Right now, we’re not supporting indexed colours,
        // so just skip however many bytes we need to.
        file_stream.skip_bytes(color_mode_data_length as usize)?;

        //
        // IMAGE RESOURCES SECTION
        //

        let image_resources_section_length: u32 = file_stream.read_be()?;
        // What is this section about? Not sure. Let’s skip it!
        file_stream.skip_bytes(image_resources_section_length as usize)?;

        //
        // LAYER AND MASK INFORMATION SECTION
        //

        let layers_section_length: u32 = file_stream.read_be()?;

        if layers_section_length == 0 {
            anyhow::bail!(ReadError::NoLayerInformation)
        }
        let layers_info_length: u32 = file_stream.read_be()?;

        if layers_info_length > 0 {
            // The number of layers might be negative according to the documentation.
            let number_of_layers = (file_stream.read_be::<i16>()?).abs();

            // Check that the maximum number of layers hasn’t been exceeded.
            if let Some(max_layers) = maximum_layers {
                if number_of_layers as u32 > max_layers {
                    anyhow::bail!(ReadError::MaximumNumberOfLayersExceeded)
                }
            }

            // The data is structured so that all of the layer info
            // is grouped together.
            for _ in 0..number_of_layers {
                let layer = Layer::from_file_stream(&mut file_stream)?;
                output.layers.push(layer);
            }

            // The layer images are grouped together after
            // the layer info for all of the layers.
            for layer in output.layers.iter_mut() {
                layer.parse_image(&mut file_stream)?;
            }
        }

        output.create_groups();

        Ok(output)
    }

    /// Creates groups from the layers based on the divider types.
    fn create_groups(&mut self) {
        let mut index = 0;
        while index < self.layers.len() {
            let layer = &self.layers[index];
            if layer.divider_type == DividerType::SectionDivider {
                // Remove the divider.
                self.layers.remove(index);
                let group = create_group(&mut self.layers, &mut index, self.size);
                self.layers.insert(index, group);
            }
            index += 1;
        }
    }
}

/// Creates a group for the layers at a given layer index.
/// This index should be that of the group divider.
fn create_group(layers: &mut Vec<Layer>, index: &mut usize, container_size: Size<u32>) -> Layer {
    let mut child_layers = Vec::new();

    // Get the next layer.
    let mut next_layer = layers.remove(*index);
    while next_layer.divider_type != DividerType::OpenFolder
        && next_layer.divider_type != DividerType::ClosedFolder
    {
        // A group inside a group.
        if next_layer.divider_type == DividerType::SectionDivider {
            let group = create_group(layers, index, container_size);
            child_layers.push(group);
        }
        // A normal layer.
        else {
            child_layers.push(next_layer.clone());
        }
        next_layer = layers.remove(*index);
    }

    let is_open = next_layer.divider_type == DividerType::OpenFolder;
    let mut group = Layer::group(child_layers, is_open, container_size);
    group.is_hidden = next_layer.is_hidden;
    group.name = next_layer.name;
    group.blend_mode = next_layer.blend_mode;
    group.bounds = next_layer.bounds;
    group.channels = next_layer.channels;

    group
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
        image_resources_file_stream.write_be(&layers_group_identifiers_size)?;
        // image_resources_file_stream.write_be(&255u8)?;
        // For each layer (including groups), output the group ID.
        for _ in 0..self.number_of_layers() {
            image_resources_file_stream.write_be(&0i16)?;
        }

        // Write the images resources section.
        file_stream.write_be(&(image_resources_file_stream.data().len() as u32))?;
        file_stream.write_bytes(&image_resources_file_stream.data())?;

        // LAYER AND MASK INFORMATION SECTION
        let mut layer_and_mask_info_file_stream = FileStreamWriter::new();

        let mut layer_info_file_stream = FileStreamWriter::new();
        layer_info_file_stream.write_be(&((self.number_of_layers() as i16) * -1))?;

        // Layer records.
        for layer in self.layers.iter_mut() {
            // Procreate can’t handle zero width and height.
            if layer.bounds == Rect::zero() && layer.layer_type != LayerType::GroupMarker {
                layer.bounds = Rect {
                    origin: Point::zero(),
                    size: self.size.into(),
                };
            }
            layer_info_file_stream.write_bytes(&(layer.record_data()?))?;
        }

        // layer_info_file_stream.write_bytes(&[0xd0, 0x0d, 0xad])?;

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
            let preview_image_data = image::psd_data(preview_image, &ImageCompression::Rle)?;
            file_stream.write_bytes(&preview_image_data)?;
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
mod import_tests {
    use crate::{Document, LayerType};

    #[test]
    fn small() {
        let document = Document::open("tests/resources/small.psd").unwrap();
        assert_eq!(
            document.size,
            graphics::Size {
                width: 2,
                height: 1
            }
        );

        assert_eq!(document.layers.len(), 1);

        let layer = document.layers.first().unwrap();
        assert_eq!(layer.name, Some("Layer 1".to_string()));
        assert_eq!(layer.opacity, u8::MAX);
        assert_eq!(layer.is_hidden, false);
        let layer_image = layer.image.as_ref().unwrap();
        assert_eq!(
            layer_image.size,
            graphics::Size {
                width: 2,
                height: 1
            }
        );
        assert_eq!(
            layer_image.pixel_color(graphics::Point { x: 0, y: 0 }),
            Some(graphics::Color::from_rgb_u32(0xd04648))
        );
    }

    #[test]
    fn simple_with_folders() {
        let document = Document::open("tests/resources/simple-with-folders.psd").unwrap();

        assert_eq!(document.layers.len(), 3);

        let group_0 = &document.layers[0];
        assert!(match group_0.layer_type {
            LayerType::Group(_) => true,
            _ => false,
        });
        let layers = match group_0.layer_type.clone() {
            LayerType::Group(group_info) => group_info.layers,
            _ => Vec::new(),
        };
        assert_eq!(group_0.name, Some("Frame 1".to_string()));
        assert_eq!(layers[0].name, Some("Layer 1".to_string()));
        assert_eq!(layers[1].name, Some("Layer 2".to_string()));

        let group_1 = &document.layers[1];
        assert!(match group_1.layer_type {
            LayerType::Group(_) => true,
            _ => false,
        });
        let layers = match group_1.layer_type.clone() {
            LayerType::Group(group_info) => group_info.layers,
            _ => Vec::new(),
        };
        assert_eq!(group_1.name, Some("Frame 2".to_string()));
        assert_eq!(layers[0].name, Some("Layer 1".to_string()));
    }

    #[test]
    fn simple_with_nested_folders() {
        let document = Document::open("tests/resources/foldered.psd").unwrap();

        assert_eq!(document.layers.len(), 1);

        let group_0 = &document.layers[0];
        assert!(match group_0.layer_type {
            LayerType::Group(_) => true,
            _ => false,
        });
        let layers = match group_0.layer_type.clone() {
            LayerType::Group(group_info) => group_info.layers,
            _ => Vec::new(),
        };
        assert_eq!(group_0.name, Some("Grouping".to_string()));
        assert_eq!(layers[0].name, Some("Background".to_string()));
        assert_eq!(layers[1].name, Some("Masterdon".to_string()));

        let group_1 = layers[1].clone();
        let inner_layers = match group_1.layer_type.clone() {
            LayerType::Group(group_info) => group_info.layers,
            _ => Vec::new(),
        };
        assert_eq!(group_1.name, Some("Masterdon".to_string()));
        assert_eq!(inner_layers[0].name, Some("Rosalina".to_string()));
        assert_eq!(inner_layers[1].name, Some("Pedro".to_string()));
    }
}

#[cfg(test)]
mod export_tests {
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
        layer_0.image = Some(yellow_image.clone());

        document.preview_image = Some(yellow_image);

        document.layers = vec![layer_0];

        let data = document.file_data().unwrap();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/yellow.psd");
        let expected_data = std::fs::read(path).unwrap();

        // std::fs::write("/tmp/yellow.psd", &data).unwrap();
        assert_eq!(data, expected_data);
    }

    #[test]
    fn file_data_2x1() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/2x1.png");
        let image = Image::open(&path).unwrap();

        let mut document = Document::new();
        document.size = image.size;
        document.preview_image = Some(image.clone());

        let layer_0_bounds = Rect {
            origin: Point::zero(),
            size: image.size.into(),
        };
        let mut layer_0 = Layer::new(layer_0_bounds);
        layer_0.name = Some("L1".to_string());
        layer_0.image = Some(image);

        document.layers = vec![layer_0];

        let data = document.file_data().unwrap();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/2x1.psd");
        let expected_data = std::fs::read(path).unwrap();

        // std::fs::write("/tmp/2x1.psd", &data).unwrap();

        assert_eq!(data, expected_data);
    }

    #[test]
    fn file_data_simple() {
        let image = Image::color(
            &Color::CYAN,
            Size {
                width: 2,
                height: 2,
            },
        );

        let mut document = Document::new();
        document.size = image.size;

        let bounds = Rect {
            origin: Point::zero(),
            size: image.size.into(),
        };
        let mut layer_0 = Layer::new(bounds);
        layer_0.name = Some("Background".to_string());
        layer_0.image = Some(image.clone());

        let mut layer_1 = Layer::new(bounds);
        layer_1.name = Some("Empty".to_string());

        document.layers = vec![layer_0, layer_1];
        document.preview_image = Some(image.clone());

        let data = document.file_data().unwrap();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/simple.psd");
        let expected_data = std::fs::read(path).unwrap();

        std::fs::write("/tmp/simple.psd", &data).unwrap();
        // Header
        assert_eq!(data[0..30], expected_data[0..30]);
        // Image resources
        assert_eq!(data[30..92], expected_data[30..92]);
        // Layer and mask info length
        assert_eq!(data[92..96], expected_data[92..96]);
        // Layer info length
        assert_eq!(data[96..100], expected_data[96..100]);

        // And the rest…
        assert_eq!(data, expected_data);
    }

    #[test]
    fn file_data_with_group() {
        let image = Image::color(
            &Color::MAGENTA,
            Size {
                width: 2,
                height: 2,
            },
        );

        let mut document = Document::new();
        document.size = image.size;

        let bounds = Rect {
            origin: Point::zero(),
            size: image.size.into(),
        };
        let mut layer_0 = Layer::new(bounds);
        layer_0.name = Some("Background".to_string());
        layer_0.image = Some(image.clone());

        // let mut layer_1 = Layer::new(bounds);
        // layer_1.name = Some("Empty".to_string());

        let mut group = Layer::group(vec![layer_0], true, document.size);
        group.name = Some("Groupella".to_string());

        document.layers = vec![group];
        document.preview_image = Some(image.clone());

        let data = document.file_data().unwrap();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/simple-with-group.psd");
        let expected_data = std::fs::read(path).unwrap();

        // std::fs::write("/tmp/simple-with-group-rs.psd", &data).unwrap();

        assert_eq!(data, expected_data);
    }
}
