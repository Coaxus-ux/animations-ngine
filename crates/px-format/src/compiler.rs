use kdl::{KdlEntry, KdlValue};
use px_engine::{Param, Shape, object_seed};
use px_expr::{CompileErrorKind, Program, Reference, ResolveError, Resolver};

use crate::diagnostic::Diagnostics;
use crate::params::SceneParams;
use crate::shapes::{self, Draft, PROPERTIES};
use crate::units::{self, Literal, Measure, Quantity, Unit};
use crate::values;

const MAX_REFERENCE_DEPTH: usize = 64;

#[derive(Clone)]
enum State {
    Pending,
    InProgress,
    Done(Param),
    Failed,
}

enum Failure {
    Cycle(String),
    Failed,
    TooDeep,
}

type Key = (usize, usize);

pub(crate) struct Compiler<'a, 'd> {
    drafts: &'a [Draft<'a>],
    params: &'a SceneParams,
    measure: Measure,
    scene_seed: u64,
    states: Vec<[State; PROPERTIES]>,
    chain: Vec<Key>,
    diags: &'d mut Diagnostics,
}

impl<'a, 'd> Compiler<'a, 'd> {
    pub(crate) fn new(
        drafts: &'a [Draft<'a>],
        params: &'a SceneParams,
        measure: Measure,
        scene_seed: u64,
        diags: &'d mut Diagnostics,
    ) -> Self {
        let states = drafts
            .iter()
            .map(|_| [const { State::Pending }; PROPERTIES])
            .collect();
        Self {
            drafts,
            params,
            measure,
            scene_seed,
            states,
            chain: Vec::new(),
            diags,
        }
    }

    pub(crate) fn shapes(mut self) -> Option<Vec<Shape>> {
        let mut shapes = Vec::with_capacity(self.drafts.len());
        let mut complete = true;
        for (object, draft) in self.drafts.iter().enumerate() {
            let params: [Option<Param>; PROPERTIES] =
                std::array::from_fn(|property| self.property((object, property)).ok());
            match (params, draft.paint) {
                ([Some(a), Some(b), Some(c), Some(d)], Some(paint)) => {
                    shapes.push(shapes::build(draft.kind, paint, [a, b, c, d]));
                }
                _ => complete = false,
            }
        }
        complete.then_some(shapes)
    }

    fn property(&mut self, key: Key) -> Result<Param, Failure> {
        match &self.states[key.0][key.1] {
            State::Done(param) => return Ok(param.clone()),
            State::Failed => return Err(Failure::Failed),
            State::InProgress => return Err(Failure::Cycle(self.cycle(key))),
            State::Pending if self.chain.len() >= MAX_REFERENCE_DEPTH => {
                return Err(Failure::TooDeep);
            }
            State::Pending => {}
        }
        self.states[key.0][key.1] = State::InProgress;
        self.chain.push(key);
        let compiled = self.compile(key);
        self.chain.pop();
        self.states[key.0][key.1] = compiled.clone().map_or(State::Failed, State::Done);
        compiled.ok_or(Failure::Failed)
    }

    fn compile(&mut self, key: Key) -> Option<Param> {
        let draft = &self.drafts[key.0];
        let entry = draft.entries[key.1]?;
        let (name, axis) = draft.kind.properties()[key.1];
        let quantity = Quantity::Length(axis);
        let text = match entry.value() {
            KdlValue::String(text) => text.as_str(),
            _ => return values::number(entry, self.diags).map(Param::Fixed),
        };
        if let Some(source) = text.strip_prefix('=') {
            return self.expression(key, entry, source);
        }
        let result = match text.split_once("..") {
            Some((low, high)) => self.range(low, high, quantity, name),
            None => units::literal(text)
                .ok_or_else(|| {
                    format!("expected a number, \"=expression\" or \"a..b\", found \"{text}\"")
                })
                .and_then(|literal| literal.convert(quantity, self.measure).map(Param::Fixed)),
        };
        result
            .map_err(|message| {
                self.diags
                    .error(entry.span().offset(), format!("`{name}`: {message}"));
            })
            .ok()
    }

    fn range(
        &self,
        low: &str,
        high: &str,
        quantity: Quantity,
        name: &str,
    ) -> Result<Param, String> {
        let parse = |text: &str| {
            units::literal(text).ok_or_else(|| format!("invalid range end \"{text}\""))
        };
        let (low, high): (Literal, Literal) = (parse(low)?, parse(high)?);
        if low.unit != high.unit {
            return Err("both ends of a range need the same unit".to_owned());
        }
        let integer = low.integer && high.integer && matches!(low.unit, Unit::None | Unit::Px);
        let (low, high) = (
            low.convert(quantity, self.measure)?,
            high.convert(quantity, self.measure)?,
        );
        Ok(Param::Expr(Program::random_range(
            low.min(high),
            low.max(high),
            integer,
            range_key(name),
        )))
    }

    fn expression(&mut self, key: Key, entry: &KdlEntry, source: &str) -> Option<Param> {
        let mut lookup = Lookup {
            compiler: self,
            object: key.0,
            detail: None,
        };
        let compiled = px_expr::compile_with(source, &mut lookup);
        let detail = lookup.detail;
        match compiled {
            Ok(program) => Some(Param::Expr(program)),
            Err(error) if error.kind == CompileErrorKind::Resolve(ResolveError::Failed) => None,
            Err(error) => {
                let snippet = source.get(error.start..error.end).unwrap_or("");
                let message = detail.unwrap_or_else(|| {
                    format!(
                        "{} (`{snippet}` at character {})",
                        error.kind,
                        error.start + 1
                    )
                });
                self.diags.error(
                    entry.span().offset(),
                    format!("invalid expression `{source}`: {message}"),
                );
                None
            }
        }
    }

    fn label(&self, (object, property): Key) -> String {
        let draft = &self.drafts[object];
        let name = draft.kind.properties()[property].0;
        draft
            .id
            .map_or_else(|| format!("#{object}.{name}"), |id| format!("{id}.{name}"))
    }

    fn cycle(&self, key: Key) -> String {
        let start = self.chain.iter().position(|item| *item == key).unwrap_or(0);
        let path: Vec<String> = self.chain[start..]
            .iter()
            .chain([&key])
            .map(|item| self.label(*item))
            .collect();
        format!("reference cycle: {}", path.join(" → "))
    }

    fn find(&self, object: usize, path: &str) -> Result<Key, (ResolveError, String)> {
        let (id, property) = path.split_once('.').ok_or_else(|| {
            (
                ResolveError::Unsupported,
                format!("`@{path}` needs a property, like `@{path}.x`"),
            )
        })?;
        if property.contains('.') || id == "parent" {
            return Err((
                ResolveError::Unsupported,
                format!("`@{path}` is not available yet (no prefabs or parents)"),
            ));
        }
        let target = if id == "self" {
            Some(object)
        } else {
            self.drafts.iter().position(|draft| draft.id == Some(id))
        };
        let Some(target) = target else {
            let known: Vec<&str> = self.drafts.iter().filter_map(|draft| draft.id).collect();
            return Err((
                ResolveError::UnknownReference,
                format!("no object with id \"{id}\" (ids: {})", known.join(", ")),
            ));
        };
        let kind = self.drafts[target].kind;
        let index = kind.property_index(property).ok_or_else(|| {
            let names: Vec<&str> = kind.properties().iter().map(|(name, _)| *name).collect();
            (
                ResolveError::UnknownReference,
                format!(
                    "`{id}` has no property `{property}` (has: {})",
                    names.join(", ")
                ),
            )
        })?;
        Ok((target, index))
    }
}

