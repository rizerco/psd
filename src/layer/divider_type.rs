/// The divider type.
#[repr(u32)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DividerType {
    /// Other type of layer.
    Other = 0,
    /// An open folder.
    OpenFolder = 1,
    /// A closed folder.
    ClosedFolder = 2,
    /// A section divider, used to mark the end of a group.
    SectionDivider = 3,
}

impl From<u32> for DividerType {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::OpenFolder,
            2 => Self::ClosedFolder,
            3 => Self::SectionDivider,
            _ => Self::Other,
        }
    }
}
