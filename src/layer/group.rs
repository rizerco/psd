use crate::layer_container::LayerContainer;

use super::Layer;

/// Information for the group.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupInfo {
    /// The child layers.
    layers: Vec<Layer>,
}

impl LayerContainer for GroupInfo {
    fn layers(&self) -> Vec<&Layer> {
        self.layers.iter().collect()
    }
}
