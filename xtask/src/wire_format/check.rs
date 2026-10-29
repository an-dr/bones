//! Checks the specification is complete in itself and agrees with the
//! conformance vectors, which `bones-messages` generates from the codec.

use std::collections::{HashMap, HashSet};

use super::model::{Entry, Field, FieldType, Payload, Spec};
use super::vector::Vector;
use crate::Result;

/// Fails with every problem found: a type that names nothing, a count that
/// is not an earlier `u16`, a duplicate tag, a vector for a topic the
/// specification lacks, or field names that differ from a vector's.
pub fn check(spec: &Spec, vectors: &HashMap<String, Vec<Vector>>) -> Result {
    let mut problems = Vec::new();
    for entry in spec.entries() {
        let at = entry.names().cloned().collect::<Vec<_>>().join(", ");
        let mut report = |problem: String| problems.push(format!("{at}: {problem}"));
        check_payload(spec, entry, &entry.payload, &mut report);
        for ty in &entry.types {
            check_payload(spec, entry, &ty.payload, &mut report);
        }
        check_vectors(entry, vectors, &mut report);
    }
    let named: HashSet<&String> = spec.entries().flat_map(Entry::names).collect();
    let mut unknown: Vec<&String> = vectors.keys().filter(|t| !named.contains(t)).collect();
    unknown.sort();
    for topic in unknown {
        problems.push(format!(
            "{topic}: has conformance vectors but no entry in wire-format.json"
        ));
    }
    match problems.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "wit/wire-format.json is inconsistent:\n  {}",
            problems.join("\n  ")
        )),
    }
}

fn check_payload(spec: &Spec, entry: &Entry, payload: &Payload, report: &mut impl FnMut(String)) {
    check_fields(spec, entry, &payload.fields, report);
    if let Some(union) = &payload.union {
        let mut seen = HashSet::new();
        for variant in &union.variants {
            if !seen.insert(variant.tag) {
                report(format!("tag {} is defined twice", variant.tag));
            }
            check_fields(spec, entry, &variant.fields, report);
        }
    }
}

fn check_fields(spec: &Spec, entry: &Entry, fields: &[Field], report: &mut impl FnMut(String)) {
    let resolves = |name: &str| spec.is_primitive(name) || entry.local_type(name).is_some();
    for (index, field) in fields.iter().enumerate() {
        match &field.ty {
            FieldType::Named(name) if !resolves(name) => {
                report(format!("field '{}' has unknown type '{name}'", field.name))
            }
            FieldType::List(list) => {
                if !resolves(&list.list) {
                    report(format!(
                        "field '{}' lists unknown type '{}'",
                        field.name, list.list
                    ));
                }
                let counted = fields[..index].iter().any(|earlier| {
                    earlier.name == list.count && earlier.ty == FieldType::Named("u16".into())
                });
                if !counted {
                    report(format!(
                        "field '{}' is counted by '{}', which is not an earlier u16 field",
                        field.name, list.count
                    ));
                }
            }
            FieldType::Named(_) => {}
        }
        if !field.values.is_empty() && field.ty != FieldType::Named("u8".into()) {
            report(format!("field '{}' has values but is not a u8", field.name));
        }
    }
}

/// Compares the entry's field names with each of its vectors, in order.
///
/// - A count is framing rather than a value, so a vector may list it or
///   leave it out.
/// - A union's fields follow its tag, for the variant the vector's tag
///   names.
/// - An entry that names topics is checked against their vectors only; an
///   endpoint sharing it has a request shape of its own, given in prose.
fn check_vectors(
    entry: &Entry,
    vectors: &HashMap<String, Vec<Vector>>,
    report: &mut impl FnMut(String),
) {
    let checked = match entry.topics.is_empty() {
        true => &entry.endpoints,
        false => &entry.topics,
    };
    let counts = count_fields(entry);
    for topic in checked {
        for vector in vectors.get(topic).into_iter().flatten() {
            let actual: Vec<&str> = vector
                .values
                .iter()
                .map(|(name, _)| name.as_str())
                .filter(|name| !counts.contains(name))
                .collect();
            match wire_names(entry, &entry.payload, vector) {
                Ok(expected) if expected == actual => {}
                Ok(expected) => report(format!(
                    "{topic}: the specification lists [{}], the vector has [{}]",
                    expected.join(", "),
                    actual.join(", ")
                )),
                Err(problem) => report(format!("{topic}: {problem}")),
            }
        }
    }
}

/// The value names a vector lists for `payload`, counts left out.
fn wire_names<'a>(
    entry: &'a Entry,
    payload: &'a Payload,
    vector: &Vector,
) -> std::result::Result<Vec<&'a str>, String> {
    let mut names = Vec::new();
    push_fields(entry, &payload.fields, &mut names);
    if let Some(union) = &payload.union {
        names.push(union.tag.as_str());
        let tag = vector
            .values
            .iter()
            .find(|(name, _)| *name == union.tag)
            .and_then(|(_, value)| value.parse::<u8>().ok())
            .ok_or_else(|| format!("the vector has no numeric '{}'", union.tag))?;
        let variant = union
            .variants
            .iter()
            .find(|variant| variant.tag == tag)
            .ok_or_else(|| format!("the vector uses tag {tag}, which the union lacks"))?;
        push_fields(entry, &variant.fields, &mut names);
    }
    Ok(names)
}

/// Field names in wire order, a defined type with fields only expanded in
/// place.
fn push_fields<'a>(entry: &'a Entry, fields: &'a [Field], names: &mut Vec<&'a str>) {
    let counts = count_fields(entry);
    for field in fields {
        match &field.ty {
            FieldType::Named(name) => match entry.local_type(name) {
                Some(ty) if ty.payload.union.is_none() => {
                    push_fields(entry, &ty.payload.fields, names)
                }
                _ if counts.contains(field.name.as_str()) => {}
                _ => names.push(&field.name),
            },
            FieldType::List(_) => names.push(&field.name),
        }
    }
}

/// The names of the fields that count a list in this entry.
fn count_fields(entry: &Entry) -> HashSet<&str> {
    let payloads = std::iter::once(&entry.payload).chain(entry.types.iter().map(|ty| &ty.payload));
    payloads
        .flat_map(|payload| &payload.fields)
        .filter_map(|field| match &field.ty {
            FieldType::List(list) => Some(list.count.as_str()),
            FieldType::Named(_) => None,
        })
        .collect()
}
