mod motif;
mod part;
mod scatter;
mod slots;
mod substitute;

use std::collections::BTreeMap;

use kdl::{KdlDocument, KdlNode};

use crate::diagnostic::Diagnostics;
use crate::schema::Kind;
use crate::sources::SourceFile;

use motif::Motif;
pub use motif::MotifSignature;
use part::Host;
pub(crate) use part::is_part;
use scatter::Scatter;
use substitute::Env;

const MAX_DEPTH: usize = 8;
const MAX_EMITTED: usize = 65_534;
const MAX_REPEAT: u32 = 4096;
const NOT_REPEATABLE: [&str; 10] = [
    "layer", "pixels", "camera", "ambient", "param", "wind", "timeline", "anim", "scatter", "slot",
];
const DEFINITIONS: [&str; 2] = ["motif", "part"];
const SLOT: &str = "slot";

pub(crate) fn run(
    documents: &mut [Option<KdlDocument>],
    files: &[SourceFile],
    diags: &mut Diagnostics,
) -> Vec<MotifSignature> {
    let motifs = motif::collect(documents, files, diags);
    let mut placed = 0;
    for (file, document) in documents.iter_mut().enumerate() {
        let Some(document) = document else { continue };
        let previous = diags.in_file(file);
        let mut scenes = Vec::new();
        for scene in document
            .nodes_mut()
            .iter_mut()
            .filter(|node| node.name().value() == "scene")
        {
            let host = Host::of(scene);
            let mut expander = Expander {
                motifs: &motifs,
                diags,
                emitted: 0,
                scenes: Vec::new(),
                placed: &mut placed,
            };
            if let Some(children) = scene.children_mut() {
                expander.block(children, Scope::top(&Env::default(), file, &host));
            }
            scenes.append(&mut expander.scenes);
        }
        document.nodes_mut().append(&mut scenes);
        diags.in_file(previous);
    }
    motifs.into_values().map(|motif| motif.signature).collect()
}

#[derive(Clone, Copy)]
struct Call<'e> {
    offset: usize,
    name: &'e str,
    label: &'static str,
}

#[derive(Clone, Copy)]
struct Scope<'e> {
    env: &'e Env,
    file: usize,
    call: Option<Call<'e>>,
    depth: usize,
    host: &'e Host,
    slot: Option<&'e [KdlNode]>,
}

impl<'e> Scope<'e> {
    const fn top(env: &'e Env, file: usize, host: &'e Host) -> Self {
        Self {
            env,
            file,
            call: None,
            depth: 0,
            host,
            slot: None,
        }
    }

    const fn with_env(self, env: &'e Env) -> Self {
        Self { env, ..self }
    }
}

enum Repeat {
    Once,
    Times(u32),
    Invalid,
}

struct Expander<'m, 'd> {
    motifs: &'m BTreeMap<String, Motif>,
    diags: &'d mut Diagnostics,
    emitted: usize,
    scenes: Vec<KdlNode>,
    placed: &'d mut usize,
}

impl Expander<'_, '_> {
    fn block(&mut self, block: &mut KdlDocument, scope: Scope<'_>) {
        let nodes = std::mem::take(block.nodes_mut());
        let mut out = Vec::with_capacity(nodes.len());
        for node in nodes {
            self.node(node, scope, &mut out);
        }
        *block.nodes_mut() = out;
    }

    fn node(&mut self, mut node: KdlNode, scope: Scope<'_>, out: &mut Vec<KdlNode>) {
        let name = node.name().value();
        if DEFINITIONS.contains(&name) {
            let message = format!("`{name}` belongs at the top level of a file");
            self.report(scope, node.span().offset(), message);
            return;
        }
        if name == SLOT {
            match scope.slot {
                Some(filled) => out.extend(filled.iter().cloned()),
                None => self.report(
                    scope,
                    node.span().offset(),
                    "`slot` belongs in the body of a `part`",
                ),
            }
            return;
        }
        if node.name().value() == "scatter" {
            self.scatter(&node, scope, out);
            return;
        }
        match self.take_repeat(&mut node, scope) {
            Repeat::Once => self.emit(node, scope, out),
            Repeat::Invalid => {}
            Repeat::Times(count) => {
                for index in 0..count {
                    let env = scope.env.with_index(index);
                    self.emit(node.clone(), scope.with_env(&env), out);
                }
            }
        }
    }

    fn scatter(&mut self, node: &KdlNode, scope: Scope<'_>, out: &mut Vec<KdlNode>) {
        let scatter = match Scatter::parse(node) {
            Ok(scatter) => scatter,
            Err(message) => {
                self.report(scope, node.span().offset(), message);
                return;
            }
        };
        let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
        for index in 0..scatter.count() {
            let env = scatter.env(scope.env, index);
            for child in children {
                self.node(child.clone(), scope.with_env(&env), out);
            }
        }
    }

