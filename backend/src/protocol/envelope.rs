//! Envelope shapes for both directions (DRAFT v1; owners B3 + F3).

use serde::{Deserialize, Serialize};

use super::client_message::ClientCommand;
use super::server_message::ServerEvent;

/// Client → server envelope.
///
/// ```json
/// { "v": 1, "id": "cmd-unique-001", "type": "game.move", "payload": { … } }
/// ```
///
/// `command` is flattened: the adjacently-tagged `type`/`payload` pair is
/// spread into this struct on the wire (serde `flatten` + tag/content).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvelope {
    /// Protocol version; must equal `PROTOCOL_VERSION` (B3 gates it).
    pub v: u32,
    /// Unique command id (client-generated UUID) — used for dedup by B2.
    pub id: String,
    #[serde(flatten)]
    pub command: ClientCommand,
}

/// Server → client envelope.
///
/// ```json
/// { "v": 1, "type": "game.state", "revision": 13, "payload": { … } }
/// ```
///
/// `revision` is present only on stateful events (`game.state`,
/// `game.move_rejected`, `game.finished`) — docs/protocol-v1.md §2.2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerEnvelope {
    pub v: u32,
    #[serde(flatten)]
    pub event: ServerEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
}
