//! `wit/wire-format.json`, compiled in, as an encoder and decoder between
//! JSON fields and payload bytes for every core topic (ADR-041).
//!
//! A payload's fields are one JSON object, keyed by field name:
//!
//! - a union's tag is a field under the union's tag name, and the fields of
//!   the variant it selects sit beside it;
//! - a defined shape written in place, such as `game-core/entity-op`'s
//!   spawn, contributes its fields to the same object;
//! - a counted list is a JSON array, and its count field is left out: the
//!   encoder writes the array's length, and the decoder does not report it.

use serde_json::{json, Map, Value};

const SPEC_JSON: &str = include_str!("../../../wit/wire-format.json");

/// The specification, parsed once per use.
pub struct Catalogue {
    spec: Value,
}

impl Catalogue {
    pub fn load() -> Self {
        Self {
            spec: serde_json::from_str(SPEC_JSON).expect("wire-format.json is valid JSON"),
        }
    }

    fn entries(&self) -> impl Iterator<Item = (&Value, &Value)> {
        self.spec["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|group| {
                group["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(move |entry| (group, entry))
            })
    }

    /// The entry whose payload `name` carries: a topic it lists, or an
    /// endpoint of an entry without topics. An endpoint that shares an
    /// entry with a topic (`persistence`) has a request shape of its own,
    /// given only in prose.
    fn layout(&self, name: &str) -> Option<&Value> {
        self.entries().map(|(_, entry)| entry).find(|entry| {
            let has = |key: &str| strings(&entry[key]).any(|n| n == name);
            has("topics") || (strings(&entry["topics"]).next().is_none() && has("endpoints"))
        })
    }

    /// The bytes of `topic` with these field values.
    pub fn encode(&self, topic: &str, values: &Map<String, Value>) -> Result<Vec<u8>, String> {
        let entry = self.layout(topic).ok_or_else(|| {
            format!("{topic} is not a core topic in wit/wire-format.json; send it as hex")
        })?;
        let mut bytes = Vec::new();
        Shape { entry }
            .write_payload(&entry["payload"], values, &mut bytes)
            .map_err(|error| format!("{topic}{error}"))?;
        Ok(bytes)
    }

    /// The fields of a `topic` payload, or `None` when the bytes do not
    /// decode as the specification says.
    pub fn decode(&self, topic: &str, bytes: &[u8]) -> Option<Value> {
        let entry = self.layout(topic)?;
        let mut cursor = bytes;
        let mut object = Map::new();
        Shape { entry }.read_payload(&entry["payload"], &mut cursor, &mut object)?;
        cursor.is_empty().then_some(Value::Object(object))
    }

    /// Every entry naming a topic or endpoint that starts with `prefix`, as
    /// the specification gives it, with its group.
    pub fn describe(&self, prefix: &str) -> Value {
        let list: Vec<Value> = self
            .entries()
            .filter(|(_, entry)| {
                strings(&entry["topics"])
                    .chain(strings(&entry["endpoints"]))
                    .any(|name| name.starts_with(prefix))
            })
            .map(|(group, entry)| {
                let mut described = entry.clone();
                described["group"] = group["name"].clone();
                described
            })
            .collect();
        Value::Array(list)
    }
}

/// The string items of a JSON array, or nothing.
fn strings(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

/// Reads and writes payloads within one entry, whose `types` its fields
/// may name.
struct Shape<'a> {
    entry: &'a Value,
}

