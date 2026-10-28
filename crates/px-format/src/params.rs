use std::collections::BTreeMap;

use kdl::{KdlNode, KdlValue};
use px_expr::ResolveError;

use crate::diagnostic::Diagnostics;
use crate::props::Props;
use crate::suggest;
use crate::values;

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Number(f64),
    Bool(bool),
    Text(String),
}

#[derive(Clone, Debug)]
struct Param {
    value: ParamValue,
    min: Option<f64>,
    max: Option<f64>,
    options: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SceneParams {
    params: BTreeMap<String, Param>,
}

impl SceneParams {
    pub(crate) fn add(&mut self, node: &KdlNode, diags: &mut Diagnostics) {
        let mut props = Props::new(node);
        let args: Vec<_> = props.args().collect();
        let name = args.first().and_then(|entry| values::string(entry, diags));
        let default = args.get(1).map(|entry| (*entry, value_of(entry.value())));
        let min = props
            .get("min")
            .and_then(|entry| values::number(entry, diags));
        let max = props
            .get("max")
            .and_then(|entry| values::number(entry, diags));
        let options = props
            .get("options")
            .and_then(|entry| values::string(entry, diags));
        for key in ["step", "label", "group", "kind"] {
            props.get(key);
        }
        props.finish(diags);
        let Some(name) = name else { return };
        let Some((entry, Some(value))) = default else {
            diags.error(
                node.span().offset(),
                format!("param `{name}` needs a default value"),
            );
            return;
        };
        let options = options
            .map(|text| text.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        let param = Param {
            value,
            min,
            max,
            options,
        };
        if let Err(message) = param.check(&param.value) {
            diags.error(entry.span().offset(), format!("param `{name}`: {message}"));
        } else if self.params.insert(name.to_owned(), param).is_some() {
            diags.error(
                node.span().offset(),
                format!("param `{name}` is defined twice"),
            );
        }
    }

    pub(crate) fn apply_overrides(
        &mut self,
        overrides: &[(String, String)],
        diags: &mut Diagnostics,
    ) {
        for (name, text) in overrides {
            let Some(param) = self.params.get_mut(name) else {
                let hint = suggest::hint(name, self.params.keys().map(String::as_str));
                diags.error(0, format!("--param {name}: unknown param.{hint}"));
                continue;
            };
            let parsed = match param.value {
                ParamValue::Number(_) => text.parse().ok().map(ParamValue::Number),
                ParamValue::Bool(_) => text.parse().ok().map(ParamValue::Bool),
                ParamValue::Text(_) => Some(ParamValue::Text(text.clone())),
            };
            match parsed
                .ok_or_else(|| format!("\"{text}\" has the wrong type"))
                .and_then(|value| {
                    param.check(&value)?;
                    Ok(value)
                }) {
                Ok(value) => param.value = value,
                Err(message) => diags.error(0, format!("--param {name}: {message}")),
            }
        }
    }

    /// Sets a param from a `use-scene` property; the value must have the param's type.
    pub(crate) fn set(&mut self, name: &str, value: ParamValue) -> Result<(), String> {
        let Some(param) = self.params.get_mut(name) else {
            let hint = suggest::hint(name, self.params.keys().map(String::as_str));
            return Err(format!("the placed scene has no param `{name}`.{hint}"));
        };
        let same_type = matches!(
            (&param.value, &value),
            (ParamValue::Number(_), ParamValue::Number(_))
                | (ParamValue::Bool(_), ParamValue::Bool(_))
                | (ParamValue::Text(_), ParamValue::Text(_))
        );
        if !same_type {
            return Err(format!("param `{name}` has a different type"));
        }
        param
            .check(&value)
            .map_err(|message| format!("param `{name}`: {message}"))?;
        param.value = value;
        Ok(())
    }

    pub(crate) fn text(&self, name: &str) -> Result<&str, String> {
        match self.params.get(name).map(|param| &param.value) {
            Some(ParamValue::Text(text)) => Ok(text),
            Some(_) => Err(format!(
                "param `${name}` is not text, so it can't name a paint"
            )),
            None => {
                let hint = suggest::hint(name, self.params.keys().map(String::as_str));
                Err(format!("unknown param `${name}`.{hint}"))
            }
        }
    }

    pub(crate) fn number(&self, name: &str) -> Result<f64, (ResolveError, String)> {
        match self.params.get(name).map(|param| &param.value) {
            Some(ParamValue::Number(value)) => Ok(*value),
            Some(ParamValue::Bool(value)) => Ok(if *value { 1.0 } else { 0.0 }),
            Some(ParamValue::Text(_)) => Err((
                ResolveError::NotNumeric,
                format!("param `${name}` is text; expressions need numbers"),
            )),
            None => {
                let hint = suggest::hint(name, self.params.keys().map(String::as_str));
                Err((
                    ResolveError::UnknownParam,
                    format!("unknown param `${name}`.{hint}"),
                ))
            }
        }
    }
}

impl Param {
    fn check(&self, value: &ParamValue) -> Result<(), String> {
        match value {
            ParamValue::Number(number) => {
                if self.min.is_some_and(|min| *number < min)
                    || self.max.is_some_and(|max| *number > max)
                {
                    let (min, max) = (
                        self.min.unwrap_or(f64::NEG_INFINITY),
                        self.max.unwrap_or(f64::INFINITY),
                    );
                    return Err(format!("{number} is outside {min}..={max}"));
                }
            }
            ParamValue::Text(text) if !self.options.is_empty() && !self.options.contains(text) => {
                return Err(format!(
                    "\"{text}\" is not one of: {}",
                    self.options.join(", ")
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

pub(crate) fn value_of(value: &KdlValue) -> Option<ParamValue> {
    match value {
        KdlValue::Integer(number) => Some(ParamValue::Number(values::integer_to_f64(*number))),
        KdlValue::Float(number) if number.is_finite() => Some(ParamValue::Number(*number)),
        KdlValue::Bool(flag) => Some(ParamValue::Bool(*flag)),
        KdlValue::String(text) => Some(ParamValue::Text(text.clone())),
        _ => None,
    }
}
