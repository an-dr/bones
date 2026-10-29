use super::*;

#[test]
fn defaults_open_devtools_on_the_documented_port() {
    assert_eq!(parse(&[]).unwrap(), (None, "8642".to_string()));
}

#[test]
fn options_are_read_by_name() {
    let args = ["--devtools", "9000", "--engine", "x/bones"].map(String::from);
    assert_eq!(
        parse(&args).unwrap(),
        (Some("x/bones".to_string()), "9000".to_string())
    );
}

#[test]
fn a_snapshot_sees_an_edit_and_skips_build_output() {
    let dir = std::env::temp_dir().join("bones-cli-test-snapshot");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("target")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "a").unwrap();
    std::fs::write(dir.join("target/out"), "a").unwrap();

    let before = snapshot(&dir);
    assert_eq!(before.len(), 1);
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(dir.join("src/lib.rs"), "b").unwrap();
    assert_ne!(snapshot(&dir), before);

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_taken_port_is_refused_with_the_fix() {
    let holder = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = holder.local_addr().unwrap().port().to_string();
    let error = ensure_port_free(&port).unwrap_err();
    assert!(error.contains("--devtools"), "{error}");
}
