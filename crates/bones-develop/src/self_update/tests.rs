use super::*;

#[test]
fn install_arguments_name_the_repository_and_an_optional_tag() {
    assert_eq!(
        install_args("https://x", None),
        [
            "install",
            "--git",
            "https://x",
            "--locked",
            "--force",
            "bones-develop"
        ]
    );
    assert_eq!(
        install_args("https://x", Some("v3.2.0")),
        [
            "install",
            "--git",
            "https://x",
            "--locked",
            "--force",
            "--tag",
            "v3.2.0",
            "bones-develop"
        ]
    );
}
