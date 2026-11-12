use kdl::KdlNode;
use px_core::hash_seed;
use px_engine::{Instance, Looping, MAX_INSTANCES, Param, Window};
use px_expr::Env;

use crate::assemble::{Output, Request, Scope};
use crate::diagnostic::Diagnostics;
use crate::expand;
use crate::scene::{self, Builder, Overrides};
use crate::schema::Kind;
use crate::suggest;
use crate::timeline::Track;

const MAX_DEPTH: usize = 8;
const SEAM_TOLERANCE: f64 = 1e-6;

pub(crate) fn expand(
    builder: &mut Builder<'_, '_>,
    request: &Request,
    out: &mut Output,
    diags: &mut Diagnostics,
) {
    let name = request.placement.scene.as_str();
    let Some((file, node)) = find(builder, request, diags) else {
        return;
    };
    let host_file = diags.in_file(file);
    let header = scene::header(node, builder.catalog, diags);
    diags.in_file(host_file);
    if header.palette_name.is_some_and(|palette| {
        palette != builder.palette_name && !builder.palette.includes(palette)
    }) {
        diags.error(
            request.offset,
            format!(
                "\"{name}\" uses another palette; a placed scene must use \"{}\" or a palette it includes",
                builder.palette_name
            ),
        );
        return;
    }
    let (Some(canvas), Some(timing)) = (header.canvas(), header.timing) else {
        return;
    };
    let copies = Copies::new(request, diags);
    let looping = request
        .placement
        .looping
        .unwrap_or(timing.looping == Looping::Loop);
    let duration = scene::seconds(timing);
    let first = copies.window(&request.params, 0);
    check_seam(request, first, looping.then_some(duration), diags);
    let tracking = builder.tracking;
    out.tracks
        .extend((tracking && !expand::is_part(node)).then(|| Track {
            scene: name.to_owned(),
            id: request.id.clone(),
            depth: builder.stack.len() - 1,
            count: copies.count,
            start: first.start,
            end: first.end,
            offset: first.offset,
            speed: first.speed,
            duration,
            looping,
        }));
    builder.stack.push(name.to_owned());
    let first_instance = out.world.instances.len();
    for index in 0..copies.count {
        let env = copies.env(index);
        let read = |prop: &str| read(&request.params, prop, &env);
        let seed = if request.seeded {
            whole(read("seed"))
        } else {
            let order = u64::try_from(request.order).unwrap_or(u64::MAX);
            hash_seed(request.parent.seed, &[order, u64::from(index)])
        };
        let instance = Instance {
            parent: request.parent.instance,
            x: prop_of(&request.params, "x").cloned().unwrap_or_default(),
            y: prop_of(&request.params, "y").cloned().unwrap_or_default(),
            scale: prop_of(&request.params, "scale")
                .cloned()
                .unwrap_or(Param::Fixed(1.0)),
            flip: read("flip") > 0.5,
            window: copies.window(&request.params, index),
            duration,
            looping,
            index,
            count: copies.count,
            seed,
            anchor: request.anchor.clone(),
        };
        let scope = Scope {
            instance: Some(out.world.instances.len()),
            seed,
            loop_period: looping.then_some(duration),
            ..request.parent.clone()
        };
        out.world.instances.push(instance);
        let overrides = Overrides::Instance {
            list: &request.placement.overrides,
            file: host_file,
        };
        diags.in_file(file);
        builder.tracking = tracking && index == 0;
        builder.build(node, canvas, &scope, overrides, out, diags);
        diags.in_file(host_file);
    }
    builder.tracking = tracking;
    builder.stack.pop();
    if let Some(id) = &request.id {
        let count = usize::try_from(copies.count).unwrap_or(usize::MAX);
        let parent = &request.parent;
        out.targets
            .placed(parent.instance, id, first_instance, count, parent.layer);
    }
}

fn find<'l>(
    builder: &Builder<'_, 'l>,
    request: &Request,
    diags: &mut Diagnostics,
) -> Option<(usize, &'l KdlNode)> {
    let name = request.placement.scene.as_str();
    let Some(&(file, node)) = builder.catalog.scenes.get(name) else {
        let hint = suggest::hint(name, builder.catalog.scenes.keys().copied());
        diags.error(request.offset, format!("no scene named \"{name}\".{hint}"));
        return None;
    };
    if builder.stack.iter().any(|open| open == name) || builder.stack.len() >= MAX_DEPTH {
        let chain = builder.stack.join(" → ");
        let reason = if builder.stack.len() >= MAX_DEPTH {
            format!("scenes can be placed at most {MAX_DEPTH} deep")
        } else {
            "a scene can't place itself".to_owned()
        };
        diags.error(request.offset, format!("{reason}: {chain} → {name}"));
        return None;
    }
    Some((file, node))
}

struct Copies {
    count: u32,
}

impl Copies {
    fn new(request: &Request, diags: &mut Diagnostics) -> Self {
        let wanted =
            prop_of(&request.params, "count").map_or(1.0, |param| eval(param, &Env::default()));
        let max = u32::try_from(MAX_INSTANCES).unwrap_or(u32::MAX);
        let count = u32::try_from(whole(wanted)).unwrap_or(u32::MAX);
        if count > max {
            diags.error(
                request.offset,
                format!("`count` is {count}; at most {max} scenes can be placed"),
            );
        }
        Self {
            count: count.min(max),
        }
    }

    fn env(&self, index: u32) -> Env {
        Env {
            index: f64::from(index),
            count: f64::from(self.count),
            ..Env::default()
        }
    }

    fn window(&self, params: &[Param], index: u32) -> Window {
        let env = self.env(index);
        Window {
            start: read(params, "start", &env),
            end: read(params, "end", &env),
            offset: read(params, "offset", &env),
            speed: read(params, "speed", &env),
        }
    }
}

fn check_seam(request: &Request, window: Window, own: Option<f64>, diags: &mut Diagnostics) {
    let (Some(host), Some(own)) = (request.parent.loop_period, own) else {
        return;
    };
    if window.start > 0.0 || window.end < host || own <= 0.0 {
        return;
    }
    let loops = host * window.speed / own;
    if (loops - loops.round()).abs() > SEAM_TOLERANCE {
        diags.warning(
            request.offset,
            format!(
                "\"{}\" plays {loops:.2} of its loops per host loop, so the host loop will show a seam; make speed × {host}s a multiple of {own}s",
                request.placement.scene
            ),
        );
    }
}

fn read(params: &[Param], prop: &str, env: &Env) -> f64 {
    prop_of(params, prop).map_or(0.0, |param| eval(param, env))
}

fn prop_of<'p>(params: &'p [Param], name: &str) -> Option<&'p Param> {
    Kind::UseScene
        .prop_index(name)
        .and_then(|index| params.get(index))
}

fn eval(param: &Param, env: &Env) -> f64 {
    param.value(env).unwrap_or(0.0)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded and clamped to the u64 range first"
)]
fn whole(value: f64) -> u64 {
    value.round().clamp(0.0, 1.8e19) as u64
}
