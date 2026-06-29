use std::mem;

use file_stream::read::{FileStreamReader, Readable};
use file_stream::write::FileStreamWriter;
use graphics::{Image, Point, Rect, Size};

use crate::blend_mode::BlendMode;
use crate::color_channel::{ColorChannel, ColorChannelType};
use crate::document;
use crate::error::ReadError;
use crate::image_compression::ImageCompression;
use crate::string::{self, pascal};
use crate::{constants, data};

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
    /// The opacity of the layer (from 0 to 255).
    pub opacity: u8,
    /// Whether or not the layer is hidden.
    pub is_hidden: bool,
    /// The layer’s name.
    pub name: Option<String>,
    /// The layer image.
    pub image: Option<Image>,
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
    /// A group marker.
    GroupMarker,
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
            divider_type: DividerType::Other,
        }
    }

    /// Creates a new Photoshop group layer.
    pub fn group(child_layers: Vec<Layer>, is_open: bool, size: Size<u32>) -> Self {
        let divider_type = if is_open {
            DividerType::OpenFolder
        } else {
            DividerType::ClosedFolder
        };

        let empty_alpha_channel = ColorChannel::new(ColorChannelType::Alpha, 0);
        let empty_red_channel = ColorChannel::new(ColorChannelType::Red, 0);
        let empty_green_channel = ColorChannel::new(ColorChannelType::Green, 0);
        let empty_blue_channel = ColorChannel::new(ColorChannelType::Blue, 0);

        let channels = vec![
            empty_alpha_channel,
            empty_red_channel,
            empty_green_channel,
            empty_blue_channel,
        ];

        Self {
            layer_type: LayerType::Group(GroupInfo::new(child_layers)),
            bounds: Rect {
                origin: Point::zero(),
                size: size.into(),
            },
            number_of_channels: 4,
            channels,
            blend_mode: BlendMode::PassThrough,
            opacity: u8::MAX,
            is_hidden: false,
            name: None,
            image: None,
            divider_type,
        }
    }

    /// Creates a new Photoshop document layer from a file stream.
    /// The file stream must have the location set to the correct
    /// place. The file stream is passed by reference, so the
    /// location will be updated when we’re done creating the document.
    pub fn from_file_stream(file_stream: &mut FileStreamReader) -> anyhow::Result<Self> {
        let top = file_stream.read_be()?;
        let left = file_stream.read_be()?;
        let bottom: i32 = file_stream.read_be()?;
        let right: i32 = file_stream.read_be()?;

        let bounds = Rect::new(left, top, right - left, bottom - top);

        let mut output = Self::new(bounds);

        output.number_of_channels = file_stream.read_be()?;

        // Get the information about each channel.
        for _ in 0..output.number_of_channels {
            let channel_information = file_stream.read_be()?;
            let channel_length: u32 = file_stream.read_be()?;

            let channel_type = ColorChannelType::from_value(channel_information);
            let channel = ColorChannel::new(channel_type, channel_length as usize);
            output.channels.push(channel);
        }

        let blend_mode_signature = file_stream.read_bytes(4)?;

        if blend_mode_signature != constants::RESOURCE_SIGNATURE {
            anyhow::bail!(ReadError::InvalidResourceSignature);
        }

        let blend_mode_string = file_stream.read_string(4)?;
        output.blend_mode = BlendMode::from(blend_mode_string.as_str());

        output.opacity = file_stream.read_be()?;

        // Don’t know what clipping is.
        file_stream.skip_bytes(1)?;

        let flags: u8 = file_stream.read_be()?;

        // If the second bit is set, then the layer is hidden (despite
        // the documentation implying the opposite).
        output.is_hidden = (flags & 0b0000_0010) == 0b0000_0010;

        // Filler
        file_stream.skip_bytes(1)?;

        let extra_data_length: u32 = file_stream.read_be()?;

        // Skip layer mask data
        let layer_mask_data_length: u32 = file_stream.read_be()?;
        file_stream.skip_bytes(layer_mask_data_length as usize)?;

        // Skip blending ranges data
        let layer_blending_ranges_data_length: u32 = file_stream.read_be()?;
        file_stream.skip_bytes(layer_blending_ranges_data_length as usize)?;

        // Get the layer name.
        let pascal_string = pascal::string_from_file_stream(file_stream)?;
        output.name = pascal_string.1;

        // Work out how many bytes are left for any other data.
        let remaining_length = extra_data_length as usize
            - std::mem::size_of::<u32>()
            - layer_mask_data_length as usize
            - std::mem::size_of::<u32>()
            - layer_blending_ranges_data_length as usize
            - std::mem::size_of::<u8>()
            - pascal_string.0;

        let additional_data = file_stream.read_bytes(remaining_length)?;

        let mut index = 0;

        while index + 8 < additional_data.len() {
            // Search through the file stream one byte at a time to find the
            // resource signatures. This could probably be optimised.
            let potential_signature = additional_data[index..index + 4].to_vec();
            if potential_signature == constants::RESOURCE_SIGNATURE {
                index += 4;

                let key = additional_data[index..index + 4].to_vec();
                index += 4;

                let length = additional_data[index..index + 4].try_into()?;
                index += 4;
                let length = u32::from_be_bytes(length);
                if key == group::constants::SECTION_DIVIDER_KEY {
                    let value = additional_data[index..index + 4].try_into()?;
                    let value = u32::from_be_bytes(value);
                    output.divider_type = DividerType::from(value);
                }

                index += length as usize;
            } else {
                index += 1;
            }
        }

        Ok(output)
    }
}

