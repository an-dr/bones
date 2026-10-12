/// Answers a direct `send` (ADR-010, messaging.md) with a reply, unlike
/// `Handler` which never returns anything back to the sender.
pub trait Respond: Send + Sync {
    fn respond(&self, sender: &str, payload: &[u8]) -> Option<Vec<u8>>;
}
