use super::*;
use crate::logging::RecordingSink;

#[test]
fn lines_below_the_minimum_are_dropped() {
    let sink = RecordingSink::new();
    let filter = LevelFilter::new(Arc::new(sink.clone()), Level::Info);
    filter.log(Level::Debug, "runner", "tick");
    filter.log(Level::Info, "engine", "loaded");
    filter.log(Level::Error, "host", "trapped");
    let kept: Vec<String> = sink.records().into_iter().map(|(_, _, m)| m).collect();
    assert_eq!(kept, ["loaded", "trapped"]);
}

#[test]
fn level_names_parse_in_any_case() {
    assert_eq!(LevelFilter::parse("WARN"), Some(Level::Warn));
    assert_eq!(LevelFilter::parse("debug"), Some(Level::Debug));
    assert_eq!(LevelFilter::parse("loud"), None);
}
