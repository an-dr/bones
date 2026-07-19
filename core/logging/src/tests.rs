use std::sync::Arc;

use crate::{Level, Logger, RecordingSink};

#[test]
fn logger_forwards_to_sink() {
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));

    logger.debug("bus", "subscribed to input/*");
    logger.info("host", "extension loaded");
    logger.warn("bus", "queue nearing budget");
    logger.error("host", "extension trapped");

    let records = sink.records();
    assert_eq!(
        records,
        vec![
            (Level::Debug, "bus".to_string(), "subscribed to input/*".to_string()),
            (Level::Info, "host".to_string(), "extension loaded".to_string()),
            (Level::Warn, "bus".to_string(), "queue nearing budget".to_string()),
            (Level::Error, "host".to_string(), "extension trapped".to_string()),
        ]
    );
}

#[test]
fn default_logger_uses_stdout_sink_without_panicking() {
    let logger = Logger::default();
    logger.info("smoke", "default logger is wired to stdout");
}

#[test]
fn logger_clone_shares_the_same_sink() {
    let sink = RecordingSink::new();
    let logger = Logger::new(Arc::new(sink.clone()));
    let cloned = logger.clone();

    logger.info("a", "first");
    cloned.info("b", "second");

    assert_eq!(sink.records().len(), 2);
}
