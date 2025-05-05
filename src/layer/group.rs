use std::mem;

use file_stream::write::FileStreamWriter;
use graphics::Rect;

use crate::color_channel::{ColorChannel, ColorChannelType};
use crate::document;
use crate::layer_container::LayerContainer;

use super::divider_type::DividerType;
use super::Layer;

mod constants;

/// Information for the group.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupInfo {
    /// The child layers.
    pub layers: Vec<Layer>,
}

// MARK: Creation

impl GroupInfo {
    /// Creates new a new group info structure.
    pub fn new(layers: Vec<Layer>) -> Self {
        Self { layers }
    }
}

impl LayerContainer for GroupInfo {
    fn layers(&self) -> Vec<&Layer> {
        self.layers.iter().collect()
    }
}

impl Layer {
    /// Creates a group marker layer.
    pub fn group_marker() -> anyhow::Result<Self> {
        let mut layer = Layer::new(Rect::zero());
        let empty_alpha_channel = ColorChannel::new(ColorChannelType::Alpha, 0);
        let empty_red_channel = ColorChannel::new(ColorChannelType::Red, 0);
        let empty_green_channel = ColorChannel::new(ColorChannelType::Green, 0);
        let empty_blue_channel = ColorChannel::new(ColorChannelType::Blue, 0);

        layer.channels = vec![
            empty_alpha_channel,
            empty_red_channel,
            empty_green_channel,
            empty_blue_channel,
        ];
        layer.name = Some("</Layer group>".to_string());

        let mut additional_info_file_stream = FileStreamWriter::new();
        additional_info_file_stream.write_bytes(&document::constants::RESOURCE_SIGNATURE)?;
        additional_info_file_stream.write_bytes(&constants::SECTION_DIVIDER_KEY)?;
        additional_info_file_stream.write_be(&(mem::size_of::<u32>() as u32))?;
        let divider_type = DividerType::SectionDivider;
        additional_info_file_stream.write_be(&(divider_type as u32))?;

        layer.additional_layer_information = Some(additional_info_file_stream.data().to_vec());

        Ok(layer)
    }
}

#[cfg(test)]
mod test {
    use crate::Layer;

    #[test]
    fn group_marker_record_data() {
        let expected_data = vec![
            0x00, 0x00, 0x00, 0x00, // Top
            0x00, 0x00, 0x00, 0x00, // Left
            0x00, 0x00, 0x00, 0x00, // Bottom
            0x00, 0x00, 0x00, 0x00, // Right
            0x00, 0x04, // Number of channels
            0xff, 0xff, // Alpha channel
            0x00, 0x00, 0x00, 0x02, // Alpha channel length
            0x00, 0x00, // Red channel
            0x00, 0x00, 0x00, 0x02, // Red channel length
            0x00, 0x01, // Green channel
            0x00, 0x00, 0x00, 0x02, // Green channel length
            0x00, 0x02, // Blue channel
            0x00, 0x00, 0x00, 0x02, // Blue channel length
            0x38, 0x42, 0x49, 0x4d, // Resource signature (8BIM)
            0x6e, 0x6f, 0x72, 0x6d, // Blend mode
            0xff, // Opacity
            0x00, // Clipping
            0x00, // Flags (visibility)
            0x00, // Filler
            0x00, 0x00, 0x00, 0x54, // Length of extra data
            0x00, 0x00, 0x00, 0x00, // Mask data
            0x00, 0x00, 0x00, 0x00, // Blending ranges
            0x0e, // Name length
            0x3c, 0x2f, 0x4c, 0x61, 0x79, 0x65, 0x72, 0x20, 0x67, 0x72, 0x6f, 0x75, 0x70, 0x3e,
            0x00, // Name "<\Layer group>" plus padding
            0x38, 0x42, 0x49, 0x4d, 0x6c, 0x75, 0x6e, 0x69, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00,
            0x00, 0x0e, 0x00, 0x3c, 0x00, 0x2f, 0x00, 0x4c, 0x00, 0x61, 0x00, 0x79, 0x00, 0x65,
            0x00, 0x72, 0x00, 0x20, 0x00, 0x67, 0x00, 0x72, 0x00, 0x6f, 0x00, 0x75, 0x00, 0x70,
            0x00, 0x3e, // Unicode name
            0x38, 0x42, 0x49, 0x4d, // Resource signature (8BIM)
            0x6c, 0x73, 0x63, 0x74, // Section divider key (lsct)
            0x00, 0x00, 0x00, 0x04, // Size of section divider
            0x00, 0x00, 0x00, 0x03, // Section divider type (end marker type)
        ];

        let mut marker = Layer::group_marker().unwrap();
        let result = marker.record_data().unwrap();
        // let result = [UInt8](try marker.layerRecordData(context: self.renderContext))
        // XCTAssertEqual(Data(result), expectedData)

        assert_eq!(result[0..=3], [0x00, 0x00, 0x00, 0x00]); // Top
        assert_eq!(result[4..=7], [0x00, 0x00, 0x00, 0x00]); // Left
        assert_eq!(result[8..=11], [0x00, 0x00, 0x00, 0x00]); // Bottom
        assert_eq!(result[12..=15], [0x00, 0x00, 0x00, 0x00]); // Right

        assert_eq!(result[16..=17], [0x00, 0x04]); // Number of channels

        assert_eq!(result[18..=19], [0xff, 0xff]); // Alpha channel
        assert_eq!(result[20..=23], [0x00, 0x00, 0x00, 0x02]); // Alpha channel length

        assert_eq!(result[24..=25], [0x00, 0x00]); // Red channel
        assert_eq!(result[26..=29], [0x00, 0x00, 0x00, 0x02]); // Red channel length

        // I started comparing each byte when this broke, but not all bytes are compared individually.
        // It might be good to keep going at some point to make it easier to diagnose when something breaks.

        assert_eq!(result, expected_data);
    }
}