// MARK: Updates

impl Layer {
    /// Updates the channel data for the image.
    fn update_channel_data(&mut self) {
        // if let LayerType::Group(_) = self.layer_type {
        //     return;
        // }
        // Procreate can’t handle empty images, so we create a clear
        // image of the size of document.
        if self.image.is_none() && self.bounds != Rect::zero()
        // && self.layer_type == LayerType::Image
        {
            self.image = Some(Image::empty(self.bounds.size.into()));
        }

        let Some(image) = self.image.as_ref() else {
            return;
            // panic!(
            //     "No image for layer {:?}.",
            //     self.name.clone().unwrap_or_default()
            // )
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

        // Convention seems to be to put the alpha channel first.
        self.channels = vec![alpha_channel, red_channel, green_channel, blue_channel];
    }
}

// MARK: Encoding

impl Layer {
    /// Returns the image encoded per channel.
    pub fn encoded_image(&mut self) -> anyhow::Result<Vec<u8>> {
        match self.layer_type.clone() {
            LayerType::Image => self.layer_encoded_image(),
            LayerType::Group(mut group_info) => self.group_encoded_image(&mut group_info),
            LayerType::GroupMarker => self.layer_encoded_image(),
        }
    }

    /// Returns the image encoded per channel for a layer.
    fn layer_encoded_image(&mut self) -> anyhow::Result<Vec<u8>> {
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

    /// Returns the encoded image data for a group marker.
    fn group_encoded_image(&mut self, group_info: &mut GroupInfo) -> anyhow::Result<Vec<u8>> {
        let mut file_stream = FileStreamWriter::new();

        // Write zero for each channel.
        for _ in 0..self.number_of_channels {
            file_stream.write_be(&0i16)?;
        }

        // Write layers in the group in here.
        for layer in group_info.layers.iter_mut() {
            file_stream.write_bytes(&layer.encoded_image()?)?;
        }

        // End of folder.
        for _ in 0..self.number_of_channels {
            file_stream.write_be(&0i16)?;
        }

        Ok(file_stream.data().to_vec())
    }

    /// Returns the data for the layer, which may represent a group.
    pub fn record_data(&mut self) -> anyhow::Result<Vec<u8>> {
        match self.layer_type.clone() {
            LayerType::Image => self.layer_record_data(),
            LayerType::Group(mut group_info) => self.group_record_data(&mut group_info),
            LayerType::GroupMarker => self.layer_record_data(),
        }
    }

    /// Returns the data for the layer record.
    fn layer_record_data(&mut self) -> anyhow::Result<Vec<u8>> {
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

            if self.layer_type != LayerType::Image {
                file_stream.write_be(&(mem::size_of::<i16>() as u32))?;
                continue;
            }

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

        if let Some(layer_information) = &self.additional_layer_information()? {
            extra_data_file_stream.write_bytes(layer_information)?;
        }

        file_stream.write_be(&(extra_data_file_stream.data().len() as u32))?;
        file_stream.write_bytes(&extra_data_file_stream.data())?;

        Ok(file_stream.data().to_vec())
    }

    /// Returns record data for a group.
    fn group_record_data(&mut self, group_info: &mut GroupInfo) -> anyhow::Result<Vec<u8>> {
        let mut group_marker = Layer::group_marker()?;
        let mut data = group_marker.layer_record_data()?;

        // Write layers in the group in here.
        for layer in group_info.layers.iter_mut() {
            // Procreate can’t handle zero width and height.
            if layer.bounds == Rect::zero() && layer.layer_type != LayerType::GroupMarker {
                layer.bounds = self.bounds;
            }
            let mut record_data = layer.record_data()?;
            data.append(&mut record_data);
        }

        let mut layer_record_data = self.layer_record_data()?;
        data.append(&mut layer_record_data);
        Ok(data)
    }

    /// Returns the data for the additional layer information.
    fn additional_layer_information(&self) -> anyhow::Result<Option<Vec<u8>>> {
        match self.layer_type {
            LayerType::Image => Ok(None),
            LayerType::Group(_) => {
                let mut additional_info_file_stream = FileStreamWriter::new();
                additional_info_file_stream
                    .write_bytes(&document::constants::RESOURCE_SIGNATURE)?;
                additional_info_file_stream.write_bytes(&group::constants::SECTION_DIVIDER_KEY)?;
                additional_info_file_stream.write_be(&12u32)?;
                additional_info_file_stream.write_be(&(self.divider_type.clone() as u32))?;
                additional_info_file_stream
                    .write_bytes(&document::constants::RESOURCE_SIGNATURE)?;
                additional_info_file_stream.write_bytes(&self.blend_mode.as_str().as_bytes())?;
                Ok(Some(additional_info_file_stream.data().to_vec()))
            }
            LayerType::GroupMarker => {
                let mut additional_info_file_stream = FileStreamWriter::new();
                additional_info_file_stream
                    .write_bytes(&document::constants::RESOURCE_SIGNATURE)?;
                additional_info_file_stream.write_bytes(&group::constants::SECTION_DIVIDER_KEY)?;
                additional_info_file_stream.write_be(&(mem::size_of::<u32>() as u32))?;
                let divider_type = DividerType::SectionDivider;
                additional_info_file_stream.write_be(&(divider_type as u32))?;
                Ok(Some(additional_info_file_stream.data().to_vec()))
            }
        }
    }
}

// MARK: Decoding

impl Layer {
    /// Parses the image from the file stream and sets it
    /// as the image for the layer if parsing is successful.
    fn parse_image(
        &mut self,
        file_stream: &mut FileStreamReader,
        image_compression: Option<&ImageCompression>,
    ) -> anyhow::Result<()> {
        let mut output_image_bytes =
            vec![0u8; self.bounds.size.width as usize * self.bounds.size.height as usize * 4];

        for channel in self.channels.iter() {
            if channel.data_length <= 2 {
                file_stream.skip_bytes(channel.data_length)?;
                continue;
            }

            let is_supported_type = channel.color_type == ColorChannelType::Alpha
                || channel.color_type == ColorChannelType::Red
                || channel.color_type == ColorChannelType::Green
                || channel.color_type == ColorChannelType::Blue;

            if !is_supported_type {
                file_stream.skip_bytes(channel.data_length)?;
                continue;
            }

            let image_compression = ImageCompression::from_value(file_stream.read_be()?).unwrap();
        }
        // for channel in self.channels {

        //     let imageCompression = ImageCompression(rawValue: fileStream.readInt16())

        //     if imageCompression == .rawData {
        //         let size = Int(self.bounds.width * self.bounds.height)
        //         channel.data = fileStream.readData(size)
        //     } else if imageCompression == .rle {
        //         // Read in all the line lengths.
        //         var lineLengths: [UInt16] = []
        //         for _ in 0 ..< height {
        //             lineLengths.append(fileStream.readUInt16())
        //         }

        //         var outputData = Data()

        //         for lineLength in lineLengths {
        //             let encodedData = fileStream.readData(Int(lineLength))

        //             guard encodedData.count > 0 else {
        //                 break
        //             }

        //             let subdata = encodedData[0 ..< lineLength]
        //             let decodedData = subdata.rleDecoded()
        //             outputData.append(decodedData)
        //         }

        //         channel.data = outputData
        //     }
        // }

        // guard let alphaChannel = self.channels.filter({ $0.type == .alpha }).first,
        //       let redChannel = self.channels.filter({ $0.type == .red }).first,
        //       let greenChannel = self.channels.filter({ $0.type == .green }).first,
        //       let blueChannel = self.channels.filter({ $0.type == .blue }).first
        // // TODO: Throw an error here.
        // else { return }

        // for yPosition in 0 ..< height {
        //     for xPosition in 0 ..< width {
        //         let byteIndex = (yPosition * width) + xPosition

        //         // Default the alpha channel to be max,
        //         // and all the colour channels to 0.
        //         var alpha: UInt8 = .max
        //         if alphaChannel.data.count > byteIndex {
        //             alpha = alphaChannel.data[byteIndex]
        //         }
        //         var red: UInt8 = .min
        //         if redChannel.data.count > byteIndex {
        //             red = redChannel.data[byteIndex]
        //         }
        //         var green: UInt8 = .min
        //         if greenChannel.data.count > byteIndex {
        //             green = greenChannel.data[byteIndex]
        //         }
        //         var blue: UInt8 = .min
        //         if blueChannel.data.count > byteIndex {
        //             blue = blueChannel.data[byteIndex]
        //         }

        //         let outputByteIndex = byteIndex * 4
        //         outputImageBytes[outputByteIndex] = red
        //         outputImageBytes[outputByteIndex + 1] = green
        //         outputImageBytes[outputByteIndex + 2] = blue
        //         outputImageBytes[outputByteIndex + 3] = alpha
        //     }
        // }

        // // try? testStream.data.write(to: URL(fileURLWithPath: "/tmp/*trees.data"))

        // if outputImageBytes.isEmpty || self.bounds.size == .zero {
        //     self.image = nil
        // } else {
        //     self.image = Image(unpremultipliedBitmapData: outputImageBytes, bytesPerRow: width * 4, size: self.bounds.size, context: context)
        // }

        Ok(())
    }
}

#[cfg(test)]
mod encode_tests {
    use std::path::PathBuf;

