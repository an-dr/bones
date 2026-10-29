use super::*;

#[test]
fn the_abi_is_read_from_the_vectors_header() {
    let vectors = "# bones core message wire format -- conformance vectors\n\
                   # ABI line 1.3.0, the version of bones-messages that generated this file\n";
    assert_eq!(abi_from_vectors(vectors).as_deref(), Some("1.3.0"));
}

#[test]
fn the_shipped_vectors_name_this_builds_abi() {
    let vectors = include_str!("../../../../wit/vectors/vectors.txt");
    assert_eq!(
        abi_from_vectors(vectors).as_deref(),
        Some(crate::versions::abi())
    );
}

#[test]
fn an_explicit_engine_that_does_not_exist_is_an_error() {
    let error = locate(Some("no/such/bones")).unwrap_err();
    assert!(error.contains("--engine"), "{error}");
}
