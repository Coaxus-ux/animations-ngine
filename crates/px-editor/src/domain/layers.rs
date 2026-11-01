use px_engine::{Geometry, Param, PixelArt, Scene};
use px_format::Origin;
use px_format::edit::{NodePath, Source, children_of, id_of};

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub path: NodePath,
    pub child: usize,
    pub art: Option<Art>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Art {
    pub path: NodePath,
    pub shape: usize,
    pub origin: Option<(i32, i32)>,
}

impl Layer {
    #[must_use]
    pub fn paintable(&self) -> bool {
        self.art.as_ref().is_some_and(|art| art.origin.is_some())
    }
}

#[must_use]
pub fn scene_path(scene: &Scene) -> NodePath {
    NodePath::root().child("scene", Some(scene.name()), 0)
}

#[must_use]
pub fn layers(source: &Source, scene: &Scene, origins: &[Origin]) -> Vec<Layer> {
    let root = scene_path(scene);
    let Some(node) = source.node(&root) else {
        return Vec::new();
    };
    let children = children_of(node);
    let mut rows = Vec::new();
    for (child, kid) in children.iter().enumerate() {
        let name = id_of(kid).unwrap_or("").to_owned();
        let path = Source::path_of(&root, children, kid);
        let art_path = match kid.name().value() {
            "layer" => {
                let inner = children_of(kid);
                inner
                    .iter()
                    .find(|node| node.name().value() == "pixels")
                    .map(|pixels| Source::path_of(&path, inner, pixels))
            }
            "pixels" => Some(path.clone()),
            _ => continue,
        };
        rows.push(Layer {
            name,
            art: art_path.and_then(|art_path| art(source, scene, origins, art_path)),
            path,
            child,
        });
    }
    rows
}

fn art(source: &Source, scene: &Scene, origins: &[Origin], path: NodePath) -> Option<Art> {
    let (start, _) = source.span(&path)?;
    let shape = origins
        .iter()
        .position(|origin| origin.file == 0 && origin.offset == start)?;
    let Geometry::Pixels(pixels) = &scene.shapes().get(shape)?.geometry else {
        return None;
    };
    Some(Art {
        path,
        shape,
        origin: fixed_origin(pixels),
    })
}

fn fixed_origin(art: &PixelArt) -> Option<(i32, i32)> {
    let whole = |param: &Param| match param {
        Param::Fixed(value) if value.fract() == 0.0 => {
            #[expect(clippy::cast_possible_truncation, reason = "a whole canvas coordinate")]
            let value = *value as i32;
            Some(value)
        }
        _ => None,
    };
    Some((whole(&art.x)?, whole(&art.y)?))
}