    use file_stream::read::FileStreamReader;
    use graphics::{Color, Point, Size};

    use super::*;

    #[test]
    fn update_channel_data() {
        let bounds = Rect::new(0, 0, 2, 2);
        let mut layer = Layer::new(bounds);
        layer.image = Some(Image::color(
            &Color {
                red: 0xab,
                green: 0xcd,
                blue: 0xef,
                alpha: 0x91,
            },
            bounds.size.into(),
        ));

        assert_eq!(layer.channels.len(), 0);

        layer.update_channel_data();

        assert_eq!(layer.channels.len(), 4);

        let red_channel = &layer.channels[1];
        assert_eq!(red_channel.color_type, ColorChannelType::Red);
        assert_eq!(red_channel.data, vec![0xab, 0xab, 0xab, 0xab]);

        let green_channel = &layer.channels[2];
        assert_eq!(green_channel.color_type, ColorChannelType::Green);
        assert_eq!(green_channel.data, vec![0xcd, 0xcd, 0xcd, 0xcd]);

        let blue_channel = &layer.channels[3];
        assert_eq!(blue_channel.color_type, ColorChannelType::Blue);
        assert_eq!(blue_channel.data, vec![0xef, 0xef, 0xef, 0xef]);

        let alpha_channel = &layer.channels[0];
        assert_eq!(alpha_channel.color_type, ColorChannelType::Alpha);
        assert_eq!(alpha_channel.data, vec![0x91, 0x91, 0x91, 0x91]);
    }

