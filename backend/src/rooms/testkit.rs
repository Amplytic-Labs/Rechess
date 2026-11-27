//! Test kit for the `rooms` module: scriptable fakes for the registry and
//! the room handle.
//!
//! NOT a room implementation — it exists so B3 (network) and B4 (integration
//! tests) can test against the [`RoomRegistry`]/[`RoomHandle`] contracts
//! WITHOUT B2's real actor. B2's real implementation must satisfy the same
//! contract (see `src/rooms/README.md` §Testing).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::contract::{
    CommandOutcome, CreateRoomError, JoinRoomError, PlayerCredentials, ResumeError, Resumed,
    RoomClosed, RoomCommand, RoomCreated, RoomEvent, RoomHandle, RoomJoined, RoomRegistry,
};
use crate::game::PlayerColor;
use crate::protocol::{
    GameSnapshot, GameStatus, MoveRecord, PlayerColor as WireColor, PlayerPublic, RoomPublicState,
    RoomStatus,
};

#[derive(Default)]
struct Shared {
    /// Scripted outcomes, drained in order; default `Accepted { revision: 0 }`.
    outcomes: VecDeque<CommandOutcome>,
    /// Every command received so far, in order.
    received: Vec<RoomCommand>,
    /// Event subscribers (`send` side of the testkit).
    subscribers: Vec<tokio::sync::mpsc::Sender<RoomEvent>>,
    closed: bool,
}

/// Scriptable fake room handle.
#[derive(Clone, Default)]
pub struct FakeHandle {
    shared: Arc<Mutex<Shared>>,
}

impl FakeHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue the outcome returned by the NEXT `send` call
    /// (default: `Accepted { revision: 0 }`).
    pub fn push_outcome(&self, outcome: CommandOutcome) {
        self.shared.lock().unwrap().outcomes.push_back(outcome);
    }

    /// All commands received so far, in order.
    pub fn received(&self) -> Vec<RoomCommand> {
        self.shared.lock().unwrap().received.clone()
    }

    /// Deliver an event to every current subscriber (drops silently if a
    /// subscriber's queue is full — tests use generous capacities).
    pub fn emit(&self, event: RoomEvent) {
        let subscribers = self.shared.lock().unwrap().subscribers.clone();
        for tx in subscribers {
            let _ = tx.try_send(event.clone());
        }
    }

    /// Close the room: subsequent `send` calls return `Err(RoomClosed)`.
    pub fn close(&self) {
        self.shared.lock().unwrap().closed = true;
    }
}

impl RoomHandle for FakeHandle {
    async fn send(&self, command: RoomCommand) -> Result<CommandOutcome, RoomClosed> {
        let mut shared = self.shared.lock().unwrap();
        if shared.closed {
            return Err(RoomClosed);
        }
        shared.received.push(command);
        Ok(shared
            .outcomes
            .pop_front()
            .unwrap_or(CommandOutcome::Accepted { revision: 0 }))
    }

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<RoomEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        self.shared.lock().unwrap().subscribers.push(tx);
        rx
    }
}

/// Fake registry with exactly ONE room, fixed codes and tokens.
///
/// * `room_code` — the only joinable code; anything else → `NotFound`.
/// * host credentials: player `host`, token `host-token` (White).
/// * guest credentials: player `guest`, token `guest-token` (Black).
pub struct FakeRegistry {
    pub handle: FakeHandle,
    pub room_code: String,
    pub host_token: String,
    pub guest_token: String,
}

impl FakeRegistry {
    pub fn new(room_code: &str) -> Self {
        Self {
            handle: FakeHandle::new(),
            room_code: room_code.to_string(),
            host_token: "host-token".to_string(),
            guest_token: "guest-token".to_string(),
        }
    }

    fn room_view(&self, status: RoomStatus) -> RoomPublicState {
        RoomPublicState {
            room_code: self.room_code.clone(),
            status,
            players: vec![
                PlayerPublic {
                    id: "host".into(),
                    nickname: "Host".into(),
                    color: WireColor::White,
                    connected: true,
                },
                PlayerPublic {
                    id: "guest".into(),
                    nickname: "Guest".into(),
                    color: WireColor::Black,
                    connected: true,
                },
            ],
        }
    }
}

impl RoomRegistry for FakeRegistry {
    type Handle = FakeHandle;

    async fn create_room(
        &self,
        _nickname: &str,
    ) -> Result<RoomCreated<Self::Handle>, CreateRoomError> {
        Ok(RoomCreated {
            room_code: self.room_code.clone(),
            credentials: PlayerCredentials {
                player_id: "host".into(),
                player_token: self.host_token.clone(),
                color: PlayerColor::White,
            },
            handle: self.handle.clone(),
            room: self.room_view(RoomStatus::Waiting),
        })
    }

    async fn join_room(
        &self,
        room_code: &str,
        _nickname: &str,
    ) -> Result<RoomJoined<Self::Handle>, JoinRoomError> {
        if room_code != self.room_code {
            return Err(JoinRoomError::NotFound);
        }
        Ok(RoomJoined {
            credentials: PlayerCredentials {
                player_id: "guest".into(),
                player_token: self.guest_token.clone(),
                color: PlayerColor::Black,
            },
            handle: self.handle.clone(),
            room: self.room_view(RoomStatus::Active),
        })
    }

    async fn resume_room(
        &self,
        room_code: &str,
        token: &str,
    ) -> Result<Resumed<Self::Handle>, ResumeError> {
        if room_code != self.room_code {
            return Err(ResumeError::NotFound);
        }
        let (player_id, color) = if token == self.host_token {
            ("host", PlayerColor::White)
        } else if token == self.guest_token {
            ("guest", PlayerColor::Black)
        } else {
            return Err(ResumeError::InvalidToken);
        };
        Ok(Resumed {
            player_id: player_id.into(),
            color,
            handle: self.handle.clone(),
            snapshot: sample_protocol_snapshot(),
        })
    }

    async fn lookup(&self, room_code: &str) -> Option<Self::Handle> {
        (room_code == self.room_code).then(|| self.handle.clone())
    }
}

/// Wire-snapshot fixture matching docs/protocol-v1.md §4.3
/// (mirrors the TS fixture in `frontend/src/types/protocol.test.ts`).
pub fn sample_protocol_snapshot() -> GameSnapshot {
    GameSnapshot {
        room_code: "TESTCODE1".into(),
        status: RoomStatus::Active,
        game_status: GameStatus::InProgress,
        result: None,
        fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".into(),
        turn: WireColor::Black,
        last_move: Some(MoveRecord {
            from: "e2".into(),
            to: "e4".into(),
            san: "e4".into(),
        }),
        move_history: vec!["e4".into()],
        legal_moves: vec!["e7e5".into(), "e7e6".into()],
        players: vec![
            PlayerPublic {
                id: "host".into(),
                nickname: "Host".into(),
                color: WireColor::White,
                connected: true,
            },
            PlayerPublic {
                id: "guest".into(),
                nickname: "Guest".into(),
                color: WireColor::Black,
                connected: false,
            },
        ],
        draw_offer: None,
    }
}
