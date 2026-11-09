use bones_messages::ui::{Event, WidgetProps};

/// How one widget kind is drawn (ADR-048): the unit the ui module's
/// registry is open for.
///
/// - Implement it, then register it with `Ui::register` (or the kernel
///   builder); every tree can then place widgets of `Props::KIND`, from a
///   native module or an extension alike.
/// - The standard widgets are implemented the same way, so a custom kind
///   has everything they have.
pub trait Kind: Send + 'static {
    /// The properties this kind reads; its `KIND` is the registry key.
    type Props: WidgetProps;

    /// Draws one widget with id `id` into `ui`, pushing what the user did
    /// onto `events`.
    fn draw(&self, ui: &mut egui::Ui, id: u32, props: Self::Props, events: &mut Vec<Event>);
}