    #[test]
    fn encoded_image_2x2() {
        let bounds = Rect::new(0, 0, 2, 2);
        let mut layer = Layer::new(bounds);
        let image = Image::color(&Color::from_rgb_u32(0x50d1e7), bounds.size.into());
        layer.image = Some(image);

        let encoded_image = layer.encoded_image().unwrap();

        // Both Acorn and Pixelmator produce exactly this data, so it
        // can be trusted.

        // Alpha compression (RLE).
        assert_eq!(encoded_image[0..2], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[2..4], [0x00, 0x03]);
        // Second row size.
        assert_eq!(encoded_image[4..6], [0x00, 0x03]);
        // Two byes of 0xFF in the top row.
        assert_eq!(encoded_image[6..9], [0x01, 0xff, 0xff]);
        // Two byes of 0xFF in the bottom row.
        assert_eq!(encoded_image[9..12], [0x01, 0xff, 0xff]);

        // Red compression (RLE).
        assert_eq!(encoded_image[12..14], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[14..16], [0x00, 0x03]);
        // Second row size.
        assert_eq!(encoded_image[16..18], [0x00, 0x03]);
        // Two byes of 0x50 in the top row.
        assert_eq!(encoded_image[18..21], [0x01, 0x50, 0x50]);
        // Two byes of 0x50 in the bottom row.
        assert_eq!(encoded_image[21..24], [0x01, 0x50, 0x50]);

        // Green compression (RLE).
        assert_eq!(encoded_image[24..26], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[26..28], [0x00, 0x03]);
        // Second row size.
        assert_eq!(encoded_image[28..30], [0x00, 0x03]);
        // Two byes of 0xd1 in the top row.
        assert_eq!(encoded_image[30..33], [0x01, 0xd1, 0xd1]);
        // Two byes of 0xd1 in the bottom row.
        assert_eq!(encoded_image[33..36], [0x01, 0xd1, 0xd1]);

        // Blue compression (RLE).
        assert_eq!(encoded_image[36..38], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[38..40], [0x00, 0x03]);
        // Second row size.
        assert_eq!(encoded_image[40..42], [0x00, 0x03]);
        // Two byes of 0xe7 in the top row.
        assert_eq!(encoded_image[42..45], [0x01, 0xe7, 0xe7]);
        // Two byes of 0xe7 in the bottom row.
        assert_eq!(encoded_image[45..48], [0x01, 0xe7, 0xe7]);
    }

