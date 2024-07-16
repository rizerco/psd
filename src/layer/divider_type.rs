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
