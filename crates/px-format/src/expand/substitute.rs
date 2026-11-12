use kdl::{KdlEntry, KdlNode, KdlValue};

use crate::units::{self, Unit};

const LARGEST_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

#[derive(Default)]
pub(super) struct Env(Vec<(String, KdlValue)>);

impl Env {
    pub(super) const fn new(bindings: Vec<(String, KdlValue)>) -> Self {
        Self(bindings)
    }

    pub(super) fn with_index(&self, index: u32) -> Self {
        let mut bindings = self.0.clone();
        bindings.push(("k".to_owned(), KdlValue::Integer(i128::from(index))));
        Self(bindings)
    }

    pub(super) fn bind(&mut self, name: String, value: KdlValue) {
        self.0.push((name, value));
    }

    fn lookup(&self, name: &str) -> Option<&KdlValue> {
        self.0
            .iter()
            .rev()
            .find(|(bound, _)| bound == name)
            .map(|(_, value)| value)
    }

    const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) fn entries(node: &mut KdlNode, env: &Env) -> Result<(), String> {
    if env.is_empty() {
        return Ok(());
    }
    node.entries_mut()
        .iter_mut()
        .try_for_each(|item| entry(item, env))
}

pub(super) fn descendants(node: &mut KdlNode, env: &Env) -> Result<(), String> {
    if env.is_empty() {
        return Ok(());
    }
    let Some(children) = node.children_mut() else {
        return Ok(());
    };
    children.nodes_mut().iter_mut().try_for_each(|child| {
        entries(child, env)?;
        descendants(child, env)
    })
}

pub(super) fn entry(entry: &mut KdlEntry, env: &Env) -> Result<(), String> {
    let KdlValue::String(text) = entry.value() else {
        return Ok(());
    };
    if let Some(value) = value(text, env)? {
        entry.set_value(value);
    }
    Ok(())
}

fn value(text: &str, env: &Env) -> Result<Option<KdlValue>, String> {
    if !text.contains('$') {
        return Ok(None);
    }
    if let Some(bound) = whole_name(text).and_then(|name| env.lookup(name)) {
        return Ok(Some(bound.clone()));
    }
    let Some(source) = text.strip_prefix('=') else {
        return Ok(rewrite(text, env, |value| Ok(in_text(value)))?.map(KdlValue::String));
    };
    let rewritten = rewrite(source, env, in_expression)?;
    Ok(rewritten
        .map(|source| fold(&source).unwrap_or_else(|| KdlValue::String(format!("={source}")))))
}

fn whole_name(text: &str) -> Option<&str> {
    let name = text.strip_prefix('$')?;
    (!name.is_empty() && name_length(name) == name.len()).then_some(name)
}

pub(super) fn name_length(text: &str) -> usize {
    let bytes = text.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_lowercase) {
        return 0;
    }
    let mut end = 1;
    while let Some(&byte) = bytes.get(end) {
        let continues = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || byte == b'_'
            || (byte == b'-' && bytes.get(end + 1).is_some_and(u8::is_ascii_lowercase));
        if !continues {
            break;
        }
        end += 1;
    }
    end
}

fn rewrite(
    text: &str,
    env: &Env,
    render: impl Fn(&KdlValue) -> Result<String, String>,
) -> Result<Option<String>, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut touched = false;
    while let Some(dollar) = rest.find('$') {
        let after = &rest[dollar + 1..];
        let length = name_length(after);
        out.push_str(&rest[..dollar]);
        match env.lookup(&after[..length]) {
            Some(bound) if length > 0 => {
                out.push_str(&render(bound).map_err(|reason| {
                    format!(
                        "`${}` cannot go in an expression: {reason}",
                        &after[..length]
                    )
                })?);
                touched = true;
            }
            _ => {
                out.push('$');
                out.push_str(&after[..length]);
            }
        }
        rest = &after[length..];
    }
    out.push_str(rest);
    Ok(touched.then_some(out))
}

fn in_text(value: &KdlValue) -> String {
    match value {
        KdlValue::String(text) => text.clone(),
        KdlValue::Integer(number) => number.to_string(),
        KdlValue::Float(number) => number.to_string(),
        KdlValue::Bool(flag) => flag.to_string(),
        KdlValue::Null => String::new(),
    }
}

fn in_expression(value: &KdlValue) -> Result<String, String> {
    match value {
        KdlValue::Integer(number) if *number < 0 => Ok(format!("({number})")),
        KdlValue::Integer(number) => Ok(number.to_string()),
        KdlValue::Float(number) if number.is_sign_negative() => Ok(format!("({number})")),
        KdlValue::Float(number) => Ok(number.to_string()),
        KdlValue::String(text) => {
            if let Some(source) = text.strip_prefix('=') {
                Ok(format!("({source})"))
            } else if whole_name(text).is_some() || is_time(text) {
                Ok(text.clone())
            } else {
                Err(format!("\"{text}\" is not a number, `=` value or time"))
            }
        }
        KdlValue::Bool(_) | KdlValue::Null => Err("only numbers fit here".to_owned()),
    }
}

fn is_time(text: &str) -> bool {
    units::literal(text)
        .is_some_and(|literal| matches!(literal.unit, Unit::Seconds | Unit::Millis | Unit::Frames))
}

fn fold(source: &str) -> Option<KdlValue> {
    px_expr::compile(source)
        .ok()?
        .arithmetic_value()
        .map(number_value)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "only whole numbers below 2^53 are converted"
)]
fn number_value(number: f64) -> KdlValue {
    if number.fract() == 0.0 && number.abs() < LARGEST_EXACT_INTEGER {
        KdlValue::Integer(number as i128)
    } else {
        KdlValue::Float(number)
    }
}