    #[test]
    fn encoded_image_2x3() {
        let bounds = Rect::new(0, 0, 3, 2);
        let mut layer = Layer::new(bounds);
        let image = Image::color(&Color::from_rgb_u32(0x50d1e7), bounds.size.into());
        layer.image = Some(image);

        let encoded_image = layer.encoded_image().unwrap();

        // Both Acorn and Pixelmator produce exactly this data, so it
        // can be trusted.

        // Alpha compression (RLE).
        assert_eq!(encoded_image[0..2], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[2..4], [0x00, 0x02]);
        // Second row size.
        assert_eq!(encoded_image[4..6], [0x00, 0x02]);
        // Three repeated byes of 0xFF in the top row.
        assert_eq!(encoded_image[6..8], [0xfe, 0xff]);
        // Three repeated byes of 0xFF in the bottom row.
        assert_eq!(encoded_image[8..10], [0xfe, 0xff]);

        // Red compression (RLE).
        assert_eq!(encoded_image[10..12], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[12..14], [0x00, 0x02]);
        // Second row size.
        assert_eq!(encoded_image[14..16], [0x00, 0x02]);
        // Three repeated byes of 0x50 in the top row.
        assert_eq!(encoded_image[16..18], [0xfe, 0x50]);
        // Three repeated byes of 0x50 in the bottom row.
        assert_eq!(encoded_image[18..20], [0xfe, 0x50]);

        // Green compression (RLE).
        assert_eq!(encoded_image[20..22], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[22..24], [0x00, 0x02]);
        // Second row size.
        assert_eq!(encoded_image[24..26], [0x00, 0x02]);
        // Three repeated byes of 0xd1 in the top row.
        assert_eq!(encoded_image[26..28], [0xfe, 0xd1]);
        // Three repeated byes of 0xd1 in the bottom row.
        assert_eq!(encoded_image[28..30], [0xfe, 0xd1]);

        // Blue compression (RLE).
        assert_eq!(encoded_image[30..32], [0x00, 0x01]);
        // First row size.
        assert_eq!(encoded_image[32..34], [0x00, 0x02]);
        // Second row size.
        assert_eq!(encoded_image[34..36], [0x00, 0x02]);
        // Three repeated byes of 0xe7 in the top row.
        assert_eq!(encoded_image[36..38], [0xfe, 0xe7]);
        // Three repeated byes of 0xe7 in the bottom row.
        assert_eq!(encoded_image[38..40], [0xfe, 0xe7]);
    }

