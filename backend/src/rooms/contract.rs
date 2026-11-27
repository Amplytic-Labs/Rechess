//! FROZEN public contract of the `rooms` module (draft for Day-1 freeze).
//!
//! **Owner: B2** implements the [`RoomRegistry`] and [`RoomHandle`] traits
//! (the actor lives behind them). **B3** consumes ONLY these traits — never
//! B2's internal structs. This is the exact surface B4 exercises in
//! integration tests.
//!
//! Dependency direction (one-way, per `docs/architecture.md`):
//! `rooms` → `game` (engine trait) and `rooms` → `protocol` (wire snapshot
//! structs, which B2 assembles). `game` and `protocol` stay independent of
//! `rooms`.
//!
//! **Change rule:** signature or semantic changes here are a PR agreed by
//! B2 + B3, with B1 if the engine trait is affected.

use crate::game::{GameResult, PlayerColor, Promotion};

/// Opaque player identifier. Public data (safe to broadcast).
pub type PlayerId = String;

/// Private credentials for one seat — created by `create_room`/`join_room`.
/// The `player_token` is a SECRET: it goes only to the owning connection
/// (B3) and is stored hashed by B2. It NEVER appears in broadcasts or logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCredentials {
    pub player_id: PlayerId,
    pub player_token: String,
    pub color: PlayerColor,
}

/// Result of `create_room`: the host (White) seat plus the room handle.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomCreated<H> {
    pub room_code: String,
    pub credentials: PlayerCredentials,
    pub handle: H,
    /// Initial public room state (waiting, host seated).
    pub room: crate::protocol::RoomPublicState,
}

/// Result of `join_room`: the guest (Black) seat plus the room handle.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomJoined<H> {
    pub credentials: PlayerCredentials,
    pub handle: H,
    /// Public room state after joining (both seats filled).
    pub room: crate::protocol::RoomPublicState,
}

/// Result of a successful `resume_room`: the full current snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Resumed<H> {
    pub player_id: PlayerId,
    pub color: PlayerColor,
    pub handle: H,
    pub snapshot: crate::protocol::GameSnapshot,
}

/// Registry errors. B3 maps these onto protocol `ErrorCode`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateRoomError {
    Internal(String),
}

/// See `docs/protocol-v1.md` §6 for the wire mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinRoomError {
    /// → `ROOM_NOT_FOUND`.
    NotFound,
    /// → `ROOM_FULL`.
    Full,
    /// → `ROOM_EXPIRED`.
    Expired,
    /// → `INTERNAL`.
    Internal,
}

/// See `docs/protocol-v1.md` §6 for the wire mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeError {
    /// → `ROOM_NOT_FOUND`.
    NotFound,
    /// → `INVALID_TOKEN`.
    InvalidToken,
    /// → `INTERNAL`.
    Internal,
}

/// One command into a room actor. B3 translates protocol commands into
/// these; the actor processes them SEQUENTIALLY.
///
/// `expected_revision` is the client-supplied freshness guard — the room
/// actor rejects stale values with `CommandOutcome::Rejected`
/// (`STALE_REVISION`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomCommand {
    Move {
        player: PlayerId,
        from: String,
        to: String,
        promotion: Option<Promotion>,
        expected_revision: u64,
    },
    OfferDraw {
        player: PlayerId,
        expected_revision: u64,
    },
    AcceptDraw {
        player: PlayerId,
        expected_revision: u64,
    },
    Resign {
        player: PlayerId,
        expected_revision: u64,
    },
    Rematch {
        player: PlayerId,
        expected_revision: u64,
    },
    Leave {
        player: PlayerId,
    },
    /// B3 notifies B2 that a player's connection dropped (starts the grace
    /// deadline in B2).
    Disconnected {
        player: PlayerId,
    },
}

/// Per-command reply to the sending connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Accepted; the room revision AFTER this command.
    Accepted { revision: u64 },
    /// Rejected; B3 turns this into `server.error` / `game.move_rejected`.
    Rejected {
        error: crate::protocol::ErrorCode,
        message: String,
        revision: u64,
    },
}

