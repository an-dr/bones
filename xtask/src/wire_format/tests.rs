use super::*;

/// The committed file at `relative`, without carriage returns: a Windows
/// checkout may hold CRLF.
fn committed(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative))
        .unwrap_or_default()
        .replace("\r\n", "\n")
}

#[test]
fn the_committed_markdown_and_schema_are_current() {
    let root = repo_root();
    let generated = generate(&root).unwrap_or_else(|error| panic!("{error}"));
    assert!(
        committed(&root, MARKDOWN) == generated.markdown,
        "{MARKDOWN} is stale; run `cargo xtask wire-format` and review the diff"
    );
    assert!(
        committed(&root, SCHEMA) == generated.schema,
        "{SCHEMA} is stale; run `cargo xtask wire-format` and review the diff"
    );
}

/// A one-entry specification with `payload` as its payload.
fn spec_with(topic: &str, payload: &str, types: &str) -> Spec {
    let json = format!(
        r#"{{"title": "t", "introduction": "",
            "sections": [{{"heading": "Primitives", "markdown": "", "primitives": [
                {{"name": "u8", "size": "1", "encoding": ""}},
                {{"name": "u16", "size": "2", "encoding": ""}},
                {{"name": "str", "size": "2 + n", "encoding": ""}}]}}],
            "groups": [{{"name": "t/*", "entries": [
                {{"topics": ["{topic}"], "payload": {payload}, "types": [{types}]}}]}}],
            "closing": []}}"#
    );
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn field_names_differing_from_the_vector_are_an_error() {
    let spec = spec_with("t/x", r#"{"fields": [{"name": "a", "type": "u8"}]}"#, "");
    let error = check::check(&spec, &vector::parse("t/x 01 b=1\n")).unwrap_err();
    assert!(error.contains("t/x") && error.contains("[a]"), "{error}");
}

#[test]
fn a_union_is_checked_against_the_variant_its_tag_names() {
    let spec = spec_with(
        "t/x",
        r#"{"fields": [{"name": "id", "type": "u8"}],
            "union": {"tag": "tag", "variants": [
                {"tag": 0, "name": "on", "fields": [{"name": "level", "type": "u8"}]},
                {"tag": 1, "name": "off"}]}}"#,
        "",
    );
    let vectors = vector::parse("t/x 070003 id=7;tag=0;level=3\nt/x 0701 id=7;tag=1\n");
    check::check(&spec, &vectors).unwrap();
    let wrong = vector::parse("t/x 0701 id=7;tag=1;level=3\n");
    assert!(check::check(&spec, &wrong).is_err());
}

#[test]
fn a_defined_type_is_expanded_in_place_and_a_count_may_be_left_out() {
    let spec = spec_with(
        "t/x",
        r#"{"fields": [{"name": "head", "type": "pair"}, {"name": "n", "type": "u16"},
            {"name": "items", "type": {"list": "str", "count": "n"}}]}"#,
        r#"{"name": "pair", "payload": {"fields": [
            {"name": "a", "type": "u8"}, {"name": "b", "type": "u8"}]}}"#,
    );
    let vectors = vector::parse("t/x - a=1;b=2;items=x\nt/x - a=1;b=2;n=1;items=x\n");
    check::check(&spec, &vectors).unwrap();
}

#[test]
fn an_unknown_type_and_a_missing_count_are_errors() {
    let spec = spec_with(
        "t/x",
        r#"{"fields": [{"name": "a", "type": "u64"},
            {"name": "items", "type": {"list": "str", "count": "n"}}]}"#,
        "",
    );
    let error = check::check(&spec, &Default::default()).unwrap_err();
    assert!(error.contains("unknown type 'u64'"), "{error}");
    assert!(error.contains("not an earlier u16"), "{error}");
}

#[test]
fn a_vector_for_an_unspecified_topic_is_an_error() {
    let spec = spec_with("t/x", r#"{"fields": []}"#, "");
    let error = check::check(&spec, &vector::parse("t/y - (no fields)\n")).unwrap_err();
    assert!(error.contains("t/y"), "{error}");
}

#[test]
fn shared_fields_render_on_one_row_and_values_after_the_meaning() {
    let spec = spec_with(
        "t/x",
        r#"{"fields": [{"name": "r", "type": "u8", "meaning": "color"},
            {"name": "g", "type": "u8", "meaning": "color"}, {"name": "layer", "type": "u8"},
            {"name": "mode", "type": "u8", "meaning": "how", "values": [
                {"value": 0, "name": "off"}, {"value": 1, "name": "on"}]}]}"#,
        "",
    );
    let markdown = render::render(&spec);
    assert!(
        markdown.contains("| `r`, `g` | `u8` each | color |"),
        "{markdown}"
    );
    assert!(markdown.contains("| `layer` | `u8` | |"), "{markdown}");
    assert!(
        markdown.contains("| `mode` | `u8` | how: `0` off, `1` on |"),
        "{markdown}"
    );
}

#[test]
fn an_empty_payload_says_so() {
    let markdown = render::render(&spec_with("t/x", r#"{"fields": []}"#, ""));
    assert!(
        markdown.contains("### `t/x`\n\nEmpty payload, zero bytes."),
        "{markdown}"
    );
}
