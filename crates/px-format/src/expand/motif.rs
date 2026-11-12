use std::collections::BTreeMap;
use std::fmt::Write as _;

use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

use super::part::{self, PLACED};
use super::substitute::{Env, name_length};
use crate::diagnostic::Diagnostics;
use crate::schema::Kind;
use crate::sources::SourceFile;
use crate::suggest;

const RESERVED: [&str; 18] = [
    "use", "palette", "font", "scene", "sequence", "motif", "param", "wind", "timeline", "anim",
    "marker", "k", "legend", "cel", "scatter", "part", "slot", "anchor",
];
const NOT_IN_BODY: [&str; 10] = [
    "layer", "pixels", "camera", "ambient", "param", "wind", "timeline", "anim", "motif", "part",
];
const DEFINITIONS: [&str; 2] = ["motif", "part"];
const SLOT: &str = "slot";
const RESERVED_PARAMS: [&str; 2] = ["k", "repeat"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotifSignature {
    pub name: String,
    pub required: Vec<String>,
    pub optional: Vec<String>,
    pub part: bool,
    pub file: usize,
    pub offset: usize,
}

impl MotifSignature {
    pub fn positional(&self) -> impl Iterator<Item = &str> {
        let placed: &[&str] = if self.part { &PLACED } else { &[] };
        let required = self
            .required
            .iter()
            .map(String::as_str)
            .filter(|name| *name != "paint");
        placed.iter().copied().chain(required)
    }

    #[must_use]
    pub const fn label(&self) -> &'static str {
        if self.part { "part" } else { "motif" }
    }

    #[must_use]
    pub fn declares_paint(&self) -> bool {
        self.params().any(|name| name == "paint")
    }

    fn params(&self) -> impl Iterator<Item = &str> {
        self.required
            .iter()
            .chain(&self.optional)
            .map(String::as_str)
    }

    fn usage(&self) -> String {
        let mut usage = self.name.clone();
        if self.part {
            usage.push_str(" [x y]");
        }
        for name in &self.required {
            usage.push(' ');
            usage.push_str(name);
        }
        for name in &self.optional {
            let _ = write!(usage, " [{name}=…]");
        }
        usage
    }
}

pub(super) struct Motif {
    pub(super) signature: MotifSignature,
    defaults: Vec<(String, KdlValue)>,
    pub(super) body: Vec<KdlNode>,
    has_slot: bool,
}

pub(super) struct Bound {
    pub(super) env: Env,
    pub(super) placement: Vec<KdlEntry>,
}

impl Motif {
    pub(super) fn bind(&self, call: &KdlNode) -> Result<Bound, String> {
        let signature = &self.signature;
        let usage = signature.usage();
        if call.children().is_some() && !self.has_slot {
            return Err(if signature.part {
                format!("this part has no `slot` for children: {usage}")
            } else {
                format!("a motif call takes no children: {usage}")
            });
        }
        let mut given: BTreeMap<&str, &KdlValue> = BTreeMap::new();
        let mut placement = Vec::new();
        let mut positional = signature.positional();
        for entry in call.entries() {
            let target = match entry.name() {
                Some(name) => name.value(),
                None => positional
                    .next()
                    .ok_or_else(|| format!("too many bare values: {usage}"))?,
            };
            let placed = signature.part && part::PLACEMENT.contains(&target);
            if !placed && !signature.params().any(|name| name == target) {
                let hint = suggest::did_you_mean(target, signature.params());
                return Err(format!("no param `{target}`: {usage}.{hint}"));
            }
            if given.insert(target, entry.value()).is_some() {
                return Err(format!("`{target}` is given twice: {usage}"));
            }
            if placed {
                let mut entry = entry.clone();
                entry.set_name(Some(target));
                placement.push(entry);
            }
        }
        let mut bindings = Vec::with_capacity(signature.required.len() + self.defaults.len());
        for name in &signature.required {
            let value = given
                .get(name.as_str())
                .ok_or_else(|| format!("missing `{name}`: {usage}"))?;
            bindings.push((name.clone(), (*value).clone()));
        }
        for (name, default) in &self.defaults {
            let value = given.get(name.as_str()).copied().unwrap_or(default);
            bindings.push((name.clone(), value.clone()));
        }
        Ok(Bound {
            env: Env::new(bindings),
            placement,
        })
    }
}

pub(super) fn collect(
    documents: &mut [Option<KdlDocument>],
    files: &[SourceFile],
    diags: &mut Diagnostics,
) -> BTreeMap<String, Motif> {
    let mut motifs: BTreeMap<String, Motif> = BTreeMap::new();
    for (file, document) in documents.iter_mut().enumerate() {
        let Some(document) = document else { continue };
        let (found, rest): (Vec<KdlNode>, Vec<KdlNode>) = std::mem::take(document.nodes_mut())
            .into_iter()
            .partition(|node| DEFINITIONS.contains(&node.name().value()));
        *document.nodes_mut() = rest;
        let previous = diags.in_file(file);
        for node in found {
            let Some(motif) = parse(&node, file, diags) else {
                continue;
            };
            if let Some(first) = motifs.get(&motif.signature.name) {
                let path = files.get(first.signature.file).map_or_else(
                    || "another file".to_owned(),
                    |source| source.path.display().to_string(),
                );
                diags.error(
                    motif.signature.offset,
                    format!(
                        "{} `{}` is defined twice; it is also in {path}. Motif and part names are global: share the file or add a family prefix",
                        motif.signature.label(),
                        motif.signature.name
                    ),
                );
                continue;
            }
            motifs.insert(motif.signature.name.clone(), motif);
        }
        diags.in_file(previous);
    }
    motifs
}

