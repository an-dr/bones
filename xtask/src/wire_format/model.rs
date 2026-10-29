//! The shape of `wit/wire-format.json`. These types read the specification,
//! and `wit/wire-format.schema.json` is derived from them, so the schema and
//! the reader cannot disagree. Every string documented as Markdown is
//! rendered into `wire-format.md` as written.

use schemars::JsonSchema;
use serde::Deserialize;

/// The bones core message wire format: every core-owned topic and
/// direct-send endpoint, and the rules that apply to all of them.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The JSON Schema this file follows, for editors.
    #[serde(rename = "$schema", default)]
    #[allow(dead_code, reason = "editors read it; xtask does not")]
    pub schema: Option<String>,
    pub title: String,
    /// Markdown shown before the first section.
    pub introduction: String,
    /// The rules that apply to every message, in order: scope, versioning,
    /// primitives, framing, decoding failures.
    pub sections: Vec<Section>,
    /// Topics and endpoints, grouped by namespace.
    pub groups: Vec<Group>,
    /// Sections after the last group.
    pub closing: Vec<Section>,
}

/// A section of prose that applies to the whole specification.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub heading: String,
    /// Markdown.
    pub markdown: String,
    /// The primitive types every field type is built from, shown as a table
    /// after `markdown`. Only the section that defines them lists them.
    #[serde(default)]
    pub primitives: Vec<Primitive>,
    /// Markdown shown after the primitives table.
    pub after_primitives: Option<String>,
}

/// One primitive field type.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Primitive {
    /// The name a field's `type` uses: `u8`, `str`, `rest`, ...
    pub name: String,
    /// Encoded size in bytes; `n` is the content length, `—` means the rest
    /// of the payload.
    pub size: String,
    /// Markdown.
    pub encoding: String,
}

/// The topics of one namespace, such as `gfx/*`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// The namespace (`gfx/*`) or a title (`Direct-send endpoints`).
    pub name: String,
    /// Markdown that applies to every entry of the group.
    pub description: Option<String>,
    pub entries: Vec<Entry>,
}

/// Topics or endpoints that share one payload shape.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Bus topics with this payload.
    #[serde(default)]
    pub topics: Vec<String>,
    /// Direct-send endpoints this entry describes. When the entry also
    /// names topics, the payload is the topics'; the notes give the
    /// endpoint's request and reply.
    #[serde(default)]
    pub endpoints: Vec<String>,
    /// Markdown shown before the payload.
    pub description: Option<String>,
    pub payload: Payload,
    /// Shapes the payload refers to by name, such as a list's element.
    #[serde(default)]
    pub types: Vec<TypeDef>,
    /// Markdown shown after the payload.
    pub notes: Option<String>,
}

/// A byte layout: fields in order, optionally followed by a tagged union.
/// No fields and no union is the empty payload.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    #[serde(default)]
    pub fields: Vec<Field>,
    /// A `u8` tag after the fields, then the fields of the variant it names.
    pub union: Option<Union>,
}

/// A named shape, local to the entry that defines it.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypeDef {
    pub name: String,
    /// Markdown.
    pub description: Option<String>,
    pub payload: Payload,
    /// Markdown.
    pub notes: Option<String>,
}

/// One field of a payload.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: FieldType,
    /// Markdown.
    pub meaning: Option<String>,
    /// The values a `u8` field may take, and what each means.
    #[serde(default)]
    pub values: Vec<EnumValue>,
}

/// A field's type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FieldType {
    /// A primitive, or a type the entry defines. A defined type with fields
    /// only is written in place: its fields follow on the wire as if listed
    /// here.
    Named(String),
    /// A counted sequence.
    List(ListType),
}

/// Elements back to back, as many as an earlier field says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListType {
    /// The element: a primitive or a type the entry defines.
    pub list: String,
    /// The earlier `u16` field that holds the number of elements.
    pub count: String,
}

/// A `u8` tag followed by the fields of the variant it selects.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Union {
    /// The tag's field name, as the conformance vectors list it.
    pub tag: String,
    pub variants: Vec<Variant>,
}

/// One variant of a tagged union.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub tag: u8,
    pub name: String,
    /// The fields after the tag; none when the tag is the whole variant.
    #[serde(default)]
    pub fields: Vec<Field>,
}

/// One value of an enumerated `u8` field.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnumValue {
    pub value: u8,
    pub name: String,
}

impl Entry {
    /// What the entry's heading names: endpoints first, then topics.
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.endpoints.iter().chain(&self.topics)
    }

    /// The type this entry defines under `name`.
    pub fn local_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.iter().find(|ty| ty.name == name)
    }
}

impl Spec {
    pub fn primitives(&self) -> impl Iterator<Item = &Primitive> {
        self.sections.iter().flat_map(|section| &section.primitives)
    }

    pub fn is_primitive(&self, name: &str) -> bool {
        self.primitives().any(|primitive| primitive.name == name)
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.groups.iter().flat_map(|group| &group.entries)
    }
}
