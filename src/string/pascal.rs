use file_stream::read::FileStreamReader;

/// Returns the Pascal UCSD string data from a string.
pub fn data_from_string(string: Option<&String>) -> anyhow::Result<Vec<u8>> {
    let Some(string) = string else {
        return Ok(vec![0x00, 0x00]);
    };

    let mut bytes = string.as_bytes().to_vec();
    bytes.insert(0, bytes.len() as u8);
    Ok(bytes)
}

/// Returns a string from USCD string data.
pub fn string_from_file_stream(
    file_stream: &mut FileStreamReader,
) -> anyhow::Result<(usize, Option<String>)> {
    // Get the length of the string
    let length: u8 = file_stream.read_be()?;

    let string_data = file_stream.read_bytes(length as usize)?;
    let output = std::str::from_utf8(&string_data).ok();
    Ok((length as usize, output.map(|output| output.to_string())))
}

#[cfg(test)]
mod tests {
    use crate::data;

    #[test]
    fn data_from_string() {
        let expected_data = vec![0x05, 0x45, 0x6d, 0x70, 0x74, 0x79, 0x00, 0x00];

        let mut result = super::data_from_string(Some(&"Empty".to_string())).unwrap();
        data::pad(&mut result, 4);

        assert_eq!(result, expected_data);
    }
}
