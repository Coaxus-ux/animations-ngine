use std::collections::BTreeSet;

use px_engine::{Param, Scene};

use super::Document;

/// `scene` with every shape written inside a hidden layer made invisible, or `None` when
/// nothing is hidden or the document hasn't loaded.
#[must_use]
pub fn with_hidden_layers(
    document: &Document,
    scene: &Scene,
    hidden: &BTreeSet<String>,
) -> Option<Scene> {
    if hidden.is_empty() {
        return None;
    }
    let loaded = document.loaded()?;
    let spans: Vec<(usize, usize)> = document
        .layers()
        .iter()
        .filter(|layer| hidden.contains(&layer.name))
        .filter_map(|layer| document.source().span(&layer.path))
        .collect();
    let mut spec = scene.spec().clone();
    for (shape, origin) in spec.shapes.iter_mut().zip(&loaded.origins) {
        if origin.file == 0
            && spans
                .iter()
                .any(|(start, end)| origin.offset >= *start && origin.offset < *end)
        {
            shape.style.visible = Param::Fixed(0.0);
        }
    }
    Scene::new(spec).ok()
}
