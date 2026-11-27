//! Client → server commands (DRAFT v1; owners B3 + F3).
//!
//! Wire shape: `{ "v": 1, "id": "<command-id>", "type": "…", "payload": {…} }`.
//! The `type`/`payload` pair is an adjacently-tagged enum flattened into the
//! envelope — see `envelope.rs`. Field names are the exact wire names.

use serde::{Deserialize, Serialize};

/// Pawn promotion choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Promotion {
    /// Queen — wire value `"q"`.
    Q,
    /// Rook — wire value `"r"`.
    R,
    /// Bishop — wire value `"b"`.
    B,
    /// Knight — wire value `"n"`.
    N,
}

/// One client command. Never rename a variant's wire name (`room.create`, …)
/// without updating `frontend/src/types/protocol.ts` and `docs/protocol-v1.md`
/// in the same PR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientCommand {
    /// Create a room; host takes White. Private reply: `room.created`.
    #[serde(rename = "room.create")]
    CreateRoom {
        /// 1–20 chars, no control characters (validated by B3 at the gate).
        nickname: String,
    },

    /// Join a waiting room by code; guest takes Black. Private reply: `room.joined`.
    #[serde(rename = "room.join")]
    JoinRoom {
        /// 8 chars from the unambiguous alphabet, case-insensitive (normalized
        /// by B3 at the gate: trim + uppercase, before B2 lookup).
        room_code: String,
        nickname: String,
    },

    /// Resume the same seat after a reconnect. Reply: `session.ready`.
    #[serde(rename = "session.resume")]
    ResumeSession {
        room_code: String,
        /// Private reconnect credential — sent only by the owning client.
        player_token: String,
    },

    /// Submit one move. Broadcast on success: `game.state`; to the mover on
    /// failure: `game.move_rejected`.
    #[serde(rename = "game.move")]
    MakeMove {
        /// From square, `"a1".."h8"`.
        from: String,
        /// To square.
        to: String,
        /// `"q" | "r" | "b" | "n"`, always serialized (null when not a promotion).
        promotion: Option<Promotion>,
        /// The revision the client believes it is moving against; stale values
        /// are rejected with `STALE_REVISION`.
        expected_revision: u64,
    },

    /// Offer a draw; if the opponent already offered, this ACCEPTS it.
    #[serde(rename = "game.offer_draw")]
    OfferDraw { expected_revision: u64 },

    /// Accept the opponent's active draw offer.
    #[serde(rename = "game.accept_draw")]
    AcceptDraw { expected_revision: u64 },

    /// Resign the current game.
    #[serde(rename = "game.resign")]
    Resign { expected_revision: u64 },

    /// Vote for a rematch; the new game starts when both players have voted.
    #[serde(rename = "game.rematch")]
    Rematch { expected_revision: u64 },

    /// Leave the room explicitly. Payload is an empty object `{}` on the wire
    /// (struct variant — a unit variant would omit `payload` and reject
    /// `"payload": {}`, which is what TS clients send).
    #[serde(rename = "room.leave")]
    LeaveRoom {},
}