struct Lookup<'c, 'a, 'd> {
    compiler: &'c mut Compiler<'a, 'd>,
    object: usize,
    detail: Option<String>,
}

impl Lookup<'_, '_, '_> {
    fn fail(&mut self, (error, message): (ResolveError, String)) -> ResolveError {
        self.detail = Some(message);
        error
    }
}

impl Resolver for Lookup<'_, '_, '_> {
    fn param(&mut self, name: &str) -> Result<f64, ResolveError> {
        self.compiler
            .params
            .number(name)
            .map_err(|failure| self.fail(failure))
    }

    fn reference(&mut self, path: &str) -> Result<Reference, ResolveError> {
        let key = self
            .compiler
            .find(self.object, path)
            .map_err(|failure| self.fail(failure))?;
        match self.compiler.property(key) {
            Ok(param) => Ok(Reference {
                program: program_of(param),
                seed: object_seed(self.compiler.scene_seed, key.0),
            }),
            Err(Failure::Failed) => Err(ResolveError::Failed),
            Err(Failure::Cycle(chain)) => Err(self.fail((ResolveError::Cycle, chain))),
            Err(Failure::TooDeep) => Err(self.fail((
                ResolveError::TooDeep,
                format!("reference chain is deeper than {MAX_REFERENCE_DEPTH}"),
            ))),
        }
    }
}

fn program_of(param: Param) -> Program {
    match param {
        Param::Fixed(value) => Program::fixed(value),
        Param::Expr(program) => program,
    }
}

fn range_key(name: &str) -> f64 {
    let hash = name.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    f64::from(hash)
}