    #[test]
    fn encoded_group_image() {
        let image = Image::color(
            &Color::MAGENTA,
            Size {
                width: 2,
                height: 2,
            },
        );

        let bounds = Rect {
            origin: Point::zero(),
            size: image.size.into(),
        };
        let mut layer_0 = Layer::new(bounds);
        layer_0.name = Some("Background".to_string());
        layer_0.image = Some(image.clone());

        let mut group = Layer::group(vec![layer_0], true, image.size);
        group.name = Some("Groupella".to_string());

        let result = group.encoded_image().unwrap();
        // std::fs::write("/tmp/encoded-group-image-rs.data", result).unwrap();
        let expected = std::fs::read("tests/resources/encoded-group-image.data").unwrap();

        assert_eq!(result, expected);
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
        // std::fs::write("/tmp/clouds-encoded-image.data", &result).unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    fn record_data() {
        let bounds = Rect {
            origin: Point::zero(),
            size: Size {
                width: 2,
                height: 2,
            },
        };

        let mut layer = Layer::new(bounds);
        layer.name = Some("Frowning".to_string());
        let image = Image::color(&Color::YELLOW, bounds.size.into());
        layer.image = Some(image);

        let mut result_file_stream =
            FileStreamReader::from_data(layer.record_data().unwrap()).unwrap();

        // Top of bounding box.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x00]
        );
        // Left.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x00]
        );
        // Bottom.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x02]
        );
        // Right.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x02]
        );
        // Number of channels.
        assert_eq!(result_file_stream.read_bytes(2).unwrap(), [0x00, 0x04]);
        // Alpha channel identifier.
        assert_eq!(result_file_stream.read_bytes(2).unwrap(), [0xFF, 0xFF]);
        // Alpha channel length.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x0c]
        );
        // Red channel identifier.
        assert_eq!(result_file_stream.read_bytes(2).unwrap(), [0x00, 0x00]);
        // Red channel length.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x0c]
        );
        // Green channel identifier.
        assert_eq!(result_file_stream.read_bytes(2).unwrap(), [0x00, 0x01]);
        // Green channel length.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x0c]
        );
        // Blue channel identifier.
        assert_eq!(result_file_stream.read_bytes(2).unwrap(), [0x00, 0x02]);
        // Blue channel length.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x0c]
        );
        // Blend mode signature.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x38, 0x42, 0x49, 0x4D]
        );
        // Blend mode.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x6E, 0x6F, 0x72, 0x6D]
        );
        // Opacity.
        assert_eq!(result_file_stream.read_bytes(1).unwrap(), [0xFF]);
        // Clipping.
        result_file_stream.skip_bytes(1).unwrap();
        // Flags (includes visibility)
        assert_eq!(result_file_stream.read_bytes(1).unwrap(), [0x00]);
        // Filler.
        result_file_stream.skip_bytes(1).unwrap();

        // Extra data length.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x34]
        );

        // Mask data.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x00]
        );
        // Blending ranges.
        assert_eq!(
            result_file_stream.read_bytes(4).unwrap(),
            [0x00, 0x00, 0x00, 0x00]
        );

        // Layer name.
        assert_eq!(
            result_file_stream.read_bytes(12).unwrap(),
            [
                0x08, 0x46, 0x72, 0x6F, 0x77, 0x6E, 0x69, 0x6E, 0x67, 0x00, 0x00, 0x00
            ]
        );

        // try? layer.layerRecordData.write(to: URL(fileURLWithPath: "/tmp/*maxston.data"))
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

        let result = layer.record_data().unwrap();

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

        // Channel type (alpha)
        assert_eq!(result[18..=19], [0xff, 0xff]);
        // Channel data length.
        assert_eq!(result[20..=23], [0x00, 0x00, 0x00, 0x12]);

        // Channel type (red)
        assert_eq!(result[24..=25], [0x00, 0x00]);
        // Channel data length.
        assert_eq!(result[26..=29], [0x00, 0x00, 0x00, 0x1b]);

        // Channel type (green)
        assert_eq!(result[30..=31], [0x00, 0x01]);
        // Channel data length.
        assert_eq!(result[32..=35], [0x00, 0x00, 0x00, 0x1b]);

        // Channel type (blue)
        assert_eq!(result[36..=37], [0x00, 0x02]);
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

