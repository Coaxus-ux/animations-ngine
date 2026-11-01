use std::collections::BTreeSet;

use px_engine::{Param, Scene};

use super::Document;

#[must_use]
pub fn with_hidden_layers(
    document: &Document,
    scene: &Scene,
    hidden: &BTreeSet<String>,
) -> Option<Scene> {
    with_hidden_layer_indices(document, scene, &hidden_layer_indices(document, hidden))
}

#[must_use]
pub fn with_hidden_layer_indices(
    document: &Document,
    scene: &Scene,
    hidden: &BTreeSet<usize>,
) -> Option<Scene> {
    if hidden.is_empty() {
        return None;
    }
    let loaded = document.loaded()?;
    let spans: Vec<(usize, usize)> = hidden
        .iter()
        .filter_map(|index| document.layers().get(*index))
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

#[must_use]
pub fn hidden_layer_indices(document: &Document, hidden: &BTreeSet<String>) -> BTreeSet<usize> {
    document
        .layers()
        .iter()
        .enumerate()
        .filter(|(_, layer)| hidden.contains(&layer.name))
        .map(|(index, _)| index)
        .collect()
}
