use thiserror::Error;

#[derive(Error, Debug)]
/// An error that can occur when writing PSD data.
pub enum WriteError {
    #[error("The image compression is not supported.")]
    UnsupportedCompression,
    #[error("The image is invalid.")]
    InvalidImage,
}
