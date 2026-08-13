/// Every message on the bus (messaging.md). TODO: `correlation` stays
/// unused — synchronous send's (ADR-010) reply is the call's return value,
/// not a separate correlated message; kept for a future async pattern that
/// needs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub topic: String,
    pub sender: String,
    pub correlation: Option<u64>,
    pub payload: Vec<u8>,
}
