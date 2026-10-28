use std::collections::BTreeMap;

use px_engine::{Anchor, Param};
use px_expr::{Blend, Program};

use crate::compiler::program_of;

use crate::schema::Kind;
use crate::suggest;

/// Something an object can be attached to: a point of an object (its `x y`, or `x1 y1`),
/// or a placed scene's origin.
#[derive(Clone, Debug)]
enum Target {
    Point {
        anchor: Anchor,
        layer: usize,
    },
    Placed {
        first: usize,
        count: usize,
        layer: usize,
    },
}

/// Every id met so far, per scene they belong to (`None` is the root scene), so
/// `attach="hero.hand"` can walk into placed scenes.
#[derive(Debug, Default)]
pub(crate) struct Targets(BTreeMap<(Option<usize>, String), Target>);

impl Targets {
    /// Records an object's point; one that is itself attached is measured from its anchor.
    pub(crate) fn point(
        &mut self,
        owner: Option<usize>,
        id: &str,
        kind: Kind,
        params: &[Param],
        attached: Option<&Anchor>,
        layer: usize,
    ) {
        let read = |names: [&str; 2]| {
            names
                .iter()
                .find_map(|name| kind.prop_index(name))
                .and_then(|index| params.get(index))
                .cloned()
        };
        let (Some(x), Some(y)) = (read(["x", "x1"]), read(["y", "y1"])) else {
            return;
        };
        let anchor = match attached {
            Some(base) => Anchor {
                x: sum(&base.x, &x),
                y: sum(&base.y, &y),
                ..base.clone()
            },
            None => Anchor {
                instance: owner,
                x,
                y,
            },
        };
        self.0
            .insert((owner, id.to_owned()), Target::Point { anchor, layer });
    }

    pub(crate) fn placed(
        &mut self,
        owner: Option<usize>,
        id: &str,
        first: usize,
        count: usize,
        layer: usize,
    ) {
        let target = Target::Placed {
            first,
            count,
            layer,
        };
        self.0.insert((owner, id.to_owned()), target);
    }

    /// The anchor `path` names from inside `owner`, and the layer it is drawn in.
    pub(crate) fn resolve(
        &self,
        owner: Option<usize>,
        path: &str,
    ) -> Result<(Anchor, usize), String> {
        let mut scene = owner;
        let mut parts = path.split('.').peekable();
        while let Some(part) = parts.next() {
            let last = parts.peek().is_none();
            match (self.0.get(&(scene, part.to_owned())), last) {
                (Some(Target::Point { anchor, layer }), true) => {
                    return Ok((anchor.clone(), *layer));
                }
                (Some(Target::Placed { count, .. }), _) if *count != 1 => {
                    return Err(format!(
                        "`{part}` places {count} copies; attach to a `use-scene` with one"
                    ));
                }
                (Some(Target::Placed { first, layer, .. }), true) => {
                    let origin = Anchor {
                        instance: Some(*first),
                        x: Param::Fixed(0.0),
                        y: Param::Fixed(0.0),
                    };
                    return Ok((origin, *layer));
                }
                (Some(Target::Placed { first, .. }), false) => scene = Some(*first),
                (Some(Target::Point { .. }), false) => {
                    return Err(format!(
                        "`{part}` is not a placed scene, so `{path}` can't look inside it"
                    ));
                }
                (None, _) => {
                    let known = self
                        .0
                        .keys()
                        .filter(|(owner, _)| *owner == scene)
                        .map(|(_, id)| id.as_str());
                    let hint = suggest::hint(part, known);
                    return Err(format!(
                        "nothing named `{part}` to attach to; attach only to objects written before.{hint}"
                    ));
                }
            }
        }
        Err("`attach` needs an id, like attach=\"hand\" or attach=\"hero.hand\"".to_owned())
    }
}

fn sum(a: &Param, b: &Param) -> Param {
    match (a, b) {
        (Param::Fixed(a), Param::Fixed(b)) => Param::Fixed(a + b),
        _ => Param::Expr(Program::blend(
            &program_of(a.clone()),
            &program_of(b.clone()),
            Blend::Add,
        )),
    }
}
