//! Stable wire error codes (DRAFT v1; owners B3 + F3).
//!
//! SCREAMING_SNAKE_CASE on the wire. B3 maps internal errors onto these;
//! F3 maps each code to a friendly, themed UI message.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Room code does not match an active room.
    RoomNotFound,
    /// Room exists but both seats are taken.
    RoomFull,
    /// Room reached its retention deadline.
    RoomExpired,
    /// Reconnect token invalid, expired or not matching the seat.
    InvalidToken,
    /// The seat is not allowed to submit a move now.
    NotYourTurn,
    /// Move rejected by the rules engine.
    IllegalMove,
    /// `expected_revision` does not match the room revision.
    StaleRevision,
    /// Game already has a result; no further actions.
    GameFinished,
    /// Payload failed validation (shape, lengths, control chars…).
    InvalidPayload,
    /// Envelope `v` is not a supported protocol version.
    UnsupportedVersion,
    /// Client exceeded the rate limit.
    RateLimited,
    /// Unexpected server error; details only in server logs.
    Internal,
}
