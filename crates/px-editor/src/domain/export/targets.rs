use std::collections::BTreeSet;
use std::path::PathBuf;

use super::{ExportJob, stem};
use crate::domain::Document;
use crate::domain::Layer;
use crate::domain::visibility::hidden_layer_indices;

pub(super) struct Target {
    pub(super) hidden: BTreeSet<usize>,
    pub(super) out: PathBuf,
    pub(super) layers: Vec<String>,
}

impl Target {
    pub(super) fn all(
        job: &ExportJob,
        document: &Document,
        hidden: &BTreeSet<String>,
    ) -> Vec<Self> {
        let layers = document.layers();
        let hidden = hidden_layer_indices(document, hidden);
        let visible: Vec<usize> = (0..layers.len())
            .filter(|index| !hidden.contains(index))
            .collect();
        if !(job.split_layers && job.kind.splits()) || visible.is_empty() {
            return vec![Self {
                hidden,
                out: job.out.clone(),
                layers: visible
                    .iter()
                    .map(|index| layers[*index].name.clone())
                    .collect(),
            }];
        }
        let stem = stem(&job.out);
        visible
            .iter()
            .map(|index| Self {
                hidden: (0..layers.len()).filter(|other| other != index).collect(),
                out: job.out.with_file_name(format!(
                    "{stem}-{}.{}",
                    suffix(layers, *index),
                    job.kind.extension()
                )),
                layers: vec![layers[*index].name.clone()],
            })
            .collect()
    }
}

fn suffix(layers: &[Layer], index: usize) -> String {
    let name = &layers[index].name;
    if name.is_empty() {
        format!("layer-{index}")
    } else {
        name.replace(['/', '\\'], "-")
    }
}
