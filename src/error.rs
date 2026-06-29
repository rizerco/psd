use thiserror::Error;

/// An error that can occur when writing PSD data.
#[derive(Error, Debug)]
pub enum WriteError {
    #[error("The image compression is not supported.")]
    UnsupportedCompression,
    #[error("The image is invalid.")]
    InvalidImage,
}

/// An error that can occur when reading a PSD file.
#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Invalid file signature.")]
    InvalidFileSignature,
    #[error("Invalid resource signature.")]
    InvalidResourceSignature,
    #[error("The file format version number is not supported.")]
    UnsupportedVersionNumber,
    #[error("The maximum canvas size was exceeded.")]
    MaximumSizeExceeded,
    #[error("The maximum number of layers was exceeded.")]
    MaximumNumberOfLayersExceeded,
    #[error("No layer information was found.")]
    NoLayerInformation,
    #[error("Unsupported image compression.")]
    UnsupportedImageCompression,
    #[error("Unsupported image channels. Only RGBA is currently supported.")]
    UnsupportedImageChannels,
}
