//! DRAFT WebSocket protocol v1 — wire types shared by backend and frontend.
//!
//! **This is the frozen-in-spirit draft contract** specified by
//! `docs/protocol-v1.md`. Owners: **B3** (this Rust module) + **F3**
//! (TypeScript mirror in `frontend/src/types/protocol.ts` — already written,
//! keep them in sync). Review it together on Day 1 and freeze it.
//!
//! **Change rule:** a protocol change must update, in the same reviewed PR:
//! 1. this module, 2. `frontend/src/types/protocol.ts`, 3. `docs/protocol-v1.md`.
//!
//! Wire shape (see the doc for full examples):
//! * client command: `{ "v": 1, "id": "<uuid>", "type": "game.move", "payload": {…} }`
//! * server event:   `{ "v": 1, "type": "game.state", "revision": 13, "payload": {…} }`

pub mod client_message;
pub mod envelope;
pub mod error_code;
pub mod server_message;

pub use client_message::{ClientCommand, Promotion};
pub use envelope::{ClientEnvelope, ServerEnvelope};
pub use error_code::ErrorCode;
pub use server_message::{
    GameResult, GameResultReason, GameSnapshot, GameStatus, MoveRecord, PlayerColor, PlayerPublic,
    RoomPublicState, RoomStatus, ServerEvent,
};

/// Current protocol version. Unsupported versions are rejected with
/// `UNSUPPORTED_VERSION` (B3 enforces at the gate).
pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip of the exact §2.1 example from docs/protocol-v1.md.
    #[test]
    fn client_move_envelope_round_trips() {
        let json = r#"{
            "v": 1,
            "id": "cmd-unique-001",
            "type": "game.move",
            "payload": {
                "from": "e2",
                "to": "e4",
                "promotion": null,
                "expected_revision": 12
            }
        }"#;
        let envelope: ClientEnvelope = serde_json::from_str(json).expect("valid envelope");
        assert_eq!(envelope.v, 1);
        assert_eq!(envelope.id, "cmd-unique-001");
        match &envelope.command {
            ClientCommand::MakeMove { from, to, promotion, expected_revision } => {
                assert_eq!((from.as_str(), to.as_str()), ("e2", "e4"));
                assert_eq!(*promotion, None);
                assert_eq!(*expected_revision, 12);
            }
            other => panic!("wrong command parsed: {other:?}"),
        }
        // Re-serialize and parse again — the wire format must be stable.
        let text = serde_json::to_string(&envelope).expect("serializes");
        let reparsed: ClientEnvelope = serde_json::from_str(&text).expect("reparses");
        assert_eq!(reparsed.id, envelope.id);
    }

    /// Every command variant must round-trip through the envelope.
    #[test]
    fn every_client_command_round_trips() {
        let envelopes: Vec<ClientEnvelope> = vec![
            ClientEnvelope { v: PROTOCOL_VERSION, id: "a".into(), command: ClientCommand::CreateRoom { nickname: "Guest 1".into() } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "b".into(), command: ClientCommand::JoinRoom { room_code: "ABC23XY9".into(), nickname: "Guest 2".into() } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "c".into(), command: ClientCommand::ResumeSession { room_code: "ABC23XY9".into(), player_token: "secret".into() } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "d".into(), command: ClientCommand::MakeMove { from: "e7".into(), to: "e8".into(), promotion: Some(Promotion::Q), expected_revision: 5 } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "e".into(), command: ClientCommand::OfferDraw { expected_revision: 5 } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "f".into(), command: ClientCommand::AcceptDraw { expected_revision: 6 } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "g".into(), command: ClientCommand::Resign { expected_revision: 6 } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "h".into(), command: ClientCommand::Rematch { expected_revision: 6 } },
            ClientEnvelope { v: PROTOCOL_VERSION, id: "i".into(), command: ClientCommand::LeaveRoom },
        ];
        for envelope in envelopes {
            let text = serde_json::to_string(&envelope).unwrap();
            let reparsed: ClientEnvelope = serde_json::from_str(&text).unwrap();
            let back = serde_json::to_string(&reparsed).unwrap();
            assert_eq!(text, back, "unstable wire format for {:?}", envelope.command);
        }
    }

    /// Round-trip of the exact §2.2 example (server event, envelope-level revision).
    #[test]
    fn server_state_envelope_round_trips() {
        let snapshot = sample_snapshot();
        let envelope = ServerEnvelope {
            v: PROTOCOL_VERSION,
            revision: Some(13),
            event: ServerEvent::GameState { snapshot },
        };
        let text = serde_json::to_string(&envelope).unwrap();
        assert!(text.contains(r#""type":"game.state""#), "got: {text}");
        assert!(text.contains(r#""revision":13"#), "got: {text}");
        assert!(!text.contains("player_token"), "no token may appear in a broadcast event");
        let reparsed: ServerEnvelope = serde_json::from_str(&text).unwrap();
        assert!(matches!(reparsed.event, ServerEvent::GameState { .. }));
        assert_eq!(reparsed.revision, Some(13));
    }

    #[test]
    fn malformed_payload_is_rejected_not_panicking() {
        let bad = r#"{ "v": 1, "id": "x", "type": "game.move", "payload": { "from": 42 } }"#;
        assert!(serde_json::from_str::<ClientEnvelope>(bad).is_err());
        let unknown = r#"{ "v": 1, "id": "x", "type": "pizza.order", "payload": {} }"#;
        assert!(serde_json::from_str::<ClientEnvelope>(unknown).is_err());
        let truncated = r#"{ "v": 1 "#;
        assert!(serde_json::from_str::<ClientEnvelope>(truncated).is_err());
    }

    /// Fixture used by other tests in this module. B3 owns the real protocol
    /// test suite; F3 mirrors the same fixture in protocol.test.ts.
    pub(crate) fn sample_snapshot() -> GameSnapshot {
        GameSnapshot {
            room_code: "ABC23XY9".into(),
            status: RoomStatus::Active,
            game_status: GameStatus::InProgress,
            result: None,
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".into(),
            turn: PlayerColor::Black,
            last_move: Some(MoveRecord { from: "e2".into(), to: "e4".into(), san: "e4".into() }),
            move_history: vec!["e4".into()],
            legal_moves: vec!["e7e5".into(), "e7e6".into()],
            players: vec![
                PlayerPublic { id: "p1".into(), nickname: "Guest 1".into(), color: PlayerColor::White, connected: true },
                PlayerPublic { id: "p2".into(), nickname: "Guest 2".into(), color: PlayerColor::Black, connected: true },
            ],
            draw_offer: None,
        }
    }
}
