// PackBits technical note: https://web.archive.org/web/20080705155158/http://developer.apple.com/technotes/tn/tn1023.html

/// Returns the data encoded using the RLE algorithm.
pub fn encoded(source: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();

    let mut repeat_count = 0;
    let mut non_repeating_run = Vec::new();
    let mut previous_byte: Option<u8> = None;
    let mut previous_repeat_count = 0;

    for (index, byte) in source.iter().enumerate() {
        let byte = *byte;
        let is_end = index == source.len() - 1;

        if Some(byte) == previous_byte && previous_repeat_count != 128 {
            repeat_count += 1;
        } else {
            repeat_count = 1;
        }

        non_repeating_run.push(byte);

        // Ending a non-repeating run due to too many repeats.
        if repeat_count == 3 {
            let length = non_repeating_run.len();
            // Drop the last 3 elements
            if length >= 3 {
                non_repeating_run.truncate(length - 3);
            } else {
                non_repeating_run.clear();
            }
            if non_repeating_run.is_empty() == false {
                output.push((non_repeating_run.len() - 1) as u8);
                output.append(&mut non_repeating_run);
            }
            non_repeating_run.clear();
        }

        // Ending a non-repeating run because the data size
        // got too high.
        if non_repeating_run.len() == 128 {
            output.push((non_repeating_run.len() - 1) as u8);
            output.append(&mut non_repeating_run);
            non_repeating_run.clear();
            repeat_count = 0;
        }

        // Ending a repeating run because the repeating
        // value has stopped repeating.
        if previous_repeat_count > 2 && repeat_count == 1 {
            let Some(previous_byte) = previous_byte else {
                break;
            };
            output.push(u8::MAX - (previous_repeat_count - 1) as u8 + 1);
            output.push(previous_byte);
            non_repeating_run = vec![byte];
        }

        // Handle the end of the data.
        if is_end {
            // Write the final repeat.
            if repeat_count >= 3 {
                output.push(u8::MAX - (repeat_count - 1) + 1);
                output.push(byte);
            }
            // Write the final non-repeating run.
            else if non_repeating_run.is_empty() == false {
                output.push((non_repeating_run.len() - 1) as u8);
                output.append(&mut non_repeating_run);
            }
        }

        previous_byte = Some(byte);
        previous_repeat_count = repeat_count;
    }

    output
}

// MARK: Test

#[cfg(test)]
mod tests {
    #[test]
    fn encode() {
        let original_bytes = vec![
            0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0xAA, 0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0x22,
            0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
        ];

        let encoded_data = super::encoded(&original_bytes);

        assert_eq!(encoded_data.len(), 15);

        // (-(-2)+1) = 3 bytes of the pattern $AA
        assert_eq!(encoded_data[0], 0xFE);
        assert_eq!(encoded_data[1], 0xAA);

        // (2)+1 = 3 bytes of discrete data
        assert_eq!(encoded_data[2], 0x02);
        assert_eq!(encoded_data[3], 0x80);
        assert_eq!(encoded_data[4], 0x00);
        assert_eq!(encoded_data[5], 0x2A);

        // (-(-3)+1) = 4 bytes of the pattern $AA
        assert_eq!(encoded_data[6], 0xFD);
        assert_eq!(encoded_data[7], 0xAA);

        // (3)+1 = 4 bytes of discrete data
        assert_eq!(encoded_data[8], 0x03);
        assert_eq!(encoded_data[9], 0x80);
        assert_eq!(encoded_data[10], 0x00);
        assert_eq!(encoded_data[11], 0x2A);
        assert_eq!(encoded_data[12], 0x22);

        // (-(-9)+1) = 10 bytes of the pattern $AA
        assert_eq!(encoded_data[13], 0xF7);
        assert_eq!(encoded_data[14], 0xAA);
    }

    #[test]
    fn tiny_encode() {
        let original_data = vec![0xac, 0x00];
        let expected_data = vec![0x01, 0xac, 0x00];
        let encoded_data = super::encoded(&original_data);
        assert_eq!(encoded_data, expected_data);
    }

