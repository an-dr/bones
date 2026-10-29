use super::*;

#[test]
fn a_line_carries_level_category_and_message() {
    assert_eq!(
        JsonSink::format(Level::Warn, "engine", "hello"),
        r#"{"level":"warn","category":"engine","message":"hello"}"#
    );
}

#[test]
fn quotes_backslashes_and_control_characters_are_escaped() {
    assert_eq!(
        JsonSink::format(Level::Error, "x", "a \"b\" \\ c\nd\u{1}"),
        r#"{"level":"error","category":"x","message":"a \"b\" \\ c\nd\u0001"}"#
    );
}
