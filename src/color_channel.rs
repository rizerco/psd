mod channel_type;

pub use channel_type::ColorChannelType;
use file_stream::write::FileStreamWriter;

use crate::{error::WriteError, image_compression::ImageCompression, rle};

/// A colour channel holds the data for one channel of
/// colours for an image.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorChannel {
    /// The length of the data for this channel.
    /// This is at least width * height, but may include some padding,
    /// which needs to be taken into account when parsing the image data.
    pub data_length: usize,
    /// The type of channel.
    pub color_type: ColorChannelType,
    /// The data for the channel.
    pub data: Vec<u8>,
    /// The cached compressed data.
    pub compressed_data: Option<Vec<u8>>,
}

/// The result of calling `compressed_data`.
pub struct CompressedDataResult {
    /// The data.
    pub data: Vec<u8>,
    /// The compression used.
    pub compression: ImageCompression,
}

// MARK: Creation

impl ColorChannel {
    /// Creates a new colour channel with empty data.
    pub fn new(color_type: ColorChannelType, data_length: usize) -> Self {
        Self {
            color_type,
            data_length,
            data: vec![0; data_length],
            compressed_data: None,
        }
    }
}

// MARK: Encoding

impl ColorChannel {
    /// Returns the compressed data using whichever compression method is appropriate.
    /// Also returns the compression method used. Cached.
    pub fn compressed_data<'a>(
        &'a mut self,
        image_height: u32,
    ) -> anyhow::Result<CompressedDataResult> {
        if self.data.len() <= 2 {
            return Ok(CompressedDataResult {
                data: self.data.clone(),
                compression: ImageCompression::RawData,
            });
        } else if let Some(compressed_data) = self.compressed_data.clone() {
            return Ok(CompressedDataResult {
                data: compressed_data,
                compression: ImageCompression::Rle,
            });
        }

        let compressed_data = self.rle_encoded_data(image_height)?;
        self.compressed_data = Some(compressed_data.clone());
        Ok(CompressedDataResult {
            data: compressed_data,
            compression: ImageCompression::Rle,
        })
    }

    /// Returns the channel data encoded with line lengths
    /// for the RLE compression.
    fn rle_encoded_data(&self, image_height: u32) -> anyhow::Result<Vec<u8>> {
        let result = self.rle_encoded_components(image_height)?;
        let mut output = result.line_lengths.clone();
        output.extend(&result.data);
        Ok(output)
    }

    /// Returns the line lengths and image data for the RLE compression of the channel.
    pub fn rle_encoded_components(&self, image_height: u32) -> anyhow::Result<RleComponents> {
        if image_height == 0 {
            anyhow::bail!(WriteError::InvalidImage)
        }

        let bytes_per_row = self.data.len() as u32 / image_height;
        let mut line_lengths = Vec::new();
        let mut encoded_data = Vec::new();

        for y in 0..image_height {
            let start = (y * bytes_per_row) as usize;
            let end = start + bytes_per_row as usize;
            let row_data = &self.data[start..end];
            let mut encoded_row = rle::encoded(row_data);
            line_lengths.push(encoded_row.len());
            encoded_data.append(&mut encoded_row);
        }

        let mut line_lengths_stream = FileStreamWriter::new();
        for line_length in line_lengths {
            line_lengths_stream.write_be(&(line_length as u16))?;
        }

        let result = RleComponents {
            line_lengths: line_lengths_stream.data().to_vec(),
            data: encoded_data,
        };
        Ok(result)
    }
}

/// Represents the components of RLE encoded data.
pub struct RleComponents {
    /// The line lengths for the data.
    pub line_lengths: Vec<u8>,
    /// The data.
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn encoded_data_2x1() {
        let mut channel = ColorChannel::new(ColorChannelType::Red, 2);
        channel.data = vec![0xac, 0x00];

        let result = channel.compressed_data(1).unwrap();

        assert_eq!(result.compression, ImageCompression::RawData);

        assert_eq!(result.data.len(), 2);
        assert_eq!(result.data[0], 0xac);
        assert_eq!(result.data[1], 0x00);
    }