impl<'a> Shape<'a> {
    fn local_type(&self, name: &str) -> Option<&'a Value> {
        items(&self.entry["types"]).find(|ty| ty["name"] == name)
    }

    /// Error text starts with `.field` or ` needs`, so the caller can put
    /// the topic in front.
    fn write_payload(
        &self,
        payload: &Value,
        values: &Map<String, Value>,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        self.write_fields(&payload["fields"], values, out)?;
        let union = &payload["union"];
        if union.is_null() {
            return Ok(());
        }
        let tag_name = union["tag"].as_str().unwrap_or_default();
        let tag = values
            .get(tag_name)
            .and_then(Value::as_u64)
            .ok_or_else(|| format!(" needs field '{tag_name}', the variant's tag"))?;
        let variant = items(&union["variants"])
            .find(|variant| variant["tag"].as_u64() == Some(tag))
            .ok_or_else(|| {
                let known: Vec<String> = items(&union["variants"])
                    .map(|v| format!("{} {}", v["tag"], v["name"].as_str().unwrap_or_default()))
                    .collect();
                format!(
                    ".{tag_name}: {tag} is not a variant; one of {}",
                    known.join(", ")
                )
            })?;
        out.push(tag as u8);
        self.write_fields(&variant["fields"], values, out)
    }

    fn write_fields(
        &self,
        fields: &Value,
        values: &Map<String, Value>,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        for field in items(fields) {
            let name = field["name"].as_str().unwrap_or_default();
            let ty = &field["type"];
            if let Some(list) = counted_list(fields, name) {
                let length = values
                    .get(list)
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len);
                let length =
                    u16::try_from(length).map_err(|_| format!(".{list}: too many items"))?;
                out.extend(length.to_le_bytes());
            } else if let Some(element) = ty["list"].as_str() {
                let list = values
                    .get(name)
                    .and_then(Value::as_array)
                    .ok_or_else(|| format!(" needs field '{name}', an array of {element}"))?;
                for (index, item) in list.iter().enumerate() {
                    self.write_value(element, item, out)
                        .map_err(|error| format!(".{name}[{index}]{error}"))?;
                }
            } else if let Some(defined) = ty.as_str().and_then(|ty| self.local_type(ty)) {
                self.write_payload(&defined["payload"], values, out)?;
            } else {
                let ty = ty.as_str().unwrap_or_default();
                let value = values
                    .get(name)
                    .ok_or_else(|| format!(" needs field '{name}' ({ty})"))?;
                self.write_value(ty, value, out)
                    .map_err(|error| format!(".{name}{error}"))?;
            }
        }
        Ok(())
    }

    /// One value of type `ty`: a primitive, or a defined shape given as an
    /// object of its fields.
    fn write_value(&self, ty: &str, value: &Value, out: &mut Vec<u8>) -> Result<(), String> {
        if let Some(primitive) = Primitive::parse(ty) {
            return primitive
                .write(value, out)
                .map_err(|error| format!(": {error}"));
        }
        let defined = self
            .local_type(ty)
            .ok_or_else(|| format!(": unknown type {ty}"))?;
        let fields = value
            .as_object()
            .ok_or_else(|| format!(": expected an object of {ty} fields, got {value}"))?;
        self.write_payload(&defined["payload"], fields, out)
    }

    fn read_payload(
        &self,
        payload: &Value,
        cursor: &mut &[u8],
        object: &mut Map<String, Value>,
    ) -> Option<()> {
        self.read_fields(&payload["fields"], cursor, object)?;
        let union = &payload["union"];
        if union.is_null() {
            return Some(());
        }
        let tag = Primitive::U8.read(cursor)?;
        let variant = items(&union["variants"]).find(|variant| variant["tag"] == tag)?;
        object.insert(union["tag"].as_str()?.to_string(), tag);
        self.read_fields(&variant["fields"], cursor, object)
    }

    fn read_fields(
        &self,
        fields: &Value,
        cursor: &mut &[u8],
        object: &mut Map<String, Value>,
    ) -> Option<()> {
        let mut counts: Map<String, Value> = Map::new();
        for field in items(fields) {
            let name = field["name"].as_str()?;
            let ty = &field["type"];
            if counted_list(fields, name).is_some() {
                counts.insert(name.to_string(), Primitive::U16.read(cursor)?);
            } else if let Some(element) = ty["list"].as_str() {
                let length = counts.get(ty["count"].as_str()?)?.as_u64()?;
                let list = (0..length)
                    .map(|_| self.read_value(element, cursor))
                    .collect::<Option<Vec<Value>>>()?;
                object.insert(name.to_string(), Value::Array(list));
            } else if let Some(defined) = ty.as_str().and_then(|ty| self.local_type(ty)) {
                self.read_payload(&defined["payload"], cursor, object)?;
            } else {
                let value = Primitive::parse(ty.as_str()?)?.read(cursor)?;
                object.insert(name.to_string(), value);
            }
        }
        Some(())
    }

    fn read_value(&self, ty: &str, cursor: &mut &[u8]) -> Option<Value> {
        if let Some(primitive) = Primitive::parse(ty) {
            return primitive.read(cursor);
        }
        let mut fields = Map::new();
        self.read_payload(&self.local_type(ty)?["payload"], cursor, &mut fields)?;
        Some(Value::Object(fields))
    }
}

/// The list that the field `name` counts, when it counts one.
fn counted_list<'a>(fields: &'a Value, name: &str) -> Option<&'a str> {
    items(fields)
        .find(|field| field["type"]["count"] == name)
        .and_then(|field| field["name"].as_str())
}

