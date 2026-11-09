use bones_kernel_infra::logging::Logger;

/// Carries out what egui asked of the platform this frame: opening a link
/// a `hyperlink` widget was clicked on, and copying selected text.
pub(crate) fn apply(output: &egui::PlatformOutput, logger: &Logger) {
    for command in &output.commands {
        match command {
            egui::OutputCommand::OpenUrl(open) => {
                if let Err(error) = open::that_detached(&open.url) {
                    logger.warn("ui", &format!("opening {}: {error}", open.url));
                }
            }
            egui::OutputCommand::CopyText(text) => {
                let copied = arboard::Clipboard::new().and_then(|mut c| c.set_text(text));
                if let Err(error) = copied {
                    logger.warn("ui", &format!("copying to the clipboard: {error}"));
                }
            }
            _ => {}
        }
    }
}

/// The clipboard's text for a paste, if it holds any.
pub(crate) fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .ok()
}
