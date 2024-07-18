use file_stream::write::FileStreamWriter;
use graphics::Image;

use crate::{
    color_channel::{ColorChannel, ColorChannelType},
    error::WriteError,
    image_compression::ImageCompression,
};

/// Returns the image data for use in Photoshop documents.
pub fn psd_data(image: &Image, compression: &ImageCompression) -> anyhow::Result<Vec<u8>> {
    match compression {
        ImageCompression::ZipWithoutPrediction | ImageCompression::ZipWithPrediction => {
            anyhow::bail!(WriteError::UnsupportedCompression)
        }
        _ => (),
    }

    let data_length = (image.size.width * image.size.height) as usize;
    let mut red_channel = ColorChannel::new(ColorChannelType::Red, data_length);
    let mut green_channel = ColorChannel::new(ColorChannelType::Green, data_length);
    let mut blue_channel = ColorChannel::new(ColorChannelType::Blue, data_length);
    let mut alpha_channel = ColorChannel::new(ColorChannelType::Alpha, data_length);

    for y_position in 0..image.size.height {
        for x_position in 0..image.size.width {
            // For now we’re just assuming that the bytes
            // are in RGBA order.
            let target_index = (y_position * image.size.width + x_position) as usize;
            let source_index = ((y_position * image.bytes_per_row) + (x_position * 4)) as usize;
            red_channel.data[target_index] = image.data[source_index];
            green_channel.data[target_index] = image.data[source_index + 1];
            blue_channel.data[target_index] = image.data[source_index + 2];
            alpha_channel.data[target_index] = image.data[source_index + 3];
        }
    }

    let mut file_stream = FileStreamWriter::new();
    file_stream.write_be(&compression.raw_value())?;
    if compression == &ImageCompression::Rle {
        let red = red_channel.rle_encoded_components(image.size.height)?;
        let green = green_channel.rle_encoded_components(image.size.height)?;
        let blue = blue_channel.rle_encoded_components(image.size.height)?;
        let alpha = alpha_channel.rle_encoded_components(image.size.height)?;
        // Put all of the line lengths up front.
        file_stream.write_bytes(&red.line_lengths)?;
        file_stream.write_bytes(&green.line_lengths)?;
        file_stream.write_bytes(&blue.line_lengths)?;
        file_stream.write_bytes(&alpha.line_lengths)?;
        // Then write all of the actual image data.
        file_stream.write_bytes(&red.data)?;
        file_stream.write_bytes(&green.data)?;
        file_stream.write_bytes(&blue.data)?;
        file_stream.write_bytes(&alpha.data)?;
    } else {
        file_stream.write_bytes(&red_channel.data)?;
        file_stream.write_bytes(&green_channel.data)?;
        file_stream.write_bytes(&blue_channel.data)?;
        file_stream.write_bytes(&alpha_channel.data)?;
    }

    let data = file_stream.data().to_vec();
    Ok(data)
}

#[cfg(test)]
mod tests {
    use graphics::{Color, Image, Size};

    use crate::image_compression::ImageCompression;

