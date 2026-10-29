use std::collections::HashMap;

/// One conformance vector: a payload and the field values it decodes to, in
/// the order the vector lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vector {
    /// Lowercase hex; `-` is the empty payload.
    pub hex: String,
    pub values: Vec<(String, String)>,
}

/// Every vector in `vectors.txt`, by topic or endpoint, in file order. The
/// format is three space-separated fields per line (wit/vectors/README.md).
pub fn parse(text: &str) -> HashMap<String, Vec<Vector>> {
    let mut vectors: HashMap<String, Vec<Vector>> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, ' ');
        let (Some(topic), Some(hex)) = (parts.next(), parts.next()) else {
            continue;
        };
        let values = parts
            .next()
            .unwrap_or_default()
            .split(';')
            .filter_map(|pair| pair.split_once('='))
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        vectors.entry(topic.to_string()).or_default().push(Vector {
            hex: hex.to_string(),
            values,
        });
    }
    vectors
}
