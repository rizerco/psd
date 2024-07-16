/// Pads the data to be a multiple of a number of bytes.
pub(crate) fn pad(data: &mut Vec<u8>, number_of_bytes: usize) {
    while data.len() % number_of_bytes != 0 {
        data.push(0);
    }
}
