//! Renders the specification as `wire-format.md`, the form people read.

use super::model::{Entry, EnumValue, Field, FieldType, Group, Payload, Section, Spec, Union};

/// The whole Markdown document. Blocks are separated by one blank line and
/// the text ends with a newline.
pub fn render(spec: &Spec) -> String {
    let mut blocks = vec![
        format!("# {}", spec.title),
        "<!-- Generated from wire-format.json by `cargo xtask wire-format`. Do not edit. -->"
            .to_string(),
        spec.introduction.clone(),
    ];
    for section in &spec.sections {
        push_section(&mut blocks, section);
    }
    for group in &spec.groups {
        push_group(&mut blocks, group);
    }
    for section in &spec.closing {
        push_section(&mut blocks, section);
    }
    let mut text = blocks.join("\n\n");
    text.push('\n');
    text
}

fn push_section(blocks: &mut Vec<String>, section: &Section) {
    blocks.push(format!("## {}", section.heading));
    blocks.push(section.markdown.clone());
    if !section.primitives.is_empty() {
        let mut table = String::from("| Type | Size | Encoding |\n| --- | --- | --- |");
        for primitive in &section.primitives {
            table.push_str(&format!(
                "\n| `{}` | {} | {} |",
                primitive.name, primitive.size, primitive.encoding
            ));
        }
        blocks.push(table);
    }
    blocks.extend(section.after_primitives.clone());
}

fn push_group(blocks: &mut Vec<String>, group: &Group) {
    match group.name.contains('/') {
        true => blocks.push(format!("## `{}`", group.name)),
        false => blocks.push(format!("## {}", group.name)),
    }
    blocks.extend(group.description.clone());
    for entry in &group.entries {
        push_entry(blocks, entry);
    }
}

fn push_entry(blocks: &mut Vec<String>, entry: &Entry) {
    let names: Vec<String> = entry.names().map(|name| format!("`{name}`")).collect();
    blocks.push(format!("### {}", names.join(", ")));
    blocks.extend(entry.description.clone());
    push_payload(blocks, entry, &entry.payload);
    for ty in &entry.types {
        blocks.push(format!("#### {}", ty.name));
        blocks.extend(ty.description.clone());
        push_payload(blocks, entry, &ty.payload);
        blocks.extend(ty.notes.clone());
    }
    blocks.extend(entry.notes.clone());
}

fn push_payload(blocks: &mut Vec<String>, entry: &Entry, payload: &Payload) {
    if payload.fields.is_empty() && payload.union.is_none() {
        blocks.push("Empty payload, zero bytes.".to_string());
        return;
    }
    if !payload.fields.is_empty() {
        blocks.push(field_table(entry, &payload.fields, payload.union.as_ref()));
    }
    if let Some(union) = &payload.union {
        blocks.push(union_table(entry, union));
    }
}

/// `| Field | Type | Meaning |`, adjacent fields that share a type and a
/// meaning on one row. A union that follows adds a row for its tag.
fn field_table(entry: &Entry, fields: &[Field], union: Option<&Union>) -> String {
    let mut rows: Vec<[String; 3]> = runs(fields, true)
        .iter()
        .map(|run| {
            let names: Vec<String> = run.iter().map(|f| format!("`{}`", f.name)).collect();
            let each = if run.len() > 1 { " each" } else { "" };
            [
                names.join(", "),
                format!("{}{each}", type_text(entry, &run[0].ty)),
                meaning_text(run[0]),
            ]
        })
        .collect();
    if let Some(union) = union {
        rows.push([
            format!("`{}`", union.tag),
            "`u8`".to_string(),
            "which variant follows, below".to_string(),
        ]);
    }
    let with_meaning = rows.iter().any(|row| !row[2].is_empty());
    let mut table = match with_meaning {
        true => String::from("| Field | Type | Meaning |\n| --- | --- | --- |"),
        false => String::from("| Field | Type |\n| --- | --- |"),
    };
    for [name, ty, meaning] in rows {
        match with_meaning {
            true if meaning.is_empty() => table.push_str(&format!("\n| {name} | {ty} | |")),
            true => table.push_str(&format!("\n| {name} | {ty} | {meaning} |")),
            false => table.push_str(&format!("\n| {name} | {ty} |")),
        }
    }
    table
}

/// `| Tag | Variant | Fields after the tag |`, one row per variant.
fn union_table(entry: &Entry, union: &Union) -> String {
    let mut table = String::from("| Tag | Variant | Fields after the tag |\n| ---: | --- | --- |");
    for variant in &union.variants {
        let cells: Vec<String> = runs(&variant.fields, false)
            .iter()
            .map(|run| inline_fields(entry, run))
            .collect();
        let fields = match cells.is_empty() {
            true => "none".to_string(),
            false => cells.join("; "),
        };
        table.push_str(&format!(
            "\n| {} | {} | {fields} |",
            variant.tag, variant.name
        ));
    }
    table
}

/// Fields of one run written in a sentence: `` `r`, `g`: `u8` ``, a defined
/// type by name, and any meaning after a dash.
fn inline_fields(entry: &Entry, run: &[&Field]) -> String {
    let first = run[0];
    if let FieldType::Named(name) = &first.ty {
        if entry.local_type(name).is_some() {
            return format!("{name} (below)");
        }
    }
    let names: Vec<String> = run.iter().map(|f| format!("`{}`", f.name)).collect();
    let mut text = format!("{}: {}", names.join(", "), type_text(entry, &first.ty));
    let meaning = meaning_text(first);
    if !meaning.is_empty() {
        text.push_str(&format!(" — {meaning}"));
    }
    text
}

/// Adjacent fields with the same type and meaning, so `r`, `g`, `b`, `a`
/// read as one row. In a table, fields join only when they share a stated
/// meaning; in a sentence, fields without one join too.
fn runs(fields: &[Field], in_table: bool) -> Vec<Vec<&Field>> {
    let mut runs: Vec<Vec<&Field>> = Vec::new();
    for field in fields {
        match runs.last_mut() {
            Some(run) if same_row(run[0], field, in_table) => run.push(field),
            _ => runs.push(vec![field]),
        }
    }
    runs
}

fn same_row(a: &Field, b: &Field, in_table: bool) -> bool {
    let plain = a.values.is_empty() && b.values.is_empty();
    let stated = !in_table || a.meaning.is_some();
    a.ty == b.ty && a.meaning == b.meaning && plain && stated
}

fn type_text(entry: &Entry, ty: &FieldType) -> String {
    let name = |name: &str| match entry.local_type(name) {
        Some(_) => name.to_string(),
        None => format!("`{name}`"),
    };
    match ty {
        FieldType::Named(ty) => name(ty),
        FieldType::List(list) => format!("`{}` × {}", list.count, name(&list.list)),
    }
}

/// The meaning, then the values: `` `0` left, `1` center ``.
fn meaning_text(field: &Field) -> String {
    let values: Vec<String> = field.values.iter().map(value_text).collect();
    match (&field.meaning, values.is_empty()) {
        (Some(meaning), true) => meaning.clone(),
        (Some(meaning), false) => format!("{meaning}: {}", values.join(", ")),
        (None, false) => values.join(", "),
        (None, true) => String::new(),
    }
}

fn value_text(value: &EnumValue) -> String {
    format!("`{}` {}", value.value, value.name)
}
