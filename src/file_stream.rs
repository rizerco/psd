use file_stream::write::FileStreamWriter;

/// Writes a string with four bytes at the start for the string length.
pub fn write_string_with_length(
    string: &String,
    file_stream: &mut FileStreamWriter,
) -> anyhow::Result<()> {
    let length = string.len() as u32;
    file_stream.write_be(&(length + 1))?;
    for character in string.chars() {
        file_stream.write_be(&(character as u16))?;
    }
    // Add some padding.
    file_stream.write_be(&0u16)?;
    Ok(())
}