    #[test]
    fn raw_data() {
        let color = Color::from_rgb_u32(0x24a4ee);
        let image = Image::color(
            &color,
            Size {
                width: 2,
                height: 2,
            },
        );
        let data = super::psd_data(&image, &ImageCompression::RawData).unwrap();

        // Compression type
        assert_eq!(data[0..=1], [0x00, 0x00]);

        // Red
        assert_eq!(data[2..=5], [0x24, 0x24, 0x24, 0x24]);

        // Green
        assert_eq!(data[6..=9], [0xa4, 0xa4, 0xa4, 0xa4]);

        // Blue
        assert_eq!(data[10..=13], [0xee, 0xee, 0xee, 0xee]);

        // Alpha
        assert_eq!(data[14..=17], [0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn raw_data_with_alpha() {
        let color = Color::from_rgba_u32(0x23a4ee99);
        let image = Image::color(
            &color,
            Size {
                width: 2,
                height: 2,
            },
        );
        let data = super::psd_data(&image, &ImageCompression::RawData).unwrap();

        // Compression type
        assert_eq!(data[0..=1], [0x00, 0x00]);

        // Red
        assert_eq!(data[2..=5], [0x23, 0x23, 0x23, 0x23]);

        // Green
        assert_eq!(data[6..=9], [0xa4, 0xa4, 0xa4, 0xa4]);

        // Blue
        assert_eq!(data[10..=13], [0xee, 0xee, 0xee, 0xee]);

        // Alpha
        assert_eq!(data[14..=17], [0x99, 0x99, 0x99, 0x99]);
    }

    #[test]
    fn rle_data() {
        let color = Color::from_rgb_u32(0x24a4ee); // A mid blue
        let image = Image::color(
            &color,
            Size {
                width: 2,
                height: 2,
            },
        );
        let data = super::psd_data(&image, &ImageCompression::Rle).unwrap();

        // Compression type
        assert_eq!(data[0..=1], [0x00, 0x01]);

        // All of the line lengths (2 rows x 4 channels).
        assert_eq!(data[2..=3], [0x00, 0x03]);
        assert_eq!(data[4..=5], [0x00, 0x03]);
        assert_eq!(data[6..=7], [0x00, 0x03]);
        assert_eq!(data[8..=9], [0x00, 0x03]);
        assert_eq!(data[10..=11], [0x00, 0x03]);
        assert_eq!(data[12..=13], [0x00, 0x03]);
        assert_eq!(data[14..=15], [0x00, 0x03]);
        assert_eq!(data[16..=17], [0x00, 0x03]);

        // Red
        assert_eq!(data[18..=20], [0x01, 0x24, 0x24]);
        assert_eq!(data[21..=23], [0x01, 0x24, 0x24]);

        // Green
        assert_eq!(data[24..=26], [0x01, 0xa4, 0xa4]);
        assert_eq!(data[27..=29], [0x01, 0xa4, 0xa4]);

        // Blue
        assert_eq!(data[30..=32], [0x01, 0xee, 0xee]);
        assert_eq!(data[33..=35], [0x01, 0xee, 0xee]);

        // Alpha
        assert_eq!(data[36..=38], [0x01, 0xff, 0xff]);
        assert_eq!(data[39..=41], [0x01, 0xff, 0xff]);
    }

    #[test]
    fn rle_data_with_alpha() {
        let color = Color::from_rgba_u32(0x24a4ee99); // A mid blue
        let image = Image::color(
            &color,
            Size {
                width: 2,
                height: 2,
            },
        );
        let data = super::psd_data(&image, &ImageCompression::Rle).unwrap();

        // Compression type
        assert_eq!(data[0..=1], [0x00, 0x01]);

        // All of the line lengths (2 rows x 4 channels).
        assert_eq!(data[2..=3], [0x00, 0x03]);
        assert_eq!(data[4..=5], [0x00, 0x03]);
        assert_eq!(data[6..=7], [0x00, 0x03]);
        assert_eq!(data[8..=9], [0x00, 0x03]);
        assert_eq!(data[10..=11], [0x00, 0x03]);
        assert_eq!(data[12..=13], [0x00, 0x03]);
        assert_eq!(data[14..=15], [0x00, 0x03]);
        assert_eq!(data[16..=17], [0x00, 0x03]);

        // Red
        assert_eq!(data[18..=20], [0x01, 0x24, 0x24]);
        assert_eq!(data[21..=23], [0x01, 0x24, 0x24]);

        // Green
        assert_eq!(data[24..=26], [0x01, 0xa4, 0xa4]);
        assert_eq!(data[27..=29], [0x01, 0xa4, 0xa4]);

        // Blue
        assert_eq!(data[30..=32], [0x01, 0xee, 0xee]);
        assert_eq!(data[33..=35], [0x01, 0xee, 0xee]);

        // Alpha
        assert_eq!(data[36..=38], [0x01, 0x99, 0x99]);
        assert_eq!(data[39..=41], [0x01, 0x99, 0x99]);
    }
}
