use super::*;
use std::time::{Duration, UNIX_EPOCH};

#[test]
fn strict_versions_preserve_prerelease_and_build_metadata() {
    let version = Version::parse("1.2.3-rc.1+build.007").unwrap();
    assert_eq!(version.pre.as_str(), "rc.1");
    assert_eq!(version.build.as_str(), "build.007");
    assert_eq!(version.to_string(), "1.2.3-rc.1+build.007");
    for invalid in [
        "1.2",
        "01.2.3",
        "1.2.3-01",
        "1.2.3-",
        "1.2.3+a..b",
        " 1.2.3",
        "1.2.3_foo",
        "18446744073709551616.0.0",
    ] {
        assert!(Version::parse(invalid).is_err(), "{invalid}");
        assert!(compare_versions(invalid, "1.0.0").is_err());
        assert!(!is_newer("1.0.0", invalid));
        assert!(!is_newer(invalid, "1.0.0"));
    }
}

#[test]
fn precedence_follows_the_semver_specification_chain() {
    let chain = [
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-alpha.beta",
        "1.0.0-beta",
        "1.0.0-beta.2",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
    ];
    for pair in chain.windows(2) {
        assert!(is_newer(pair[0], pair[1]), "{pair:?}");
        assert!(!is_newer(pair[1], pair[0]));
    }
    assert!(is_newer("1.2.0", "1.10.0"));
    assert_eq!(
        compare_versions("1.0.0+a", "1.0.0+z").unwrap(),
        Ordering::Equal
    );
    assert!(!is_newer("1.0.0+a", "1.0.0+z"));
}

#[test]
fn requirements_use_cargo_ranges_and_prerelease_opt_in() {
    let requirement = VersionReq::parse("^1.2.3").unwrap();
    for version in ["1.2.3", "1.9.0+build"] {
        assert!(requirement.matches(&Version::parse(version).unwrap()));
    }
    for version in ["1.2.2", "2.0.0", "1.3.0-beta"] {
        assert!(!requirement.matches(&Version::parse(version).unwrap()));
    }
    let prerelease = VersionReq::parse(">=1.2.3-alpha, <2.0.0").unwrap();
    assert!(prerelease.matches(&Version::parse("1.2.3-beta").unwrap()));
    assert!(!prerelease.matches(&Version::parse("1.3.0-beta").unwrap()));
    assert!(VersionReq::parse("nonsense").is_err());
}

#[test]
fn folder_instances_are_distinct_from_semver_identifiers() {
    let version = Version::parse("1.2.3-rc.1+build.7").unwrap();
    let name = format_folder_name(&version, "abc-2").unwrap();
    assert_eq!(name, "1.2.3-rc.1+build.7~abc-2");
    assert_eq!(parse_folder_name(&name), Some(version));
    assert_eq!(
        parse_folder_name("1.2.3-hash").unwrap().pre.as_str(),
        "hash"
    );
    for instance in ["", "../escape", "a\\b", "a~b", "a_b"] {
        assert!(format_folder_name(&Version::new(1, 2, 3), instance).is_none());
        assert!(parse_folder_name(&format!("1.2.3~{instance}")).is_none());
    }
}

#[test]
fn folders_order_by_precedence_then_time_including_build_and_instance_ties() {
    let folders = order_version_folders([
        (PathBuf::from("1.2.0"), UNIX_EPOCH),
        (PathBuf::from("1.10.0+z~aaa"), UNIX_EPOCH),
        (
            PathBuf::from("1.10.0+a~bbb"),
            UNIX_EPOCH + Duration::from_secs(1),
        ),
        (
            PathBuf::from("1.10.0-rc.1"),
            UNIX_EPOCH + Duration::from_secs(9),
        ),
        (PathBuf::from(".staging-2"), UNIX_EPOCH),
    ]);
    assert_eq!(
        folders,
        ["1.10.0+a~bbb", "1.10.0+z~aaa", "1.10.0-rc.1", "1.2.0"].map(PathBuf::from)
    );
}

#[test]
fn ordering_is_stable_and_does_not_require_paths_to_exist() {
    let a = (PathBuf::from("missing/2.0.0~a"), UNIX_EPOCH);
    let b = (PathBuf::from("missing/2.0.0~b"), UNIX_EPOCH);
    assert_eq!(
        order_version_folders([a.clone(), b.clone()]),
        order_version_folders([b, a])
    );
}