// `Primitive` stays in this file rather than splitting further: it is how
// `Shape` reads a type name, never named or constructed outside it.
/// One primitive of `wit/wire-format.json`, with its JSON form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Primitive {
    U8,
    U16,
    U32,
    I32,
    F32,
    Bool,
    Str,
    StrRest,
    Blob,
    Rest,
}

impl Primitive {
    fn parse(ty: &str) -> Option<Self> {
        Some(match ty {
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "i32" => Self::I32,
            "f32" => Self::F32,
            "bool" => Self::Bool,
            "str" => Self::Str,
            "str-rest" => Self::StrRest,
            "blob" => Self::Blob,
            "rest" => Self::Rest,
            _ => return None,
        })
    }

    /// Appends `value` in this type's encoding; bytes are given as hex.
    fn write(self, value: &Value, out: &mut Vec<u8>) -> Result<(), String> {
        let integer = |max: u64| {
            value
                .as_u64()
                .filter(|n| *n <= max)
                .ok_or_else(|| format!("expected an integer from 0 to {max}, got {value}"))
        };
        match self {
            Self::U8 => out.push(integer(u8::MAX as u64)? as u8),
            Self::U16 => out.extend((integer(u16::MAX as u64)? as u16).to_le_bytes()),
            Self::U32 => out.extend((integer(u32::MAX as u64)? as u32).to_le_bytes()),
            Self::I32 => {
                let n = value
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or_else(|| format!("expected a 32-bit integer, got {value}"))?;
                out.extend(n.to_le_bytes());
            }
            Self::F32 => {
                let n = value
                    .as_f64()
                    .ok_or_else(|| format!("expected a number, got {value}"))?;
                out.extend((n as f32).to_le_bytes());
            }
            Self::Bool => {
                let b = value
                    .as_bool()
                    .ok_or_else(|| format!("expected true or false, got {value}"))?;
                out.push(u8::from(b));
            }
            Self::Str | Self::StrRest => {
                let text = value
                    .as_str()
                    .ok_or_else(|| format!("expected a string, got {value}"))?;
                if self == Self::Str {
                    let length =
                        u16::try_from(text.len()).map_err(|_| "string longer than 65535 bytes")?;
                    out.extend(length.to_le_bytes());
                }
                out.extend(text.as_bytes());
            }
            Self::Blob | Self::Rest => {
                let bytes = value
                    .as_str()
                    .and_then(crate::hex::parse)
                    .ok_or_else(|| format!("expected bytes as a hex string, got {value}"))?;
                if self == Self::Blob {
                    let length = u32::try_from(bytes.len()).map_err(|_| "blob too large")?;
                    out.extend(length.to_le_bytes());
                }
                out.extend(bytes);
            }
        }
        Ok(())
    }

    /// Reads this type from the front of `cursor`, advancing it.
    fn read(self, cursor: &mut &[u8]) -> Option<Value> {
        fn take<'a>(cursor: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
            let (head, tail) = (cursor.get(..n)?, cursor.get(n..)?);
            *cursor = tail;
            Some(head)
        }
        Some(match self {
            Self::U8 => json!(take(cursor, 1)?[0]),
            Self::U16 => json!(u16::from_le_bytes(take(cursor, 2)?.try_into().ok()?)),
            Self::U32 => json!(u32::from_le_bytes(take(cursor, 4)?.try_into().ok()?)),
            Self::I32 => json!(i32::from_le_bytes(take(cursor, 4)?.try_into().ok()?)),
            Self::F32 => json!(f32::from_le_bytes(take(cursor, 4)?.try_into().ok()?)),
            Self::Bool => json!(take(cursor, 1)?[0] != 0),
            Self::Str => {
                let length = u16::from_le_bytes(take(cursor, 2)?.try_into().ok()?) as usize;
                json!(std::str::from_utf8(take(cursor, length)?).ok()?)
            }
            Self::StrRest => json!(std::str::from_utf8(take(cursor, cursor.len())?).ok()?),
            Self::Blob => {
                let length = u32::from_le_bytes(take(cursor, 4)?.try_into().ok()?) as usize;
                json!(crate::hex::format(take(cursor, length)?))
            }
            Self::Rest => json!(crate::hex::format(take(cursor, cursor.len())?)),
        })
    }
}

#[cfg(test)]
mod tests;
