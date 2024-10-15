use crate::layer_container::LayerContainer;

use super::Layer;

/// Information for the group.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupInfo {
    /// The child layers.
    pub layers: Vec<Layer>,
}

// MARK: Creation

impl GroupInfo {
    /// Creates new a new group info structure.
    pub fn new(layers: Vec<Layer>) -> Self {
        Self { layers }
    }
}

impl LayerContainer for GroupInfo {
    fn layers(&self) -> Vec<&Layer> {
        self.layers.iter().collect()
    }
}