    fn take_repeat(&mut self, node: &mut KdlNode, scope: Scope<'_>) -> Repeat {
        let position = node
            .entries()
            .iter()
            .position(|entry| entry.name().is_some_and(|name| name.value() == "repeat"));
        let Some(position) = position else {
            return Repeat::Once;
        };
        let name = node.name().value().to_owned();
        let mut entry = node.entries_mut().remove(position);
        let offset = entry.span().offset();
        if NOT_REPEATABLE.contains(&name.as_str()) {
            self.report(scope, offset, format!("`{name}` cannot take `repeat`"));
            return Repeat::Invalid;
        }
        if let Err(message) = substitute::entry(&mut entry, scope.env) {
            self.report(scope, offset, message);
            return Repeat::Invalid;
        }
        let count = entry
            .value()
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| (1..=MAX_REPEAT).contains(value));
        count.map_or_else(
            || {
                self.report(
                    scope,
                    offset,
                    format!(
                        "`repeat` must be a whole number in 1..={MAX_REPEAT} at load; use `use-scene count=` when a param sets the count"
                    ),
                );
                Repeat::Invalid
            },
            Repeat::Times,
        )
    }

    fn emit(&mut self, mut node: KdlNode, scope: Scope<'_>, out: &mut Vec<KdlNode>) {
        let offset = node.span().offset();
        self.emitted += 1;
        if self.emitted > MAX_EMITTED {
            self.report(
                scope,
                offset,
                format!("this scene expands to more than {MAX_EMITTED} nodes"),
            );
            return;
        }
        if let Err(message) = part::anchor(&mut node) {
            self.report(scope, offset, message);
            return;
        }
        let kind = Kind::parse(node.name().value());
        if let Some(kind) = kind
            && let Err(message) = slots::assign(&mut node, kind)
        {
            self.report(scope, offset, message);
            return;
        }
        if let Err(message) = substitute::entries(&mut node, scope.env) {
            self.report(scope, offset, message);
            return;
        }
        if let Some(kind) = kind
            && let Err(message) = slots::check_bound(&node, kind)
        {
            self.report(scope, offset, message);
            return;
        }
        if self.motifs.contains_key(node.name().value()) {
            self.call(&node, scope, out);
            return;
        }
        if let Some(Kind::Layer | Kind::Group) = kind {
            if let Some(children) = node.children_mut() {
                self.block(children, scope);
            }
        } else if let Err(message) = substitute::descendants(&mut node, scope.env) {
            self.report(scope, offset, message);
            return;
        }
        out.push(node);
    }

    fn call(&mut self, node: &KdlNode, scope: Scope<'_>, out: &mut Vec<KdlNode>) {
        let motifs = self.motifs;
        let name = node.name().value();
        let offset = node.span().offset();
        let Some(motif) = motifs.get(name) else {
            return;
        };
        if scope.depth >= MAX_DEPTH {
            self.report(
                scope,
                offset,
                format!("motifs and parts nest at most {MAX_DEPTH} deep"),
            );
            return;
        }
        let bound = match motif.bind(node) {
            Ok(bound) => bound,
            Err(message) => {
                self.report(scope, offset, message);
                return;
            }
        };
        let part = motif.signature.part;
        let mut filled = Vec::new();
        for child in node.children().map(KdlDocument::nodes).unwrap_or_default() {
            self.node(child.clone(), scope, &mut filled);
        }
        let inner = Scope {
            env: &bound.env,
            file: motif.signature.file,
            call: scope.call.or(Some(Call {
                offset,
                name,
                label: motif.signature.label(),
            })),
            depth: scope.depth + 1,
            host: scope.host,
            slot: part.then_some(&filled),
        };
        let start = out.len();
        let made = self.scenes.len();
        let mut body = Vec::new();
        for child in &motif.body {
            self.node(
                child.clone(),
                inner,
                if part { &mut body } else { &mut *out },
            );
        }
        if part {
            *self.placed += 1;
            let scene = format!("{name}#{}", *self.placed);
            out.push(scope.host.placement(&scene, bound.placement, node));
            self.scenes.push(scope.host.scene(&scene, body, node));
        }
        let foreign = motif.signature.file != scope.file;
        for emitted in out[start..].iter_mut().chain(&mut self.scenes[made..]) {
            respan(emitted, node, foreign);
        }
    }

    fn report(&mut self, scope: Scope<'_>, offset: usize, message: impl Into<String>) {
        let message = message.into();
        match scope.call {
            Some(call) => self.diags.error(
                call.offset,
                format!("in {} `{}`: {message}", call.label, call.name),
            ),
            None => self.diags.error(offset, message),
        }
    }
}

fn respan(node: &mut KdlNode, call: &KdlNode, foreign: bool) {
    node.set_span(call.span());
    if foreign {
        for entry in node.entries_mut() {
            entry.set_span(call.span());
        }
    }
    if let Some(children) = node.children_mut() {
        for child in children.nodes_mut() {
            respan(child, call, foreign);
        }
    }
}
