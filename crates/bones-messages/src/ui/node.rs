use crate::{DecodeError, Reader, Writer};

use super::{Layout, WidgetProps};

/// Deepest nesting a decoded tree may have; a deeper payload is rejected
/// as `DecodeError::TooDeep` so a hostile payload cannot exhaust the stack.
pub const MAX_DEPTH: usize = 64;

/// One node of a widget tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A layout container.
    Container {
        /// Owner-assigned id; the ui module keeps state such as scroll
        /// position under it. Zero when the owner does not care.
        id: u32,
        /// How the children are arranged.
        layout: Layout,
        /// The children, in order.
        children: Vec<Node>,
    },
    /// A widget of a registered kind.
    Widget {
        /// The registry key, e.g. `"button"`.
        kind: String,
        /// Owner-assigned id that events name.
        id: u32,
        /// Properties, encoded as the kind defines.
        props: Vec<u8>,
    },
}

const TAG_CONTAINER: u8 = 0;
const TAG_WIDGET: u8 = 1;

impl Node {
    /// A container with no id.
    pub fn container(layout: Layout, children: Vec<Node>) -> Self {
        Node::Container {
            id: 0,
            layout,
            children,
        }
    }

    /// A widget of kind `P` with typed properties.
    pub fn widget<P: WidgetProps>(id: u32, props: &P) -> Self {
        Node::Widget {
            kind: P::KIND.to_owned(),
            id,
            props: props.encode(),
        }
    }

    /// Every widget of kind `P` in this subtree, depth first, with its id;
    /// what a test reads to check what an owner showed. A widget whose
    /// properties do not decode is skipped.
    pub fn widgets_of<P: WidgetProps>(&self) -> Vec<(u32, P)> {
        let mut found = Vec::new();
        self.collect(&mut found);
        found
    }

    fn collect<P: WidgetProps>(&self, found: &mut Vec<(u32, P)>) {
        match self {
            Node::Container { children, .. } => {
                for child in children {
                    child.collect(found);
                }
            }
            Node::Widget { kind, id, props } if kind == P::KIND => {
                if let Ok(props) = P::decode(props) {
                    found.push((*id, props));
                }
            }
            Node::Widget { .. } => {}
        }
    }

    pub(super) fn encode_into(&self, writer: Writer) -> Writer {
        match self {
            Node::Container {
                id,
                layout,
                children,
            } => {
                let count: u16 = children
                    .len()
                    .try_into()
                    .expect("more than u16::MAX children in one container");
                let mut writer = layout
                    .encode_into(writer.u8(TAG_CONTAINER).u32(*id))
                    .u16(count);
                for child in children {
                    writer = child.encode_into(writer);
                }
                writer
            }
            Node::Widget { kind, id, props } => {
                writer.u8(TAG_WIDGET).str(kind).u32(*id).blob(props)
            }
        }
    }

    pub(super) fn decode(reader: &mut Reader<'_>, depth: usize) -> Result<Self, DecodeError> {
        if depth > MAX_DEPTH {
            return Err(DecodeError::TooDeep);
        }
        let tag = reader.read_u8()?;
        match tag {
            TAG_CONTAINER => {
                let id = reader.read_u32()?;
                let layout = Layout::decode(reader)?;
                let count = reader.read_u16()?;
                let mut children = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    children.push(Node::decode(reader, depth + 1)?);
                }
                Ok(Node::Container {
                    id,
                    layout,
                    children,
                })
            }
            TAG_WIDGET => Ok(Node::Widget {
                kind: reader.read_str()?.to_owned(),
                id: reader.read_u32()?,
                props: reader.read_blob()?.to_vec(),
            }),
            _ => Err(DecodeError::InvalidTag {
                message: "ui node",
                tag,
            }),
        }
    }
}
