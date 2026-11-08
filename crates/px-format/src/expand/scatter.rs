use kdl::{KdlEntry, KdlNode, KdlValue};
use px_core::hash_seed;

use super::substitute::{Env, name_length};
use super::{MAX_REPEAT, NOT_REPEATABLE};

const RESERVED: [&str; 3] = ["k", "seed", "repeat"];
const UNIT: f64 = 1.0 / 9_007_199_254_740_992.0;
const USAGE: &str = "scatter 20 seed=1 x=\"0..320\" tone=\"base|hi\" { … }";

pub(super) struct Scatter {
    count: u32,
    seed: u64,
    draws: Vec<Draw>,
}

struct Draw {
    name: String,
    key: u64,
    spread: Spread,
}

enum Spread {
    Whole(i128, i128),
    Between(f64, f64),
    Choice(Vec<KdlValue>),
}

impl Scatter {
    pub(super) fn parse(node: &KdlNode) -> Result<Self, String> {
        let mut count = None;
        let mut seed = 0;
        let mut draws = Vec::new();
        for entry in node.entries() {
            match entry.name().map(kdl::KdlIdentifier::value) {
                None if count.is_none() => count = Some(whole(entry, 1, MAX_REPEAT)?),
                None => return Err(format!("`scatter` takes one count: {USAGE}")),
                Some("seed") => seed = whole(entry, 0, u32::MAX)?,
                Some(name) => draws.push(Draw::parse(name, entry)?),
            }
        }
        let count = count.ok_or_else(|| format!("`scatter` needs a count: {USAGE}"))?;
        let children = node.children().map(kdl::KdlDocument::nodes);
        let children = children.filter(|nodes| !nodes.is_empty());
        let children = children.ok_or_else(|| format!("`scatter` has no body: {USAGE}"))?;
        if let Some(banned) = children
            .iter()
            .map(|child| child.name().value())
            .find(|name| NOT_REPEATABLE.contains(name))
        {
            return Err(format!("`scatter` cannot hold `{banned}`"));
        }
        Ok(Self {
            count,
            seed: u64::from(seed),
            draws,
        })
    }

    pub(super) const fn count(&self) -> u32 {
        self.count
    }

    pub(super) fn env(&self, outer: &Env, index: u32) -> Env {
        let mut env = outer.with_index(index);
        for draw in &self.draws {
            let bits = hash_seed(self.seed, &[draw.key, u64::from(index)]);
            env.bind(draw.name.clone(), draw.spread.value(bits));
        }
        env
    }
}

impl Draw {
    fn parse(name: &str, entry: &KdlEntry) -> Result<Self, String> {
        if RESERVED.contains(&name) || name_length(name) != name.len() {
            return Err(format!("`{name}` cannot be a scatter value"));
        }
        let Some(text) = entry.value().as_string() else {
            return Err(format!(
                "`{name}` needs a range \"a..b\" or choices \"a|b\": {USAGE}"
            ));
        };
        Ok(Self {
            name: name.to_owned(),
            key: key(&format!("{name}={text}")),
            spread: Spread::parse(name, text)?,
        })
    }
}

impl Spread {
    fn parse(name: &str, text: &str) -> Result<Self, String> {
        if let Some((low, high)) = text.split_once("..") {
            let (low, high) = (low.trim(), high.trim());
            if let (Ok(low), Ok(high)) = (low.parse::<i128>(), high.parse::<i128>()) {
                return Ok(Self::Whole(low.min(high), low.max(high)));
            }
            let ends = (low.parse::<f64>(), high.parse::<f64>());
            if let (Ok(low), Ok(high)) = ends
                && low.is_finite()
                && high.is_finite()
            {
                return Ok(Self::Between(low.min(high), low.max(high)));
            }
            return Err(format!("`{name}`: \"{text}\" is not a range of numbers"));
        }
        let options: Vec<&str> = text.split('|').map(str::trim).collect();
        if options.len() < 2 || options.iter().any(|option| option.is_empty()) {
            return Err(format!(
                "`{name}` needs a range \"a..b\" or at least two choices \"a|b\""
            ));
        }
        Ok(Self::Choice(options.into_iter().map(option).collect()))
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "the top 53 bits make an exact unit fraction; spans and choice counts are small"
    )]
    fn value(&self, bits: u64) -> KdlValue {
        match self {
            Self::Whole(low, high) => {
                let span = (high - low + 1).unsigned_abs();
                KdlValue::Integer(low + (u128::from(bits) % span) as i128)
            }
            Self::Between(low, high) => {
                KdlValue::Float(low + (high - low) * ((bits >> 11) as f64 * UNIT))
            }
            Self::Choice(options) => options[(bits % options.len() as u64) as usize].clone(),
        }
    }
}

fn option(text: &str) -> KdlValue {
    text.parse::<i128>().map_or_else(
        |_| {
            text.parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map_or_else(|| KdlValue::String(text.to_owned()), KdlValue::Float)
        },
        KdlValue::Integer,
    )
}

fn whole(entry: &KdlEntry, min: u32, max: u32) -> Result<u32, String> {
    entry
        .value()
        .as_integer()
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| format!("expected a whole number in {min}..={max}"))
}

fn key(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}
