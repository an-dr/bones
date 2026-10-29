use super::*;

const VECTORS: &str = include_str!("../../../../wit/vectors/vectors.txt");

/// The primitive type of every field name `topic`'s layout uses, union tags
/// included. Names repeat across variants with the same type.
fn field_types(catalogue: &Catalogue, topic: &str) -> Map<String, Value> {
    fn collect(payload: &Value, types: &mut Map<String, Value>) {
        let union = &payload["union"];
        if let Some(tag) = union["tag"].as_str() {
            types.insert(tag.to_string(), json!("u8"));
        }
        let fields = items(&payload["fields"])
            .chain(items(&union["variants"]).flat_map(|variant| items(&variant["fields"])));
        for field in fields {
            let name = field["name"].as_str().unwrap().to_string();
            types.insert(name, field["type"].clone());
        }
    }
    let entry = catalogue.layout(topic).unwrap();
    let mut types = Map::new();
    collect(&entry["payload"], &mut types);
    for ty in items(&entry["types"]) {
        collect(&ty["payload"], &mut types);
    }
    types
}

/// A vector's `name=value` as the JSON the field's type takes.
fn json_value(ty: &str, text: &str) -> Value {
    match ty {
        "u8" | "u16" | "u32" => json!(text.parse::<u64>().unwrap()),
        "i32" => json!(text.parse::<i64>().unwrap()),
        "f32" => json!(text.parse::<f64>().unwrap()),
        "bool" => json!(text == "1" || text == "true"),
        _ => json!(text),
    }
}

#[test]
fn every_vector_without_a_list_encodes_and_decodes_exactly() {
    let catalogue = Catalogue::load();
    let mut checked = 0;
    for line in VECTORS
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let mut parts = line.splitn(3, ' ');
        let (topic, hex) = (parts.next().unwrap(), parts.next().unwrap());
        if catalogue.layout(topic).is_none() {
            continue;
        }
        let types = field_types(&catalogue, topic);
        // Lists are written differently in a vector; they have tests below.
        if types.values().any(|ty| ty.get("list").is_some()) {
            continue;
        }
        let given: Map<String, Value> = parts
            .next()
            .unwrap_or_default()
            .split(';')
            .filter_map(|pair| pair.split_once('='))
            .map(|(name, text)| {
                let ty = types[name].as_str().unwrap_or_default();
                (name.to_string(), json_value(ty, text))
            })
            .collect();
        let bytes = catalogue.encode(topic, &given).unwrap();
        assert_eq!(crate::hex::format(&bytes), hex.replace('-', ""), "{line}");
        let decoded = catalogue.decode(topic, &bytes).unwrap();
        let again = catalogue.encode(topic, decoded.as_object().unwrap());
        assert_eq!(again.unwrap(), bytes, "{line}");
        checked += 1;
    }
    assert!(checked > 70, "only {checked} vectors were checked");
}

#[test]
fn a_widget_list_encodes_with_its_count() {
    let catalogue = Catalogue::load();
    let fields = json!({ "title": "notes", "widgets": [
        { "kind": 0, "text": "name" },
        { "kind": 1, "id": 1, "text": "ada" },
        { "kind": 2, "id": 2, "label": "save" },
    ]});
    let bytes = catalogue
        .encode("ui/spec", fields.as_object().unwrap())
        .unwrap();
    assert_eq!(
        crate::hex::format(&bytes),
        "05006e6f74657303000004006e616d65010100000003006164610202000000040073617665"
    );
    assert_eq!(catalogue.decode("ui/spec", &bytes), Some(fields));
}

#[test]
fn a_list_of_strings_encodes_as_the_vector_says() {
    let catalogue = Catalogue::load();
    let fields = json!({ "cwd": "/home/me", "args": ["--x", "y"] });
    let bytes = catalogue
        .encode("instance/launched", fields.as_object().unwrap())
        .unwrap();
    assert_eq!(
        crate::hex::format(&bytes),
        "08002f686f6d652f6d65020003002d2d78010079"
    );
    assert_eq!(catalogue.decode("instance/launched", &bytes), Some(fields));
}

#[test]
fn a_key_press_encodes_from_its_field() {
    let catalogue = Catalogue::load();
    let fields = json!({ "key": "Left" });
    let bytes = catalogue
        .encode("input/key-down", fields.as_object().unwrap())
        .unwrap();
    assert_eq!(bytes, b"Left");
    assert_eq!(catalogue.decode("input/key-down", &bytes), Some(fields));
}

#[test]
fn a_union_needs_a_known_tag() {
    let catalogue = Catalogue::load();
    let missing = catalogue
        .encode("game-core/entity-op", &Map::new())
        .unwrap_err();
    assert!(missing.contains("'tag'"), "{missing}");
    let unknown = json!({ "tag": 99 });
    let error = catalogue
        .encode("game-core/entity-op", unknown.as_object().unwrap())
        .unwrap_err();
    assert!(
        error.contains("99 is not a variant") && error.contains("9 reset"),
        "{error}"
    );
}

#[test]
fn a_missing_or_out_of_range_field_names_itself() {
    let catalogue = Catalogue::load();
    let missing = catalogue.encode("gfx/clear", &Map::new()).unwrap_err();
    assert!(missing.contains("'r'"), "{missing}");
    let wide = json!({ "r": 300, "g": 0, "b": 0, "a": 0 });
    let error = catalogue
        .encode("gfx/clear", wide.as_object().unwrap())
        .unwrap_err();
    assert!(error.contains("gfx/clear.r"), "{error}");
}

#[test]
fn an_application_topic_asks_for_hex() {
    let error = Catalogue::load()
        .encode("app/score", &Map::new())
        .unwrap_err();
    assert!(error.contains("hex"), "{error}");
}

#[test]
fn describe_lists_entries_under_a_prefix_with_their_group() {
    let listed = Catalogue::load().describe("input/key");
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["group"], "input/*");
    assert_eq!(listed[0]["payload"]["fields"][0]["name"], "key");
}