    #[test]
    fn encode_with_repeat_of_two() {
        let original_data = vec![
            0x73, 0x73, 0x73, 0x73, 0x73, 0x42, 0x42, 0x73, 0x73, 0x73, 0x73, 0x42, 0x42, 0x42,
        ];

        let encoded_data = super::encoded(&original_data);

        // Expecting: FC 73 01 42 42 80 FD 73 FE 42 80 FE 73 FD 42 80 FE 73 FD 42 80

        // (-(-4)+1) = 5 bytes of 0x73
        assert_eq!(encoded_data[0], 0xFC);
        assert_eq!(encoded_data[1], 0x73);

        // (1)+1 = 2 bytes of discrete data (even though it is a small repeat)
        assert_eq!(encoded_data[2], 0x01);
        assert_eq!(encoded_data[3], 0x42);
        assert_eq!(encoded_data[4], 0x42);

        // (-(-3)+1) = 4 bytes of 0x73
        assert_eq!(encoded_data[5], 0xFD);
        assert_eq!(encoded_data[6], 0x73);

        // (-(-2)+1) = 3 bytes of 0x73
        assert_eq!(encoded_data[7], 0xFE);
        assert_eq!(encoded_data[8], 0x42);
    }

    #[test]
    fn rle_encode_with_double_repeat_of_two() {
        let original_data = vec![0xB1, 0xB1, 0x00, 0x00];

        let encoded_data = super::encoded(&original_data);

        // Expecting: 03 B1 B1 00 00

        assert_eq!(encoded_data.len(), 5);

        assert_eq!(encoded_data[0], 0x03);
        assert_eq!(encoded_data[1], 0xB1);
        assert_eq!(encoded_data[2], 0xB1);
        assert_eq!(encoded_data[3], 0x00);
        assert_eq!(encoded_data[4], 0x00);
    }

    #[test]
    fn rle_encode_with_one_one_two_pattern() {
        let original_data = vec![0xFC, 0x00, 0xFC, 0xFC];

        let encoded_data = super::encoded(&original_data);

        // Expecting: 03 FC 00 FC FC

        assert_eq!(encoded_data.len(), 5);

        assert_eq!(encoded_data[0], 0x03);
        assert_eq!(encoded_data[1], 0xFC);
        assert_eq!(encoded_data[2], 0x00);
        assert_eq!(encoded_data[3], 0xFC);
        assert_eq!(encoded_data[4], 0xFC);
    }

    #[test]
    fn rle_encode_with128_repeats() {
        let mut original_data = vec![0xFF; 128];
        original_data.push(0xEE);

        let encoded_data = super::encoded(&original_data);

        assert_eq!(encoded_data.len(), 4);
        assert_eq!(encoded_data[0], 0x81);
        assert_eq!(encoded_data[1], 0xFF);
        assert_eq!(encoded_data[2], 0x00);
        assert_eq!(encoded_data[3], 0xEE);
    }

    #[test]
    fn rle_encode_with129_repeats() {
        let original_data = vec![0xff; 129];

        let encoded_data = super::encoded(&original_data);

        assert_eq!(encoded_data.len(), 4);
        assert_eq!(encoded_data[0], 0x81);
        assert_eq!(encoded_data[1], 0xFF);
        assert_eq!(encoded_data[2], 0x00);
        assert_eq!(encoded_data[3], 0xFF);
    }

    #[test]
    fn rle_encode_long_non_repeating_values() {
        let original_data = vec![
            0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76,
            0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A,
            0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39,
            0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A,
            0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39,
            0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A,
            0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39,
            0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A,
            0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x63, 0x63, 0x76, 0x76, 0x63, 0x63,
            0x76, 0x76, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84,
            0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x91, 0x91, 0x81, 0x81,
            0x91, 0x91, 0x81, 0x81, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x76, 0x76,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
            0x76, 0x76, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84,
            0x76, 0x76, 0x76, 0x76, 0x76, 0x76,
        ];

        let encoded_data = super::encoded(&original_data);

        assert_eq!(encoded_data[0], 0x7F);
        assert_eq!(encoded_data[129], 0x1D);

        let expected_data = vec![
            0x7F, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76,
            0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x6A, 0x6A, 0x39, 0x39, 0x6A,
            0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39,
            0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A,
            0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39,
            0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A,
            0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39,
            0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A,
            0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x39, 0x39, 0x6A, 0x6A, 0x63, 0x63, 0x76, 0x76, 0x63,
            0x63, 0x76, 0x76, 0x1D, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76,
            0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x84, 0x84, 0x76, 0x76, 0x91, 0x91,
            0x81, 0x81, 0x91, 0x91, 0x81, 0x81, 0xF9, 0x84, 0xEF, 0x76, 0xF5, 0x84, 0xFB, 0x76,
        ];

        assert_eq!(encoded_data, expected_data);
    }
}