    #[test]
    fn encoded_data_2x2() {
        // Expecting: 00010004 000401fb e5800142 2080

        let mut channel = ColorChannel::new(ColorChannelType::Red, 4);
        channel.data = vec![0xfb, 0xe5, 0x42, 0x20];

        let result = channel.compressed_data(2).unwrap();

        assert_eq!(result.compression, ImageCompression::Rle);

        let data = result.data;

        assert_eq!(data.len(), 10);

        // First line length
        assert_eq!(data[0], 0x00);
        assert_eq!(data[1], 0x03);

        // Second line length
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x03);

        // First line data.
        assert_eq!(data[4], 0x01);
        assert_eq!(data[5], 0xfb);
        assert_eq!(data[6], 0xe5);

        // Second line data.
        assert_eq!(data[7], 0x01);
        assert_eq!(data[8], 0x42);
        assert_eq!(data[9], 0x20);
    }

    #[test]
    fn large_encoded_data() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/clouds-raw-red.data");
        let raw_data = std::fs::read(&path).unwrap();

        // Expected data intentionally differs from ImageMagick output because they will never write 0x81
        // for the repeat.
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/resources/clouds-rle-red.data");
        let expected_data = std::fs::read(&path).unwrap();

        let mut channel = ColorChannel::new(ColorChannelType::Red, raw_data.len());
        channel.data = raw_data;

        let result = channel.compressed_data(4).unwrap();
        assert_eq!(result.compression, ImageCompression::Rle);

        let data = result.data;

        // Line lengths
        assert_eq!(data[0..=1], [0x00, 0x22]);
        assert_eq!(data[2..=3], [0x00, 0x31]);
        assert_eq!(data[4..=5], [0x00, 0x2D]);
        assert_eq!(data[6..=7], [0x00, 0x32]);

        // Top row
        // (-(-5)+1) = 6 bytes of 0x73
        assert_eq!(data[8], 0xFB);
        assert_eq!(data[9], 0x73);

        // (-(-8)+1) = 9 bytes of 0x42
        assert_eq!(data[10], 0xF8);
        assert_eq!(data[11], 0x42);

        // (-(-3)+1) = 4 bytes of 0x24
        assert_eq!(data[12], 0xFD);
        assert_eq!(data[13], 0x24);

        // (-(-3)+1) = 4 bytes of 0x42
        assert_eq!(data[14], 0xFD);
        assert_eq!(data[15], 0x42);

        // (-(-10)+1) = 11 bytes of 0x24
        assert_eq!(data[16], 0xF6);
        assert_eq!(data[17], 0x24);

        // (-(-4)+1) = 5 bytes of 0x42
        assert_eq!(data[18], 0xFC);
        assert_eq!(data[19], 0x42);

        // (-(-2)+1) = 3 bytes of 0x24
        assert_eq!(data[20], 0xFE);
        assert_eq!(data[21], 0x24);

        // (-(-2)+1) = 3 bytes of 0x42
        assert_eq!(data[22], 0xFE);
        assert_eq!(data[23], 0x42);

        // (-(-2)+1) = 3 bytes of 0x24
        assert_eq!(data[24], 0xFE);
        assert_eq!(data[25], 0x24);

        // (-(-5)+1) = 6 bytes of 0x42
        assert_eq!(data[26], 0xFB);
        assert_eq!(data[27], 0x42);

        // (-(-5)+1) = 6 bytes of 0x24
        assert_eq!(data[28], 0xFB);
        assert_eq!(data[29], 0x24);

        // (-(-4)+1) = 5 bytes of 0x42
        assert_eq!(data[30], 0xFC);
        assert_eq!(data[31], 0x42);

        // Long repeat of 467 coming up…
        // (-(-127)+1) = 128 bytes of 0x24
        assert_eq!(data[32], 0x81);
        assert_eq!(data[33], 0x24);

        // (-(-127)+1) = 128 bytes of 0x24
        assert_eq!(data[34], 0x81);
        assert_eq!(data[35], 0x24);

        // (-(-127)+1) = 128 bytes of 0x24
        assert_eq!(data[36], 0x81);
        assert_eq!(data[37], 0x24);

        // (-(-82)+1) = 83 bytes of 0x24
        assert_eq!(data[38], 0xAE);
        assert_eq!(data[39], 0x24);

        // (-(-7)+1) = 8 bytes of 0x24
        assert_eq!(data[40], 0xF9);
        assert_eq!(data[41], 0x73);

        // Second row
        // (-(-4)+1) = 5 bytes of 0x73
        assert_eq!(data[42], 0xFC);
        assert_eq!(data[43], 0x73);

        // (-(-9)+1) = 11 bytes of 0x42
        assert_eq!(data[44], 0xF6);
        assert_eq!(data[45], 0x42);

        // (1)+1 = 2 bytes of discrete data
        assert_eq!(data[46], 0x01);
        assert_eq!(data[47], 0x24);
        assert_eq!(data[48], 0x24);

        // (-(-2)+1) = 3 bytes of 0x42
        assert_eq!(data[49], 0xFE);
        assert_eq!(data[50], 0x42);

        // (-(-2)+1) = 3 bytes of 0x24
        assert_eq!(data[51], 0xFE);
        assert_eq!(data[52], 0x24);

        // (-(-3)+1) = 4 bytes of 0x42
        assert_eq!(data[53], 0xFD);
        assert_eq!(data[54], 0x42);

        // (-(-5)+1) = 6 bytes of 0x24
        assert_eq!(data[55], 0xFB);
        assert_eq!(data[56], 0x24);

        // (-(-4)+1) = 5 bytes of 0x42
        assert_eq!(data[57], 0xFC);
        assert_eq!(data[58], 0x42);

        // (-(-3)+1) = 4 bytes of 0x24
        assert_eq!(data[59], 0xFD);
        assert_eq!(data[60], 0x24);

        // (1)+1 = 2 bytes of discrete data
        assert_eq!(data[61], 0x01);
        assert_eq!(data[62], 0x42);
        assert_eq!(data[63], 0x42);

        // (-(-2)+1) = 3 bytes of 0x24
        assert_eq!(data[64], 0xFE);
        assert_eq!(data[65], 0x24);

        // (-(-5)+1) = 6 bytes of 0x42
        assert_eq!(data[66], 0xFB);
        assert_eq!(data[67], 0x42);

        // Long repeat of 315 coming up…
        // (-(-127)+1) = 128 bytes of 0x24
        assert_eq!(data[68], 0x81);
        assert_eq!(data[69], 0x24);

        // (-(-127)+1) = 128 bytes of 0x24
        assert_eq!(data[70], 0x81);
        assert_eq!(data[71], 0x24);

        // (-(-58)+1) = 59 bytes of 0x24
        assert_eq!(data[72], 0xC6);
        assert_eq!(data[73], 0x24);

        // (-(-4)+1) = 5 bytes of 0xFC
        assert_eq!(data[74], 0xFC);
        assert_eq!(data[75], 0xFC);

        // Long repeat of 129 coming up…
        // (-(-127)+1) = 128 bytes of 0x24
        assert_eq!(data[76], 0x81);
        assert_eq!(data[77], 0x24);

        // (1)+1 = 2 bytes of discrete data
        assert_eq!(data[78], 0x01);
        assert_eq!(data[79], 0x24);
        assert_eq!(data[80], 0x42);

        // (-(-6)+1) = 7 bytes of 0x24
        assert_eq!(data[81], 0xFA);
        assert_eq!(data[82], 0x24);

        // (-(-3)+1) = 4 bytes of 0x42
        assert_eq!(data[83], 0xFD);
        assert_eq!(data[84], 0x42);

        // (-(-13)+1) = 14 bytes of 0x24
        assert_eq!(data[85], 0xF3);
        assert_eq!(data[86], 0x24);

        // (-(-9)+1) = 10 bytes of 0x73
        assert_eq!(data[87], 0xF7);
        assert_eq!(data[88], 0x73);

        // (0)+1 = 1 byte of discrete data
        assert_eq!(data[89], 0x00);
        assert_eq!(data[90], 0x42);

        // Not comparing the third and fourth rows in detail.

        assert_eq!(data, expected_data);
    }
}