fn parse(node: &KdlNode, file: usize, diags: &mut Diagnostics) -> Option<Motif> {
    let offset = node.span().offset();
    let part = node.name().value() == "part";
    let what = if part { "part" } else { "motif" };
    let mut positional = node
        .entries()
        .iter()
        .filter(|entry| entry.name().is_none())
        .map(KdlEntry::value);
    let Some(name) = positional.next().and_then(KdlValue::as_string) else {
        diags.error(
            offset,
            format!("`{what}` needs a name: {what} pier {{ … }}"),
        );
        return None;
    };
    if Kind::parse(name).is_some() || RESERVED.contains(&name) {
        diags.error(
            offset,
            format!("{what} name `{name}` is taken by the format"),
        );
        return None;
    }
    let name = name.to_owned();
    let mut required = Vec::new();
    for value in positional {
        let Some(param) = value.as_string() else {
            diags.error(offset, format!("{what} `{name}`: params are bare names"));
            return None;
        };
        required.push(param.to_owned());
    }
    let defaults: Vec<(String, KdlValue)> = node
        .entries()
        .iter()
        .filter_map(|entry| Some((entry.name()?.value().to_owned(), entry.value().clone())))
        .collect();
    let invalid = required
        .iter()
        .chain(defaults.iter().map(|(param, _)| param))
        .find(|param| {
            RESERVED_PARAMS.contains(&param.as_str())
                || (part && part::PLACEMENT.contains(&param.as_str()))
                || name_length(param) != param.len()
        });
    if let Some(param) = invalid {
        diags.error(
            offset,
            format!("{what} `{name}`: `{param}` cannot be a param name"),
        );
        return None;
    }
    let body = node
        .children()
        .map(|children| children.nodes().to_vec())
        .unwrap_or_default();
    if body.is_empty() {
        diags.error(offset, format!("{what} `{name}` has no body"));
        return None;
    }
    if let Some(banned) = banned_in_body(&body, part) {
        diags.error(
            offset,
            format!("{what} `{name}` cannot hold `{banned}` in its body"),
        );
        return None;
    }
    let params: Vec<&str> = required
        .iter()
        .chain(defaults.iter().map(|(param, _)| param))
        .map(String::as_str)
        .collect();
    warn_bare_params(&body, &params, diags);
    let optional = defaults.iter().map(|(param, _)| param.clone()).collect();
    Some(Motif {
        signature: MotifSignature {
            name,
            required,
            optional,
            part,
            file,
            offset,
        },
        defaults,
        has_slot: part && holds_slot(&body),
        body,
    })
}

fn holds_slot(nodes: &[KdlNode]) -> bool {
    nodes.iter().any(|node| {
        node.name().value() == SLOT
            || node
                .children()
                .is_some_and(|children| holds_slot(children.nodes()))
    })
}

fn banned_in_body(nodes: &[KdlNode], part: bool) -> Option<String> {
    nodes.iter().find_map(|node| {
        let name = node.name().value();
        if NOT_IN_BODY.contains(&name) || (!part && name == SLOT) {
            return Some(name.to_owned());
        }
        let children = node.children().map(KdlDocument::nodes).unwrap_or_default();
        let inner: Vec<KdlNode> = children
            .iter()
            .filter(|child| child.name().value() != "anim")
            .cloned()
            .collect();
        banned_in_body(&inner, part)
    })
}

fn warn_bare_params(nodes: &[KdlNode], params: &[&str], diags: &mut Diagnostics) {
    for node in nodes {
        for entry in node.entries() {
            let source = entry
                .value()
                .as_string()
                .and_then(|text| text.strip_prefix('='));
            if let Some(param) = source.and_then(|source| bare_param(source, params)) {
                diags.warning(
                    entry.span().offset(),
                    format!(
                        "`{param}` here is the engine's variable; write `${param}` for the param"
                    ),
                );
            }
        }
        if let Some(children) = node.children() {
            warn_bare_params(children.nodes(), params, diags);
        }
    }
}

fn bare_param<'p>(source: &str, params: &[&'p str]) -> Option<&'p str> {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let length = name_length(&source[index..]);
        if length == 0 {
            index += source[index..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        let token = &source[index..index + length];
        let attached = index > 0
            && (matches!(bytes[index - 1], b'$' | b'@' | b'.' | b'_')
                || bytes[index - 1].is_ascii_alphanumeric());
        if !attached && let Some(param) = params.iter().find(|param| **param == token) {
            return Some(param);
        }
        index += length;
    }
    None
}
