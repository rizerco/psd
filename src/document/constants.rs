pub(crate) mod resource_identifiers;

// Many of these constants are taken from this
// wonderful document: http://www.adobe.com/devnet-apps/photoshop/fileformatashtml/

/// The file signature for all Photoshop documents.
pub const FILE_SIGNATURE: [u8; 4] = [0x38, 0x42, 0x50, 0x53]; // 8BPS

/// The signature for various resources.
pub const RESOURCE_SIGNATURE: [u8; 4] = [0x38, 0x42, 0x49, 0x4d]; // "8BIM"

/// The version number of all PSDs (PSBs are version 2, but aren’t supported).
pub const VERSION_NUMBER: i16 = 1;
