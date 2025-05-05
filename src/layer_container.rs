use crate::layer::Layer;
use crate::layer::LayerType;

/// Specification for an object that can contain layers.
pub trait LayerContainer {
    /// Returns the layers.
    fn layers(&self) -> Vec<&Layer>;

    /// Returns the number of sublayers.
    fn number_of_layers(&self) -> usize {
        let mut count = 0;

        for layer in self.layers() {
            count += 1;
            if let LayerType::Group(info) = &layer.layer_type {
                count += info.number_of_layers();
                count += 1; // Groups count as 2 layers.
            };
        }

        count
    }

    /// Returns all the layers by recursing into any groups
    /// in this container.
    fn all_layers(&self) -> Vec<&Layer> {
        let mut output = Vec::new();

        for layer in self.layers() {
            output.push(layer);
            if let LayerType::Group(info) = &layer.layer_type {
                output.append(&mut info.layers());
            };
        }

        output
    }
}
