use super::*;

const SPEC: &str = include_str!("../../../../wit/wire-format.json");
const VECTORS: &str = include_str!("../../../../wit/vectors/vectors.txt");

/// Every topic `wire-format.json` specifies, in order; endpoints are not
/// topics and are left out.
fn specified_topics() -> Vec<String> {
    let spec: serde_json::Value = serde_json::from_str(SPEC).unwrap();
    let entries = spec["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["entries"].as_array().unwrap());
    entries
        .flat_map(|entry| entry["topics"].as_array().into_iter().flatten())
        .filter_map(|topic| topic.as_str())
        .map(str::to_string)
        .collect()
}

#[test]
fn the_list_matches_the_specification() {
    let listed: Vec<&str> = TOPICS.to_vec();
    let specified = specified_topics();
    let specified: Vec<&str> = specified.iter().map(String::as_str).collect();
    for topic in &listed {
        assert!(
            specified.contains(topic),
            "{topic} is not in wire-format.json"
        );
    }
    for topic in &specified {
        assert!(listed.contains(topic), "{topic} is missing from TOPICS");
    }
}

#[test]
fn every_listed_topic_is_validated() {
    for topic in TOPICS {
        assert!(validate(topic, &[]).is_some(), "{topic} is not validated");
    }
}

#[test]
fn every_conformance_vector_validates() {
    for line in VECTORS
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let mut parts = line.split(' ');
        let (topic, hex) = (parts.next().unwrap(), parts.next().unwrap());
        let payload: Vec<u8> = match hex {
            "-" => Vec::new(),
            hex => (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect(),
        };
        if let Some(result) = validate(topic, &payload) {
            assert_eq!(result, Ok(()), "{line}");
        }
    }
}

#[test]
fn application_topics_are_not_judged() {
    assert_eq!(validate("my-game/score", b"anything"), None);
    assert_eq!(validate("", b"a direct send"), None);
}

#[test]
fn a_malformed_core_payload_is_an_error() {
    assert!(matches!(
        validate(gfx::DrawRect::TOPIC, &[1, 2, 3]),
        Some(Err(_))
    ));
}

#[test]
fn an_explanation_names_the_topic_the_bytes_and_the_specification() {
    let text = explain("gfx/draw-rect", &[1, 2, 3], &DecodeError::Truncated);
    assert!(text.contains("'gfx/draw-rect'"), "{text}");
    assert!(text.contains("3 bytes"), "{text}");
    assert!(text.contains("010203"), "{text}");
    assert!(text.contains("wit/wire-format.md"), "{text}");
}

#[test]
fn a_misspelt_topic_names_the_core_topic_it_meant() {
    assert_eq!(closest("gfx/draw-rec"), Some("gfx/draw-rect"));
    assert_eq!(closest("gfx/drawrect"), Some("gfx/draw-rect"));
}

#[test]
fn a_core_topic_or_a_distant_one_has_no_suggestion() {
    assert_eq!(closest("gfx/draw-rect"), None);
    assert_eq!(closest("pong/score"), None);
}