#[cfg(test)]
mod decode_tests {
    use file_stream::read::FileStreamReader;

    use super::*;

    #[test]
    fn create_with_file_stream() {
        let mut data = Vec::new();

        // Top of bounding box.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x00]);
        // Left.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x00]);
        // Bottom.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x02]);
        // Right.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x02]);
        // Number of channels.
        data.append(&mut vec![0x00, 0x04]);
        // Alpha channel identifier.
        data.append(&mut vec![0xFF, 0xFF]);
        // Alpha channel length.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x06]);
        // Red channel identifier.
        data.append(&mut vec![0x00, 0x00]);
        // Red channel length.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x06]);
        // Green channel identifier.
        data.append(&mut vec![0x00, 0x01]);
        // Green channel length.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x06]);
        // Blue channel identifier.
        data.append(&mut vec![0x00, 0x02]);
        // Blue channel length.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x06]);
        // Blend mode signature.
        data.append(&mut vec![0x38, 0x42, 0x49, 0x4D]);
        // Blend mode.
        data.append(&mut vec![0x6E, 0x6F, 0x72, 0x6D]);
        // Opacity.
        data.append(&mut vec![0xFF]);
        // Clipping.
        data.append(&mut vec![0x00]);
        // Flags (includes visibility)
        data.append(&mut vec![0x00]);
        // Filler.
        data.append(&mut vec![0x00]);

        // Extra data length.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x14]);

        // Mask data.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x00]);
        // Blending ranges.
        data.append(&mut vec![0x00, 0x00, 0x00, 0x00]);

        // Layer name.
        data.append(&mut vec![
            0x08, 0x46, 0x72, 0x6F, 0x77, 0x6E, 0x69, 0x6E, 0x67, 0x00, 0x00, 0x00,
        ]);

        let mut file_stream = FileStreamReader::from_data(data).unwrap();

        let layer = Layer::from_file_stream(&mut file_stream).unwrap();

        assert_eq!(layer.bounds, Rect::new(0, 0, 2, 2));
        assert_eq!(layer.number_of_channels, 4);
        assert_eq!(layer.channels.len(), 4);
        assert_eq!(layer.blend_mode, BlendMode::Normal);
        assert_eq!(layer.opacity, u8::MAX);
        assert_eq!(layer.is_hidden, false);
        assert_eq!(layer.name, Some("Frowning".to_string()));
    }
}
