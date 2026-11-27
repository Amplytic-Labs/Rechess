//! Protocol wire-contract tests — verify that the exact JSON from
//! `docs/protocol-v1.md` round-trips through the public types.
//!
//! These tests run green TODAY against the contract types alone (no module
//! implementations needed). B3 owns extending this suite; F3 mirrors the
//! same fixtures in `frontend/src/types/protocol.test.ts`.

use rechess_backend::protocol::*;
use rechess_backend::rooms::testkit::sample_protocol_snapshot;

/// The exact §2.1 example from docs/protocol-v1.md.
#[test]
fn doc_section_2_1_example_round_trips() {
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
    let back = serde_json::to_value(&envelope).expect("serializes");
    let payload = back["payload"].as_object().expect("payload object");
    // `promotion` is always present on the wire (null when not a promotion).
    assert!(payload.contains_key("promotion"));
    assert_eq!(payload["promotion"], serde_json::Value::Null);
}

/// `game.state` payload IS the snapshot (flattened), revision at envelope level.
#[test]
fn game_state_payload_is_the_snapshot() {
    let envelope = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: Some(13),
        event: ServerEvent::GameState { snapshot: sample_protocol_snapshot() },
    };
    let value = serde_json::to_value(&envelope).expect("serializes");
    assert_eq!(value["type"], "game.state");
    assert_eq!(value["revision"], 13);
    // Snapshot fields sit directly in the payload…
    assert!(value["payload"].get("fen").is_some(), "fen must be in the payload");
    assert!(value["payload"].get("turn").is_some(), "turn must be in the payload");
    // …not nested under a "snapshot" key.
    assert!(value["payload"].get("snapshot").is_none(), "no nested snapshot key");
}

/// `player_token` appears ONLY in the private `room.created`/`room.joined` events.
#[test]
fn only_room_created_joined_carry_player_token() {
    let snapshot = sample_protocol_snapshot();
    let room = RoomPublicState {
        room_code: "TESTCODE1".into(),
        status: RoomStatus::Active,
        players: vec![PlayerPublic {
            id: "host".into(),
            nickname: "Host".into(),
            color: PlayerColor::White,
            connected: true,
        }],
    };

    let private_events = vec![
        ServerEvent::RoomCreated {
            room_code: "TESTCODE1".into(),
            player_id: "host".into(),
            player_token: "secret-token".into(),
            color: PlayerColor::White,
            room: room.clone(),
        },
        ServerEvent::RoomJoined {
            player_id: "guest".into(),
            player_token: "secret-token".into(),
            color: PlayerColor::Black,
            room,
        },
    ];
    for event in private_events {
        let text = serde_json::to_string(&event).expect("serializes");
        assert!(text.contains("player_token"), "private event must carry the token: {text}");
    }

    let public_events = vec![
        ServerEvent::SessionReady { player_id: "host".into(), color: PlayerColor::White, snapshot: snapshot.clone() },
        ServerEvent::GameState { snapshot: snapshot.clone() },
        ServerEvent::PlayerPresence { player_id: "host".into(), connected: false },
        ServerEvent::MoveRejected { error: ErrorCode::IllegalMove, message: "not legal".into(), snapshot: Some(snapshot.clone()) },
        ServerEvent::GameFinished {
            result: GameResult { winner: Some(PlayerColor::White), reason: GameResultReason::Checkmate },
            snapshot: snapshot.clone(),
        },
        ServerEvent::ServerError { error: ErrorCode::Internal, message: "nope".into() },
    ];
    for event in public_events {
        let text = serde_json::to_string(&event).expect("serializes");
        assert!(!text.contains("token"), "token leaked into a public event: {text}");
    }
}

/// `revision` lives on the ENVELOPE, never inside the payload.
#[test]
fn revision_lives_on_the_envelope_only() {
    let envelope = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: Some(7),
        event: ServerEvent::MoveRejected { error: ErrorCode::StaleRevision, message: "stale".into(), snapshot: None },
    };
    let value = serde_json::to_value(&envelope).expect("serializes");
    assert_eq!(value["revision"], 7);
    assert!(value["payload"].get("revision").is_none(), "revision must not repeat in the payload");

    let envelope = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: Some(9),
        event: ServerEvent::GameFinished {
            result: GameResult { winner: None, reason: GameResultReason::DrawAgreed },
            snapshot: sample_protocol_snapshot(),
        },
    };
    let value = serde_json::to_value(&envelope).expect("serializes");
    assert_eq!(value["revision"], 9);
    assert!(value["payload"].get("revision").is_none(), "revision must not repeat in the payload");
}

/// Malformed or unknown input is rejected with an error, never a panic.
#[test]
fn malformed_and_unknown_commands_fail_cleanly() {
    let bad_inputs = [
        r#"{ "v": 1, "id": "x", "type": "pizza.order", "payload": {} }"#, // unknown type
        r#"{ "v": 1, "id": "x", "type": "game.move", "payload": { "from": 42, "to": "e4", "expected_revision": 1 } }"#, // wrong field type
        r#"{ "v": 1, "id": "x", "type": "game.move", "payload": {} }"#, // missing fields
        r#"{ "v": 1, "id": "x" }"#, // no type/payload at all
        r#"{ "v": 1 "#,              // truncated JSON
        "not json at all",
    ];
    for input in bad_inputs {
        assert!(
            serde_json::from_str::<ClientEnvelope>(input).is_err(),
            "invalid input was accepted: {input}"
        );
    }
}

/// Every error code round-trips; wire form is SCREAMING_SNAKE_CASE.
#[test]
fn every_error_code_round_trips() {
    let codes = [
        ErrorCode::RoomNotFound,
        ErrorCode::RoomFull,
        ErrorCode::RoomExpired,
        ErrorCode::InvalidToken,
        ErrorCode::NotYourTurn,
        ErrorCode::IllegalMove,
        ErrorCode::StaleRevision,
        ErrorCode::GameFinished,
        ErrorCode::InvalidPayload,
        ErrorCode::UnsupportedVersion,
        ErrorCode::RateLimited,
        ErrorCode::Internal,
    ];
    for code in codes {
        let text = serde_json::to_string(&code).expect("serializes");
        let back: ErrorCode = serde_json::from_str(&text).expect("reparses");
        assert_eq!(code, back);
    }
    assert_eq!(serde_json::to_string(&ErrorCode::NotYourTurn).unwrap(), r#""NOT_YOUR_TURN""#);
}

/// Promotion letters on the wire: "q" | "r" | "b" | "n".
#[test]
fn promotion_serializes_as_lowercase_letter() {
    let envelope = ClientEnvelope {
        v: PROTOCOL_VERSION,
        id: "x".into(),
        command: ClientCommand::MakeMove {
            from: "e7".into(),
            to: "e8".into(),
            promotion: Some(Promotion::Q),
            expected_revision: 3,
        },
    };
    let text = serde_json::to_string(&envelope).unwrap();
    assert!(text.contains(r#""promotion":"q""#), "got: {text}");
}
