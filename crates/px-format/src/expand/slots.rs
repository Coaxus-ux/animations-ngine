use kdl::{KdlNode, KdlValue};

use crate::schema::Kind;

pub(super) fn assign(node: &mut KdlNode, kind: Kind) -> Result<(), String> {
    let slots: Vec<&'static str> = kind.coordinates().collect();
    if slots.is_empty() {
        return Ok(());
    }
    let positional: Vec<usize> = node
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.name().is_none())
        .map(|(index, _)| index)
        .collect();
    let leading_is_named =
        positional
            .first()
            .is_some_and(|&index| match node.entries()[index].value() {
                KdlValue::String(text) => !kind.leading_is_coordinate(text),
                _ => false,
            });
    let values = &positional[usize::from(leading_is_named)..];
    if values.len() > slots.len() {
        return Err(format!(
            "too many bare values: `{}` takes {}",
            node.name().value(),
            slots.join(" ")
        ));
    }
    for (&index, slot) in values.iter().zip(&slots) {
        let named = node
            .entries()
            .iter()
            .any(|entry| entry.name().is_some_and(|name| name.value() == *slot));
        if named {
            return Err(format!("`{slot}` is given both bare and by name"));
        }
        node.entries_mut()[index].set_name(Some(*slot));
    }
    Ok(())
}

pub(super) fn check_bound(node: &KdlNode, kind: Kind) -> Result<(), String> {
    for slot in kind.coordinates() {
        let value = node
            .entries()
            .iter()
            .find(|entry| entry.name().is_some_and(|name| name.value() == slot))
            .and_then(|entry| entry.value().as_string());
        if let Some(text) = value.filter(|text| text.starts_with('$')) {
            return Err(format!(
                "`{slot}` cannot read `{text}` bare; write \"={text}\""
            ));
        }
    }
    Ok(())
}
