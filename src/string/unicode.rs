use file_stream::write::FileStreamWriter;

use crate::document;

/// Returns the unicode string data from a string.
pub fn data_from_string(string: Option<&String>) -> anyhow::Result<Vec<u8>> {
    let mut file_stream = FileStreamWriter::new();
    file_stream.write_bytes(&document::constants::RESOURCE_SIGNATURE)?;
    file_stream.write_bytes("luni".as_bytes())?;

    let empty_string = String::new();
    let string = string.unwrap_or(&empty_string);

    let mut string_file_stream = FileStreamWriter::new();

    let length = string.encode_utf16().count() as u32;
    string_file_stream.write_be(&length)?;

    for character in string.encode_utf16() {
        string_file_stream.write_be(&character)?;
    }

    // Write the length of the string data.
    file_stream.write_be(&(string_file_stream.data().len() as u32))?;
    file_stream.write_bytes(string_file_stream.data())?;

    // The documentation says to pad, but this seems to make it less compatible.

    Ok(file_stream.data().to_vec())
}

#[cfg(test)]
mod tests {
    #[test]
    fn data_from_string() {
        let expected_data = vec![
            0x38, 0x42, 0x49, 0x4D, // 8BIM
            0x6C, 0x75, 0x6E, 0x69, // luni
            0x00, 0x00, 0x00, 0x0E, // Length of bytes
            0x00, 0x00, 0x00, 0x05, // Length of string
            0x00, 0x4C, // .L
            0x00, 0x61, // .a
            0x00, 0x79, // .y
            0x00, 0x65, // .e
            0x00, 0x72, // .r
        ];

        let result = super::data_from_string(Some(&"Layer".to_string())).unwrap();

        assert_eq!(result, expected_data);
    }

    #[test]
    fn data_from_tricky_string() {
        let expected_data = vec![
            0x38, 0x42, 0x49, 0x4D, // 8BIM
            0x6C, 0x75, 0x6E, 0x69, // luni
            0x00, 0x00, 0x00, 0x12, // Length of bytes
            0x00, 0x00, 0x00, 0x07, // Length of string
            0x00, 0x59, // .Y
            0x00, 0x65, // .e
            0x00, 0x6C, // .l
            0x20, 0x19, // ’
            0x00, 0x6C, // .l
            0x00, 0x6F, // .o
            0x00, 0x77, // .w
        ];

        let result = super::data_from_string(Some(&"Yel’low".to_string())).unwrap();

        assert_eq!(result, expected_data);
    }
}
