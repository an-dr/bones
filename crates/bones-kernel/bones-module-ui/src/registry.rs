use std::collections::HashMap;

use bones_messages::ui::WidgetProps;
use bones_messages::DecodeError;

use crate::draw_context::DrawContext;
use crate::kind::Kind;
use crate::standard;

/// Why a widget could not be drawn.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DrawProblem {
    /// No kind of that name is registered.
    UnknownKind,
    /// The kind is registered but the properties did not decode.
    BadProps(DecodeError),
}

/// Object-safe face of a `Kind`, so kinds with different `Props` share
/// one map.
trait AnyKind: Send {
    fn draw(
        &self,
        ui: &mut egui::Ui,
        id: u32,
        props: &[u8],
        cx: &mut DrawContext<'_>,
    ) -> Result<(), DecodeError>;
}

impl<K: Kind> AnyKind for K {
    fn draw(
        &self,
        ui: &mut egui::Ui,
        id: u32,
        props: &[u8],
        cx: &mut DrawContext<'_>,
    ) -> Result<(), DecodeError> {
        let props = K::Props::decode(props)?;
        Kind::draw(self, ui, id, props, cx);
        Ok(())
    }
}

/// Every widget kind the ui module can draw, by name.
pub(crate) struct Registry {
    kinds: HashMap<&'static str, Box<dyn AnyKind>>,
}

impl Registry {
    /// The standard kinds, registered through the same door as custom ones.
    pub(crate) fn standard() -> Self {
        let mut registry = Self {
            kinds: HashMap::new(),
        };
        standard::register_all(&mut registry);
        registry
    }

    /// Adds `kind`, replacing any kind registered under the same name.
    pub(crate) fn register<K: Kind>(&mut self, kind: K) {
        self.kinds.insert(K::Props::KIND, Box::new(kind));
    }

    pub(crate) fn draw(
        &self,
        ui: &mut egui::Ui,
        kind: &str,
        id: u32,
        props: &[u8],
        cx: &mut DrawContext<'_>,
    ) -> Result<(), DrawProblem> {
        let entry = self.kinds.get(kind).ok_or(DrawProblem::UnknownKind)?;
        entry.draw(ui, id, props, cx).map_err(DrawProblem::BadProps)
    }
}
