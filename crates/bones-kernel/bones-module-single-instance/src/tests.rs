use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use super::*;
use crate::attempt::Attempt;

const TIMEOUT: Duration = Duration::from_secs(2);

/// App uids are per user, not per process, so every test claims its own — tests
/// run in parallel, and a run of this suite beside another must not collide.
fn unique(name: &str) -> String {
    format!("bones-test-{}-{name}", std::process::id())
}

fn handoff(args: &[&str]) -> Handoff {
    Handoff {
        args: args.iter().map(|arg| arg.to_string()).collect(),
        cwd: PathBuf::from("/somewhere"),
    }
}

fn primary(app_uid: &str) -> Primary {
    match claim(app_uid, handoff(&[]), TIMEOUT).unwrap() {
        Claim::Primary(primary) => primary,
        Claim::HandedOff => panic!("{app_uid} was already claimed"),
    }
}

#[test]
fn the_first_claim_becomes_the_instance() {
    let primary = primary(&unique("first"));

    assert!(primary.is_listening());
}

#[test]
fn a_second_launch_hands_its_arguments_and_directory_to_the_first() {
    let app_uid = unique("handoff");
    let primary = primary(&app_uid);
    let started = Instant::now();

    let second = claim(&app_uid, handoff(&["--x", "y"]), TIMEOUT).unwrap();

    assert!(matches!(second, Claim::HandedOff));
    assert!(started.elapsed() < TIMEOUT);
    let received = primary.incoming().recv_timeout(TIMEOUT).unwrap();
    assert_eq!(received, handoff(&["--x", "y"]));
}

#[test]
fn every_later_launch_arrives_in_order() {
    let app_uid = unique("order");
    let primary = primary(&app_uid);

    for index in 0..5 {
        let arg = index.to_string();
        assert!(matches!(
            claim(&app_uid, handoff(&[&arg]), TIMEOUT).unwrap(),
            Claim::HandedOff
        ));
    }

    let received: Vec<_> = (0..5)
        .map(|_| primary.incoming().recv_timeout(TIMEOUT).unwrap().args)
        .collect();
    assert_eq!(received, [["0"], ["1"], ["2"], ["3"], ["4"]]);
}

#[test]
fn two_app_uids_run_side_by_side_without_interfering() {
    let one = primary(&unique("side-one"));
    let two = primary(&unique("side-two"));

    assert!(matches!(
        claim(&unique("side-two"), handoff(&["two"]), TIMEOUT).unwrap(),
        Claim::HandedOff
    ));

    assert_eq!(
        two.incoming().recv_timeout(TIMEOUT).unwrap(),
        handoff(&["two"])
    );
    assert!(one.incoming().try_recv().is_err());
}

#[test]
fn dropping_the_instance_releases_the_claim() {
    let app_uid = unique("release");
    drop(primary(&app_uid));

    let next = primary(&app_uid);

    assert!(next.is_listening());
}

#[test]
fn an_instance_that_never_answers_is_given_up_on_after_the_timeout() {
    let app_uid = unique("hung");
    // Claimed, but never started: the endpoint exists and nothing answers,
    // which is exactly what a hung instance looks like from outside.
    let Attempt::Claimed(_hung) = platform::attempt(&app_uid, TIMEOUT).unwrap() else {
        panic!("the app_uid should have been free");
    };
    let timeout = Duration::from_millis(300);
    let started = Instant::now();

    let claim = claim(&app_uid, handoff(&["x"]), timeout).unwrap();

    let elapsed = started.elapsed();
    let Claim::Primary(primary) = claim else {
        panic!("a hung instance must not swallow the launch");
    };
    assert!(!primary.is_listening());
    assert!(elapsed >= timeout, "gave up after {elapsed:?}");
    assert!(elapsed < TIMEOUT, "took {elapsed:?}");
    assert!(matches!(
        primary.incoming().recv_timeout(Duration::ZERO),
        Err(RecvTimeoutError::Disconnected)
    ));
}

#[test]
fn an_invalid_app_uid_is_refused_before_anything_is_created() {
    for app_uid in [
        "",
        "has space",
        "slash/inside",
        "back\\slash",
        &"x".repeat(65),
    ] {
        assert!(
            claim(app_uid, handoff(&[]), TIMEOUT).is_err(),
            "{app_uid:?}"
        );
    }
}

#[test]
fn a_handoff_survives_the_wire_unchanged() {
    let original = Handoff {
        args: vec!["".into(), "ünïcode".into(), "with space".into()],
        cwd: PathBuf::from("dir"),
    };
    let frame = wire::encode(&original).unwrap();

    assert_eq!(wire::decode(&frame[4..]).unwrap(), original);
}

#[test]
fn a_truncated_or_padded_handoff_is_refused() {
    let frame = wire::encode(&handoff(&["a"])).unwrap();
    let body = &frame[4..];

    assert!(wire::decode(&body[..body.len() - 1]).is_err());
    let mut padded = body.to_vec();
    padded.push(0);
    assert!(wire::decode(&padded).is_err());
}

const CHILD_APP_UID: &str = "BONES_INSTANCE_TEST_CHILD_APP_UID";

/// Not a test of its own: the body of the child process
/// `a_killed_instance_leaves_nothing_a_later_launch_trusts` starts and kills.
#[test]
#[ignore = "runs only as the child process of the crash-recovery test"]
fn child_claims_and_waits_to_be_killed() {
    let Ok(app_uid) = std::env::var(CHILD_APP_UID) else {
        return;
    };
    let claimed = primary(&app_uid);
    assert!(claimed.is_listening());
    println!("claimed");
    std::thread::sleep(Duration::from_secs(60));
}

#[test]
fn a_killed_instance_leaves_nothing_a_later_launch_trusts() {
    let app_uid = unique("crash");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "tests::child_claims_and_waits_to_be_killed",
            "--nocapture",
        ])
        .env(CHILD_APP_UID, &app_uid)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    assert!(
        lines.any(|line| line.is_ok_and(|line| line.contains("claimed"))),
        "the child never claimed {app_uid}"
    );

    child.kill().unwrap();
    child.wait().unwrap();

    let after = primary(&app_uid);
    assert!(after.is_listening());
}

#[cfg(unix)]
#[test]
fn a_socket_file_left_by_a_crash_is_taken_over() {
    let app_uid = unique("stale");
    let (socket, _) = unix_paths(&app_uid);
    // Binding and dropping leaves the file behind with nothing accepting on
    // it, which is what a crashed instance leaves.
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    assert!(socket.exists());

    let claimed = primary(&app_uid);

    assert!(claimed.is_listening());
}

#[cfg(unix)]
fn unix_paths(app_uid: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("bones-single-instance-{}", unsafe {
                libc::getuid()
            }))
        });
    std::fs::create_dir_all(&dir).unwrap();
    (
        dir.join(format!("bones-single-instance-{app_uid}.sock")),
        dir.join(format!("bones-single-instance-{app_uid}.lock")),
    )
}
