//! Hidden acceptance test for the calculator task.

use bones_test::App;

fn app() -> App {
    let wasm = bones_test::build(concat!(env!("CARGO_MANIFEST_DIR"), "/../extension")).unwrap();
    let mut app = App::load(wasm).unwrap();
    app.step(1);
    app
}

/// The reply to `request`, parsed as JSON.
fn ask(app: &mut App, request: &str) -> serde_json_lite::Value {
    let reply = app.request(request.as_bytes()).expect("no reply to the send");
    serde_json_lite::parse(&String::from_utf8(reply).expect("the reply is not UTF-8"))
}

#[test]
fn it_adds_subtracts_and_multiplies() {
    let mut app = app();
    assert_eq!(ask(&mut app, r#"{"op":"add","a":2,"b":3}"#).number("result"), Some(5.0));
    assert_eq!(ask(&mut app, r#"{"op":"sub","a":2,"b":3}"#).number("result"), Some(-1.0));
    assert_eq!(ask(&mut app, r#"{"op":"mul","a":2.5,"b":4}"#).number("result"), Some(10.0));
}

#[test]
fn it_divides_and_refuses_zero() {
    let mut app = app();
    assert_eq!(ask(&mut app, r#"{"op":"div","a":9,"b":2}"#).number("result"), Some(4.5));
    assert_eq!(
        ask(&mut app, r#"{"op":"div","a":1,"b":0}"#).text("error").as_deref(),
        Some("division by zero")
    );
}

#[test]
fn a_bad_request_gets_an_error_not_silence() {
    let mut app = app();
    for request in ["not json", r#"{"op":"pow","a":2,"b":3}"#, r#"{"op":"add","a":2}"#] {
        assert!(ask(&mut app, request).text("error").is_some(), "{request}");
    }
}

/// A JSON reader small enough to keep the test free of the agent's own
/// dependency choices.
mod serde_json_lite {
    /// A member's value: a string, or anything else kept as its raw text.
    enum Member {
        Text(String),
        Raw(String),
    }

    /// A flat JSON object's members.
    pub struct Value(Vec<(String, Member)>);

    impl Value {
        pub fn number(&self, key: &str) -> Option<f64> {
            match self.member(key)? {
                Member::Raw(raw) => raw.parse().ok(),
                Member::Text(_) => None,
            }
        }

        pub fn text(&self, key: &str) -> Option<String> {
            match self.member(key)? {
                Member::Text(text) => Some(text.clone()),
                Member::Raw(_) => None,
            }
        }

        fn member(&self, key: &str) -> Option<&Member> {
            self.0
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, member)| member)
        }
    }

    /// Reads `{"key": value, ...}`, where a string value may hold commas,
    /// colons, and escaped quotes; anything else is kept up to the next
    /// comma or the closing brace.
    pub fn parse(text: &str) -> Value {
        let mut chars = text.trim().chars().peekable();
        let mut members = Vec::new();
        let mut read_string = |chars: &mut std::iter::Peekable<std::str::Chars>| {
            let mut out = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some(other) => out.push(other),
                        None => break,
                    },
                    c => out.push(c),
                }
            }
            out
        };
        while let Some(c) = chars.next() {
            if c != '"' {
                continue;
            }
            let key = read_string(&mut chars);
            while chars.next_if(|c| c.is_whitespace() || *c == ':').is_some() {}
            let member = match chars.peek() {
                Some('"') => {
                    chars.next();
                    Member::Text(read_string(&mut chars))
                }
                _ => {
                    let mut raw = String::new();
                    while let Some(c) = chars.next_if(|c| *c != ',' && *c != '}') {
                        raw.push(c);
                    }
                    Member::Raw(raw.trim().to_string())
                }
            };
            members.push((key, member));
        }
        Value(members)
    }
}
