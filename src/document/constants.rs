pub(crate) mod resource_identifiers;

// Many of these constants are taken from this
// wonderful document: http://www.adobe.com/devnet-apps/photoshop/fileformatashtml/

/// The file signature for all Photoshop documents.
pub const FILE_SIGNATURE: &str = "8BPS";

/// The signature for various resources.
pub const RESOURCE_SIGNATURE: &str = "8BIM";

/// The version number of all PSDs (PSBs are version 2, but aren’t supported).
pub const VERSION_NUMBER: i16 = 1;
