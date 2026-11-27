//! Server → client events (DRAFT v1; owners B3 + F3).
//!
//! Wire shape: `{ "v": 1, "type": "…", "revision"?: n, "payload": {…} }`.
//! `revision` sits at the ENVELOPE level on stateful events
//! (`game.state`, `game.move_rejected`, `game.finished`).

use serde::{Deserialize, Serialize};

use super::error_code::ErrorCode;

/// Seat color. Wire values: `"white" | "black"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerColor {
    White,
    Black,
}

/// Room lifecycle status. Wire values: `waiting | active | finished | expired`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    Waiting,
    Active,
    Finished,
    Expired,
}

/// Game phase (independent of room status). Wire values:
/// `in_progress | check | checkmate | stalemate | draw | resigned`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    InProgress,
    Check,
    Checkmate,
    Stalemate,
    Draw,
    Resigned,
}

/// Why the game ended. Wire values are snake_case as listed in
/// `docs/protocol-v1.md` §4.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResultReason {
    Checkmate,
    Stalemate,
    Resignation,
    DrawAgreed,
    InsufficientMaterial,
    ThreefoldRepetition,
    FiftyMoveRule,
}

/// Final result. `winner` is `None` for draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameResult {
    pub winner: Option<PlayerColor>,
    pub reason: GameResultReason,
}

/// One move as displayed in history / last-move highlight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRecord {
    pub from: String,
    pub to: String,
    /// Standard algebraic notation, e.g. `"Nf3"`, `"exd5"`, `"e8=Q"`.
    pub san: String,
}

/// Public player data — safe to broadcast. NEVER contains `player_token`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerPublic {
    pub id: String,
    pub nickname: String,
    pub color: PlayerColor,
    pub connected: bool,
}

/// Public room state for the waiting room / lobby views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomPublicState {
    pub room_code: String,
    pub status: RoomStatus,
    pub players: Vec<PlayerPublic>,
}

/// The full authoritative game state — the only source of truth for the
/// client. Built by B2 (rooms) from B1's engine snapshot + room data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub room_code: String,
    pub status: RoomStatus,
    pub game_status: GameStatus,
    /// Present only when the game is finished.
    pub result: Option<GameResult>,
    /// Full FEN (with legal en-passant square).
    pub fen: String,
    pub turn: PlayerColor,
    pub last_move: Option<MoveRecord>,
    /// SAN move list, oldest first.
    pub move_history: Vec<String>,
    /// UCI-style legal moves for the side to move, e.g. `"e2e4"`, `"e7e8q"`.
    /// Server-generated only.
    pub legal_moves: Vec<String>,
    pub players: Vec<PlayerPublic>,
    /// Color of the player with an active draw offer, if any.
    pub draw_offer: Option<PlayerColor>,
}

/// One server event. Adjacently tagged: `type` + `payload`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerEvent {
    /// PRIVATE reply to `room.create` (only to the creating connection).
    #[serde(rename = "room.created")]
    RoomCreated {
        room_code: String,
        player_id: String,
        /// Private reconnect credential — this is the ONLY place it ever
        /// appears on the wire.
        player_token: String,
        color: PlayerColor,
        room: RoomPublicState,
    },

    /// PRIVATE reply to `room.join` (only to the joining connection).
    #[serde(rename = "room.joined")]
    RoomJoined {
        player_id: String,
        player_token: String,
        color: PlayerColor,
        room: RoomPublicState,
    },

    /// PRIVATE reply to `session.resume` — full snapshot at the current revision.
    #[serde(rename = "session.ready")]
    SessionReady {
        player_id: String,
        color: PlayerColor,
        snapshot: GameSnapshot,
    },

    /// Broadcast after every accepted state change. Envelope carries `revision`;
    /// the payload IS the snapshot (its fields sit directly inside `payload`).
    #[serde(rename = "game.state")]
    GameState {
        #[serde(flatten)]
        snapshot: GameSnapshot,
    },

    /// Broadcast when a player connects/disconnects.
    #[serde(rename = "player.presence")]
    PlayerPresence { player_id: String, connected: bool },

    /// Targeted at the mover whose move failed. Envelope carries `revision`.
    #[serde(rename = "game.move_rejected")]
    MoveRejected {
        error: ErrorCode,
        /// Safe, human-readable message (no internals).
        message: String,
        snapshot: Option<GameSnapshot>,
    },

    /// Broadcast when the game ends. Envelope carries `revision`.
    #[serde(rename = "game.finished")]
    GameFinished {
        result: GameResult,
        snapshot: GameSnapshot,
    },

    /// Any rejected command or protocol violation.
    #[serde(rename = "server.error")]
    ServerError { error: ErrorCode, message: String },
}