/// One outbound room event. `target` selects the delivery route:
/// * `None` — broadcast to every room connection (→ `game.state`,
///   `player.presence`, `game.finished`).
/// * `Some(player)` — deliver ONLY to that player's connection.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomEvent {
    pub target: Option<PlayerId>,
    pub kind: RoomEventKind,
}

impl RoomEvent {
    /// Helper: broadcast event.
    pub fn broadcast(kind: RoomEventKind) -> Self {
        Self { target: None, kind }
    }

    /// Helper: event targeted at one player.
    pub fn to(player: PlayerId, kind: RoomEventKind) -> Self {
        Self {
            target: Some(player),
            kind,
        }
    }
}

/// Event payload. Wire names in comments — B3 maps these onto
/// `protocol::ServerEvent` and attaches the envelope-level revision.
#[derive(Debug, Clone, PartialEq)]
pub enum RoomEventKind {
    /// → broadcast `game.state`.
    Snapshot {
        revision: u64,
        snapshot: crate::protocol::GameSnapshot,
    },
    /// → broadcast `player.presence`.
    Presence {
        player_id: PlayerId,
        connected: bool,
    },
    /// → broadcast `game.finished`.
    Finished {
        revision: u64,
        result: GameResult,
        snapshot: crate::protocol::GameSnapshot,
    },
    /// → targeted `game.move_rejected` (to the mover).
    MoveRejected {
        player_id: PlayerId,
        error: crate::protocol::ErrorCode,
        message: String,
        revision: u64,
        snapshot: Option<crate::protocol::GameSnapshot>,
    },
}

/// The handle into one room's actor. Cloning shares the same room.
///
/// Methods return boxed-less `impl Future + Send` futures (the recommended
/// form for public traits): implementors simply write `async fn` — the
/// compiler proves the future `Send`. Consumers use the trait generically
/// (`H: RoomHandle`), not as `dyn`.
pub trait RoomHandle: Clone + Send + Sync {
    /// Send one command; resolves with the actor's reply.
    /// `Err(RoomClosed)` means the room expired/finished and its actor ended.
    fn send(
        &self,
        command: RoomCommand,
    ) -> impl std::future::Future<Output = Result<CommandOutcome, RoomClosed>> + Send;

    /// Subscribe to this room's outbound events. One receiver per subscriber;
    /// bounded (capacity chosen by the implementation — B2 documents it).
    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<RoomEvent>;
}

/// The room actor has ended (room closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomClosed;

impl std::fmt::Display for RoomClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "room closed")
    }
}

impl std::error::Error for RoomClosed {}

/// The registry B3 calls for room lifecycle operations. B4 fakes this in
/// tests (see `rooms::testkit::FakeRegistry`).
///
/// Like `RoomHandle`, futures are `impl Future + Send`; implementors write
/// plain `async fn`.
pub trait RoomRegistry: Send + Sync {
    type Handle: RoomHandle;

    /// Create a room; the host sits White. Returns the public room code and
    /// the host's PRIVATE credentials (delivered only to the creator).
    fn create_room(
        &self,
        nickname: &str,
    ) -> impl std::future::Future<Output = Result<RoomCreated<Self::Handle>, CreateRoomError>> + Send;

    /// Join a `waiting` room with a free seat (first-wins on races).
    /// `room_code` is already normalized (uppercased/trimmed) by B3.
    fn join_room(
        &self,
        room_code: &str,
        nickname: &str,
    ) -> impl std::future::Future<Output = Result<RoomJoined<Self::Handle>, JoinRoomError>> + Send;

    /// Resume a seat with the private reconnect token (full snapshot back).
    fn resume_room(
        &self,
        room_code: &str,
        token: &str,
    ) -> impl std::future::Future<Output = Result<Resumed<Self::Handle>, ResumeError>> + Send;

    /// Look a room up by normalized code (used when routing in-room commands).
    fn lookup(
        &self,
        room_code: &str,
    ) -> impl std::future::Future<Output = Option<Self::Handle>> + Send;
}
