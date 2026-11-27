# 03 — Testing: two layers + a manual smoke

> Prereqs: `01` and `02`. Everything below was written against the REAL
> frozen contracts; layer (a) is **verified green** against
> `backend/Cargo.toml` as of this writing (14 tests, see §2). Layer (b) is
> target code for the moment your router exists (§3).

---

## 1. How to run everything

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

cargo test                        # the whole suite (B4's merge gate)
cargo test network::              # unit tests inside src/network/ (§2.3)
cargo test --test network_protocol # layer (a) — the serialization suite (§2)
cargo test --test network_ws       # layer (b) — the two-client harness (§3)
```

Baseline before you touch anything: 15 green tests — `protocol::` unit
tests (4), `tests/protocol_contract.rs` (7), `tests/testkit_demo.rs` (4).
If the baseline is not green, stop and tell B4.

## 2. Layer (a) — serialization unit tests: `backend/tests/network_protocol.rs`

These need ONLY `src/protocol/` and `rooms::testkit` — they compile and run
TODAY, before any of your code exists. **Start your work by adding this
file** (file placement in `backend/tests/` is B4's territory — `tests/README.md`
§1 — announce it, it is a formality: B4's own README tells feature owners to
coordinate test files there).

What each test guarantees (the "why" column is what you say if someone asks
why the test exists):

| Test | Guarantees |
|---|---|
| `every_server_event_serializes_with_its_wire_type` | All 8 events emit their exact `type` string — F3 switches on these |
| `game_state_payload_is_the_flattened_snapshot` | `GameState`'s `#[serde(flatten)]` works: snapshot fields sit directly in `payload`, no nesting |
| `envelope_revision_is_omitted_when_none` | `revision` appears only on stateful events; `skip_serializing_if` keeps v1 envelopes clean |
| `token_never_appears_in_public_event_envelopes` | The core security invariant, per envelope, for every public event |
| `private_replies_carry_the_token_in_their_payload` | `room.created` carries the token — the ONLY place it may |
| `gate_rejects_malformed_input_as_invalid_payload` | The 3-step parse gate (doc 02 §2.1) classifies garbage as `INVALID_PAYLOAD` |
| `gate_rejects_unknown_version_as_unsupported_version` | Version is checked BEFORE command shape (doc 02 §3) |
| `gate_rejects_unknown_command_type_as_invalid_payload` | v1 + unknown type ⇒ `INVALID_PAYLOAD`, not a disconnect |
| `gate_accepts_every_v1_command` | Every command in docs/protocol-v1.md §3 parses |
| `every_state_changing_command_serializes_expected_revision` | The freshness guard is on the wire for all 5 state-changing commands |
| `move_command_always_carries_promotion` | `promotion` is always present (null when absent) — F3 reads it unconditionally |
| `server_error_envelope_has_stable_code_and_safe_message` | `server.error` shape: SCREAMING_SNAKE code, safe message, no revision |
| `session_ready_carries_the_full_snapshot` | Resume replaces the client's whole state atomically |
| `leave_wire_shape_matches_the_frozen_types` | Pins the agreed `room.leave` wire shape: always `"payload": {}` in both directions (D4 — resolved in the base, doc 02 §9) |

The file, complete (verified green against the frozen contracts):

```rust
//! Layer (a): serialization tests for the network layer's wire contract (B3).
//!
//! These tests need ONLY the `protocol` module and `rooms::testkit` — they
//! compile and run TODAY, before the router exists. They extend the pattern
//! of `backend/tests/protocol_contract.rs`. Target location when B3 adopts
//! them: `backend/tests/network_protocol.rs` (coordinate the file with B4,
//! owner of `backend/tests/`).
//!
//! Run: `cargo test --test network_protocol`

use rechess_backend::protocol::*;
use rechess_backend::rooms::testkit::sample_protocol_snapshot;
use serde_json::Value;

/// What B3's parse gate (future `network::dispatcher`) rejects, and why.
/// This is the SPEC of the 3-step gate in
/// `src/network/02-protocol-implementation.md` §2.1 — when you implement
/// `dispatcher::parse_command`, copy this logic (or import these tests).
#[derive(Debug, PartialEq, Eq)]
enum GateRejection {
    UnsupportedVersion,
    InvalidPayload,
}

/// The parse gate: text frame -> validated ClientEnvelope.
/// Step 1: raw JSON must parse. Step 2: `v` must be a number; any number
/// other than PROTOCOL_VERSION => UNSUPPORTED_VERSION (checked BEFORE the
/// command shape, so a v2 client gets the right error even for unknown
/// v2 commands). Step 3: the command itself must deserialize.
fn parse_command(text: &str) -> Result<ClientEnvelope, GateRejection> {
    let value: Value = serde_json::from_str(text).map_err(|_| GateRejection::InvalidPayload)?;
    match value.get("v") {
        Some(Value::Number(n)) if n.as_u64() == Some(PROTOCOL_VERSION as u64) => {}
        Some(Value::Number(_)) => return Err(GateRejection::UnsupportedVersion),
        _ => return Err(GateRejection::InvalidPayload),
    }
    serde_json::from_value(value).map_err(|_| GateRejection::InvalidPayload)
}

/// Serialize an envelope and hand back a Value for field assertions.
fn wire(envelope: &ServerEnvelope) -> Value {
    serde_json::to_value(envelope).expect("serializes")
}

/// Minimal public room state fixture.
fn room_public() -> RoomPublicState {
    RoomPublicState {
        room_code: "TESTCODE1".into(),
        status: RoomStatus::Waiting,
        players: vec![PlayerPublic {
            id: "host".into(),
            nickname: "Host".into(),
            color: PlayerColor::White,
            connected: true,
        }],
    }
}

/// Every ServerEvent variant serializes with its exact wire `type` string.
#[test]
fn every_server_event_serializes_with_its_wire_type() {
    let snap = sample_protocol_snapshot();
    let events: Vec<(ServerEvent, &str)> = vec![
        (
            ServerEvent::RoomCreated {
                room_code: "TESTCODE1".into(),
                player_id: "host".into(),
                player_token: "host-token".into(),
                color: PlayerColor::White,
                room: room_public(),
            },
            "room.created",
        ),
        (
            ServerEvent::RoomJoined {
                player_id: "guest".into(),
                player_token: "guest-token".into(),
                color: PlayerColor::Black,
                room: room_public(),
            },
            "room.joined",
        ),
        (
            ServerEvent::SessionReady {
                player_id: "host".into(),
                color: PlayerColor::White,
                snapshot: snap.clone(),
            },
            "session.ready",
        ),
        (ServerEvent::GameState { snapshot: snap.clone() }, "game.state"),
        (
            ServerEvent::PlayerPresence { player_id: "guest".into(), connected: false },
            "player.presence",
        ),
        (
            ServerEvent::MoveRejected {
                error: ErrorCode::IllegalMove,
                message: "illegal move".into(),
                snapshot: Some(snap.clone()),
            },
            "game.move_rejected",
        ),
        (
            ServerEvent::GameFinished {
                result: GameResult { winner: Some(PlayerColor::White), reason: GameResultReason::Checkmate },
                snapshot: snap.clone(),
            },
            "game.finished",
        ),
        (
            ServerEvent::ServerError { error: ErrorCode::Internal, message: "internal".into() },
            "server.error",
        ),
    ];
    for (event, expected) in events {
        let value = serde_json::to_value(&event).expect("serializes");
        assert_eq!(value["type"], expected, "wrong wire type, got: {value}");
    }
}

/// `game.state`'s payload IS the flattened snapshot: fields sit directly in
/// `payload`, never nested under a `"snapshot"` key.
#[test]
fn game_state_payload_is_the_flattened_snapshot() {
    let envelope = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: Some(13),
        event: ServerEvent::GameState { snapshot: sample_protocol_snapshot() },
    };
    let value = wire(&envelope);
    assert_eq!(value["type"], "game.state");
    assert_eq!(value["revision"], 13);
    for field in [
        "room_code", "status", "game_status", "fen", "turn", "last_move", "move_history",
        "legal_moves", "players", "draw_offer",
    ] {
        assert!(value["payload"].get(field).is_some(), "missing snapshot field {field}");
    }
    assert!(value["payload"].get("snapshot").is_none(), "snapshot must not be nested");
}

/// Envelope `revision` is optional: present on stateful events, absent when
/// `None` (skip_serializing_if), and stable across a round trip.
#[test]
fn envelope_revision_is_omitted_when_none() {
    let with = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: Some(4),
        event: ServerEvent::GameState { snapshot: sample_protocol_snapshot() },
    };
    assert!(wire(&with).get("revision").is_some());

    let without = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: None,
        event: ServerEvent::PlayerPresence { player_id: "host".into(), connected: true },
    };
    let value = wire(&without);
    assert!(value.get("revision").is_none(), "revision must be absent, got: {value}");
    let back: ServerEnvelope = serde_json::from_str(&serde_json::to_string(&without).unwrap()).unwrap();
    assert_eq!(back.revision, None);
}

/// `player_token` NEVER appears in any public event serialized through the
/// full envelope (broadcasts, presence, rejections, finished, errors).
#[test]
fn token_never_appears_in_public_event_envelopes() {
    let snap = sample_protocol_snapshot();
    let publics = vec![
        ServerEnvelope {
            v: PROTOCOL_VERSION,
            revision: None,
            event: ServerEvent::SessionReady { player_id: "host".into(), color: PlayerColor::White, snapshot: snap.clone() },
        },
        ServerEnvelope { v: PROTOCOL_VERSION, revision: Some(1), event: ServerEvent::GameState { snapshot: snap.clone() } },
        ServerEnvelope { v: PROTOCOL_VERSION, revision: None, event: ServerEvent::PlayerPresence { player_id: "host".into(), connected: false } },
        ServerEnvelope {
            v: PROTOCOL_VERSION,
            revision: Some(2),
            event: ServerEvent::MoveRejected { error: ErrorCode::StaleRevision, message: "stale".into(), snapshot: None },
        },
        ServerEnvelope {
            v: PROTOCOL_VERSION,
            revision: Some(3),
            event: ServerEvent::GameFinished { result: GameResult { winner: None, reason: GameResultReason::DrawAgreed }, snapshot: snap.clone() },
        },
        ServerEnvelope { v: PROTOCOL_VERSION, revision: None, event: ServerEvent::ServerError { error: ErrorCode::Internal, message: "x".into() } },
    ];
    for envelope in publics {
        let text = serde_json::to_string(&envelope).unwrap();
        assert!(!text.contains("token"), "token leaked into a public event: {text}");
    }
}

/// Private replies DO carry the token, in the payload, addressed to one seat.
#[test]
fn private_replies_carry_the_token_in_their_payload() {
    let created = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: None,
        event: ServerEvent::RoomCreated {
            room_code: "TESTCODE1".into(),
            player_id: "host".into(),
            player_token: "host-token".into(),
            color: PlayerColor::White,
            room: room_public(),
        },
    };
    let value = wire(&created);
    assert_eq!(value["type"], "room.created");
    assert_eq!(value["payload"]["player_token"], "host-token");
    assert_eq!(value["payload"]["color"], "white");
}

/// The parse gate: malformed JSON / wrong field shapes => INVALID_PAYLOAD.
#[test]
fn gate_rejects_malformed_input_as_invalid_payload() {
    let bad = [
        "",
        "{",
        "not json",
        r#"{ "v": 1 }"#,                              // missing id/type/payload
        r#"{ "v": 1, "id": "x" }"#,                    // no command at all
        r#"{ "v": "one", "id": "x", "type": "room.create", "payload": {} }"#, // non-numeric v
        r#"{ "v": 1, "id": "x", "type": "game.move", "payload": {} }"#,        // missing move fields
    ];
    for input in bad {
        assert_eq!(parse_command(input), Err(GateRejection::InvalidPayload), "input: {input:?}");
    }
}

/// The parse gate: a numeric but unsupported `v` => UNSUPPORTED_VERSION,
/// decided BEFORE command shape (a v2 client with v2-only commands still
/// gets UNSUPPORTED_VERSION, not INVALID_PAYLOAD).
#[test]
fn gate_rejects_unknown_version_as_unsupported_version() {
    let v2_move = r#"{ "v": 2, "id": "x", "type": "game.move", "payload": { "from": "e2", "to": "e4", "promotion": null, "expected_revision": 1 } }"#;
    assert_eq!(parse_command(v2_move), Err(GateRejection::UnsupportedVersion));
    let v2_unknown_type = r#"{ "v": 2, "id": "x", "type": "pizza.order", "payload": {} }"#;
    assert_eq!(parse_command(v2_unknown_type), Err(GateRejection::UnsupportedVersion));
}

/// The parse gate: v1 + unknown command type => INVALID_PAYLOAD (version is
/// fine, the payload is not).
#[test]
fn gate_rejects_unknown_command_type_as_invalid_payload() {
    let unknown = r#"{ "v": 1, "id": "x", "type": "pizza.order", "payload": {} }"#;
    assert_eq!(parse_command(unknown), Err(GateRejection::InvalidPayload));
}

/// The parse gate accepts every v1 command exactly as the client sends it.
#[test]
fn gate_accepts_every_v1_command() {
    let texts = [
        r#"{ "v": 1, "id": "a", "type": "room.create", "payload": { "nickname": "Host" } }"#,
        r#"{ "v": 1, "id": "b", "type": "room.join", "payload": { "room_code": "TESTCODE1", "nickname": "Guest" } }"#,
        r#"{ "v": 1, "id": "c", "type": "session.resume", "payload": { "room_code": "TESTCODE1", "player_token": "host-token" } }"#,
        r#"{ "v": 1, "id": "d", "type": "game.move", "payload": { "from": "e2", "to": "e4", "promotion": null, "expected_revision": 12 } }"#,
        r#"{ "v": 1, "id": "e", "type": "game.offer_draw", "payload": { "expected_revision": 12 } }"#,
        r#"{ "v": 1, "id": "f", "type": "game.accept_draw", "payload": { "expected_revision": 13 } }"#,
        r#"{ "v": 1, "id": "g", "type": "game.resign", "payload": { "expected_revision": 13 } }"#,
        r#"{ "v": 1, "id": "h", "type": "game.rematch", "payload": { "expected_revision": 14 } }"#,
        // room.leave is the one UNIT variant: today's frozen types accept a
        // missing payload key or null (NOT `{}` — see the open issue below).
        r#"{ "v": 1, "id": "i", "type": "room.leave", "payload": null }"#,
    ];
    for text in texts {
        assert!(parse_command(text).is_ok(), "gate rejected a valid v1 command: {text}");
    }
}

/// Every state-changing command carries `expected_revision` on the wire —
/// B3 forwards it into the RoomCommand unchanged.
#[test]
fn every_state_changing_command_serializes_expected_revision() {
    let commands = [
        ClientCommand::MakeMove { from: "e2".into(), to: "e4".into(), promotion: None, expected_revision: 12 },
        ClientCommand::OfferDraw { expected_revision: 12 },
        ClientCommand::AcceptDraw { expected_revision: 13 },
        ClientCommand::Resign { expected_revision: 13 },
        ClientCommand::Rematch { expected_revision: 14 },
    ];
    for command in commands {
        let envelope = ClientEnvelope { v: PROTOCOL_VERSION, id: "x".into(), command };
        let value = serde_json::to_value(&envelope).unwrap();
        assert!(
            value["payload"].get("expected_revision").is_some(),
            "missing expected_revision: {value}"
        );
    }
}

/// `game.move` always serializes `promotion` (null when not a promotion) —
/// F3 relies on the key being present.
#[test]
fn move_command_always_carries_promotion() {
    let mv = ClientEnvelope {
        v: PROTOCOL_VERSION,
        id: "y".into(),
        command: ClientCommand::MakeMove { from: "e7".into(), to: "e8".into(), promotion: None, expected_revision: 3 },
    };
    let value = serde_json::to_value(&mv).unwrap();
    assert_eq!(value["payload"]["promotion"], Value::Null);
}

/// `room.leave` is the one unit variant — TODAY's frozen Rust types omit the
/// `payload` key on the wire and accept it absent or null, while
/// docs/protocol-v1.md §3 (`room.leave | {}`) and F3's
/// `frontend/src/types/protocol.ts` (`payload: EmptyPayload`) specify `{}`.
/// This test pins the REAL behavior; it is the OPEN ISSUE documented in
/// `src/network/02-protocol-implementation.md` §9 — resolve it in the
/// Day-1 freeze review with F3 via the protocol change rule, then flip this
/// test to the agreed shape.
#[test]
fn leave_wire_shape_matches_the_frozen_types() {
    let leave = ClientEnvelope {
        v: PROTOCOL_VERSION,
        id: "x".into(),
        command: ClientCommand::LeaveRoom {},
    };
    let value = serde_json::to_value(&leave).unwrap();
    assert_eq!(value["type"], "room.leave");
    assert_eq!(value["payload"], serde_json::json!({}), "payload is always {}: {value}");

    // The agreed shape (D4, resolved in the base): "payload": {} in both
    // directions. Verified against the frozen types: {} parses; null and
    // absent are rejected.
    let braces = r#"{ "v": 1, "id": "i", "type": "room.leave", "payload": {} }"#;
    assert!(parse_command(braces).is_ok());
    let null = r#"{ "v": 1, "id": "i", "type": "room.leave", "payload": null }"#;
    assert_eq!(parse_command(null), Err(GateRejection::InvalidPayload));
    let absent = r#"{ "v": 1, "id": "i", "type": "room.leave" }"#;
    assert_eq!(parse_command(absent), Err(GateRejection::InvalidPayload));
}

/// `server.error` shape: stable SCREAMING_SNAKE code in the payload, safe
/// message, no revision on the envelope.
#[test]
fn server_error_envelope_has_stable_code_and_safe_message() {
    let envelope = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: None,
        event: ServerEvent::ServerError { error: ErrorCode::UnsupportedVersion, message: "protocol v1 supported only".into() },
    };
    let value = wire(&envelope);
    assert_eq!(value["type"], "server.error");
    assert_eq!(value["payload"]["error"], "UNSUPPORTED_VERSION");
    assert!(value.get("revision").is_none());
}

/// `session.ready` carries the FULL snapshot — the resume path replaces the
/// client's whole local state in one message.
#[test]
fn session_ready_carries_the_full_snapshot() {
    let snap = sample_protocol_snapshot();
    let envelope = ServerEnvelope {
        v: PROTOCOL_VERSION,
        revision: None,
        event: ServerEvent::SessionReady { player_id: "host".into(), color: PlayerColor::White, snapshot: snap.clone() },
    };
    let back: ServerEnvelope = serde_json::from_str(&serde_json::to_string(&envelope).unwrap()).unwrap();
    match back.event {
        ServerEvent::SessionReady { snapshot, .. } => assert_eq!(snapshot, snap),
        other => panic!("wrong event after round trip: {other:?}"),
    }
}
```

### 2.3 Unit tests inside `src/network/` (`cargo test network::`)

Layer (a) covers the wire; your module also owns pure logic that deserves
plain `#[cfg(test)]` tests inside the module files (run by
`cargo test network::`):

- `dispatcher.rs`: nickname validation (empty, 21 chars, control chars,
  Unicode within 20 chars, trim semantics), room-code normalization
  (`" testcode1 "` → `TESTCODE1`), malformed shapes, rate bucket behavior.
- the four converters of doc 02 §7.3 (`to_domain_promotion`,
  `to_wire_color`, `to_wire_result`, `to_wire_reason`) — exhaustively, they
  are pure functions.
- target filtering: a `RoomEvent::to("guest", …)` event is dropped for the
  host connection, delivered for the guest connection.

## 3. Layer (b) — the two-client WebSocket harness: `backend/tests/network_ws.rs`

**This file does not compile today**, for two reasons — land both in ONE PR:

1. `network::router` must exist (doc 01 §2, Phase 1).
2. **`tokio-tungstenite` is NOT in `Cargo.toml`** — `tests/README.md` §2
   already anticipates it ("optionally: tokio-tungstenite (raw WS client)").
   Adding a dev-dependency is a `Cargo.toml` PR — coordinate with B4 (they
   own the dev-dep section; `tests/README.md` says the additions are yours
   to propose). Regular dependencies (`axum`, `tokio`, `serde_json`,
   `futures-util`, `uuid`) are already available to integration tests
   without changes:

   ```toml
   [dev-dependencies]
   # Pin the version you verified, and commit Cargo.lock (repo rule).
   tokio-tungstenite = "0.27"   # raw WS client for the integration harness
   # (B4 may also want: tower = { version = "0.5", features = ["util"] } for
   #  Router::oneshot tests — their call, see tests/README.md §2.)
   ```

   Version note: check the `Message` enum of the version Cargo resolves —
   since tungstenite 0.26, `Message::Text` wraps `Utf8Bytes` (use
   `Message::text(...)` to construct and `.as_str()` to read — both work on
   older `String`-based versions too) and a `Frame` variant exists. Adjust
   the match arms in `next_json` if your resolved version differs.

The harness (target code — every assertion maps to a requirement in doc 04):

```rust
//! Layer (b): two-client WebSocket integration harness (B3, with B4).
//!
//! REQUIRES: (1) `network::router` exists
//!           (src/network/01-axum-websocket-guide.md §2 Phase 1),
//!           (2) `tokio-tungstenite` as a dev-dependency (§3 above).
//! Run: `cargo test --test network_ws`

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rechess_backend::network::router;
use rechess_backend::protocol::{ErrorCode, PROTOCOL_VERSION};
use rechess_backend::rooms::testkit::{sample_protocol_snapshot, FakeRegistry};
use rechess_backend::rooms::{CommandOutcome, RoomCommand, RoomEvent, RoomEventKind};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

type Client = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

/// The app on an ephemeral port, backed by the scripted fake registry.
/// (After doc 01 §2 Phase 2, `build_router(registry)` is equivalent —
/// it just merges the health route on top of this same router.)
async fn spawn_server(registry: FakeRegistry) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = router(registry);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn connect(addr: std::net::SocketAddr) -> Client {
    let (stream, _response) =
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws")).await.expect("connect");
    stream
}

async fn send_json(client: &mut Client, value: Value) {
    client.send(Message::Text(value.to_string().into())).await.expect("send");
}

/// Next JSON event; skips ping/pong; panics on binary/close/raw frames.
async fn next_json(client: &mut Client) -> Value {
    loop {
        let message = client.next().await.expect("stream open").expect("frame ok");
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str()).expect("server JSON must parse");
            }
            Message::Ping(_) | Message::Pong(_) => continue, // heartbeat noise
            Message::Close(frame) => panic!("unexpected close: {frame:?}"),
            Message::Binary(bytes) => panic!("unexpected binary frame: {bytes:?}"),
            Message::Frame(frame) => panic!("unexpected raw frame: {frame:?}"),
        }
    }
}

/// Bind a connection as the host (White). Returns the private reply event.
async fn bind_host(client: &mut Client) -> Value {
    send_json(client, json!({
        "v": PROTOCOL_VERSION, "id": "cmd-create", "type": "room.create",
        "payload": { "nickname": "Host" }
    })).await;
    let created = next_json(client).await;
    assert_eq!(created["type"], "room.created", "got: {created}");
    created
}

/// Bind a connection as the guest (Black). The room code is deliberately
/// lowercase + padded: the gate must normalize (trim + uppercase).
async fn bind_guest(client: &mut Client) -> Value {
    send_json(client, json!({
        "v": PROTOCOL_VERSION, "id": "cmd-join", "type": "room.join",
        "payload": { "room_code": "  testcode1 ", "nickname": "Guest" }
    })).await;
    let joined = next_json(client).await;
    assert_eq!(joined["type"], "room.joined", "got: {joined}");
    joined
}

/// create -> join -> move -> BOTH clients receive the same game.state with
/// the same revision; the bound identity (not a client claim) is injected.
#[tokio::test]
async fn two_clients_create_join_move_and_see_the_same_state() {
    let registry = FakeRegistry::new("TESTCODE1");
    let fake = registry.handle.clone(); // the test plays the actor's event half
    let addr = spawn_server(registry).await;

    let mut host = connect(addr).await;
    let mut guest = connect(addr).await;

    let created = bind_host(&mut host).await;
    assert_eq!(created["payload"]["room_code"], "TESTCODE1");
    assert_eq!(created["payload"]["color"], "white");
    assert!(!created["payload"]["player_token"].as_str().unwrap().is_empty());

    let joined = bind_guest(&mut guest).await;
    assert_eq!(joined["payload"]["color"], "black");

    // Host moves. The fake's default outcome is Accepted { revision: 0 } —
    // doc 02 §7.2: B3 sends NOTHING for an accepted command.
    send_json(&mut host, json!({
        "v": PROTOCOL_VERSION, "id": "cmd-3", "type": "game.move",
        "payload": { "from": "e2", "to": "e4", "promotion": null, "expected_revision": 0 }
    })).await;

    // The actor's other half: broadcast the new snapshot. Both connections
    // subscribed at bind time, BEFORE their private replies (doc 01 §4's
    // ordering rule), so this cannot race.
    fake.emit(RoomEvent::broadcast(RoomEventKind::Snapshot {
        revision: 1,
        snapshot: sample_protocol_snapshot(),
    }));

    let host_state = next_json(&mut host).await;
    let guest_state = next_json(&mut guest).await;
    for state in [&host_state, &guest_state] {
        assert_eq!(state["type"], "game.state", "got: {state}");
        assert_eq!(state["revision"], 1);
        assert!(state["payload"]["fen"].is_string(), "payload IS the snapshot: {state}");
    }
    assert_eq!(host_state["payload"], guest_state["payload"]);
    assert!(
        !serde_json::to_string(&host_state).unwrap().contains("token"),
        "no token in a broadcast event"
    );

    // The gate injected the BOUND identity into the RoomCommand (doc 02 §5.2).
    let received = fake.received();
    assert_eq!(received.len(), 1);
    match &received[0] {
        RoomCommand::Move { player, from, to, promotion, expected_revision } => {
            assert_eq!(player, "host");
            assert_eq!(from, "e2");
            assert_eq!(to, "e4");
            assert_eq!(*promotion, None);
            assert_eq!(*expected_revision, 0);
        }
        other => panic!("expected a Move command, got {other:?}"),
    }
}

/// A scripted rejection reaches the MOVER only — the opponent receives
/// nothing — and carries the envelope-level revision from the outcome.
#[tokio::test]
async fn rejected_move_reaches_the_mover_only() {
    let registry = FakeRegistry::new("TESTCODE1");
    let fake = registry.handle.clone();
    let addr = spawn_server(registry).await;

    let mut host = connect(addr).await;
    let mut guest = connect(addr).await;
    bind_host(&mut host).await;
    bind_guest(&mut guest).await;

    fake.push_outcome(CommandOutcome::Rejected {
        error: ErrorCode::NotYourTurn,
        message: "not your turn".into(),
        revision: 3,
    });
    send_json(&mut host, json!({
        "v": PROTOCOL_VERSION, "id": "cmd-9", "type": "game.move",
        "payload": { "from": "e7", "to": "e5", "promotion": null, "expected_revision": 3 }
    })).await;

    let rejected = next_json(&mut host).await;
    assert_eq!(rejected["type"], "game.move_rejected", "got: {rejected}");
    assert_eq!(rejected["payload"]["error"], "NOT_YOUR_TURN");
    assert_eq!(rejected["revision"], 3); // envelope-level, from the outcome
    assert!(rejected["payload"].get("snapshot").is_some()); // key present (null or object)

    // The opponent receives NOTHING for the mover's failure.
    let nothing = tokio::time::timeout(Duration::from_millis(150), guest.next()).await;
    assert!(nothing.is_err(), "game.move_rejected leaked to the opponent");
}

/// Auth gate: an UNBOUND connection may only create/join/resume.
#[tokio::test]
async fn unbound_connection_rejects_in_room_commands() {
    let registry = FakeRegistry::new("TESTCODE1");
    let addr = spawn_server(registry).await;
    let mut client = connect(addr).await;

    send_json(&mut client, json!({
        "v": PROTOCOL_VERSION, "id": "cmd-1", "type": "game.move",
        "payload": { "from": "e2", "to": "e4", "promotion": null, "expected_revision": 0 }
    })).await;
    let error = next_json(&mut client).await;
    assert_eq!(error["type"], "server.error", "got: {error}");
    assert_eq!(error["payload"]["error"], "INVALID_PAYLOAD");
}

/// Version gate: structured UNSUPPORTED_VERSION first, THEN the close.
#[tokio::test]
async fn unsupported_version_gets_structured_error_then_close() {
    let registry = FakeRegistry::new("TESTCODE1");
    let addr = spawn_server(registry).await;
    let mut client = connect(addr).await;

    send_json(&mut client, json!({
        "v": 99, "id": "cmd-1", "type": "room.create",
        "payload": { "nickname": "A" }
    })).await;

    let error = next_json(&mut client).await;
    assert_eq!(error["type"], "server.error");
    assert_eq!(error["payload"]["error"], "UNSUPPORTED_VERSION");

    let closed = tokio::time::timeout(Duration::from_millis(500), client.next()).await;
    match closed {
        Ok(Some(Ok(Message::Close(_)))) => {} // expected: error first, then close
        other => panic!("expected close frame after UNSUPPORTED_VERSION, got {other:?}"),
    }
}

/// Malformed frames are answered with INVALID_PAYLOAD but do NOT kill the
/// connection (budget: 3 — doc 02 §2.2).
#[tokio::test]
async fn malformed_frame_is_answered_but_does_not_kill_the_connection() {
    let registry = FakeRegistry::new("TESTCODE1");
    let addr = spawn_server(registry).await;
    let mut client = connect(addr).await;

    client.send(Message::Text("not json".into())).await.unwrap();
    let error = next_json(&mut client).await;
    assert_eq!(error["type"], "server.error");
    assert_eq!(error["payload"]["error"], "INVALID_PAYLOAD");

    // Still usable: bind right after the garbage frame.
    send_json(&mut client, json!({
        "v": PROTOCOL_VERSION, "id": "cmd-2", "type": "room.create",
        "payload": { "nickname": "Host" }
    })).await;
    let created = next_json(&mut client).await;
    assert_eq!(created["type"], "room.created");
}
```

Notes on the harness design, so you can extend it without breaking it:

- The fake's `send` returns scripted outcomes but emits no events by
  itself — **the test plays the actor's event half** with
  `fake.emit(...)`. That split is the whole point of layer (b): it tests
  YOUR mapping (outcome → event, event → envelope, target filtering) without
  B2's actor.
- Once B2's real registry lands, add a second harness (or extend this one)
  that swaps `FakeRegistry` for the real one — the generic router makes it a
  one-line change. B4 owns the 50-room benchmark harness
  (`tests/README.md` §3); build on your shared helpers, don't fork them.
- Timing assertions use short timeouts (150/500 ms), not sleeps — and never
  `tokio::time::pause()` across a real socket (it only advances with
  in-process timers).

## 4. Manual smoke test (websocat)

For when you want to see the bytes with your own eyes (also the demo script
B4 will ask for):

```bash
# One-time install (uses the sandbox toolchain):
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cargo install websocat   # binary lands in /tmp/rust/cargo/bin

# Terminal 1 — the server:
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo run

# Terminal 2 — health first (B4's gate):
curl http://127.0.0.1:3000/health
# → {"status":"ok","service":"rechess-backend"}

# Terminal 2 — client A (host). websocat sends each pasted LINE as one text
# message, which is exactly what the protocol wants:
websocat ws://127.0.0.1:3000/ws
{"v":1,"id":"smoke-1","type":"room.create","payload":{"nickname":"SmokeHost"}}
# expect a private {"type":"room.created","payload":{...,"player_token":"…","color":"white",...}}
# note the room_code in the reply.

# Terminal 3 — client B (guest), with the code from A's reply:
websocat ws://127.0.0.1:3000/ws
{"v":1,"id":"smoke-2","type":"room.join","payload":{"room_code":"XXXXXXXX","nickname":"SmokeGuest"}}
# expect {"type":"room.joined","payload":{...,"color":"black",...}}

# Terminal 3 — guest tries to move first (host is White):
{"v":1,"id":"smoke-3","type":"game.move","payload":{"from":"e7","to":"e5","promotion":null,"expected_revision":0}}
# expect game.move_rejected NOT_YOUR_TURN in terminal 3 ONLY (terminal 2 stays silent)

# Terminal 2 — host moves:
{"v":1,"id":"smoke-4","type":"game.move","payload":{"from":"e2","to":"e4","promotion":null,"expected_revision":0}}
# expect BOTH terminals to receive {"v":1,"type":"game.state","revision":1,"payload":{…}} (identical payloads)

# Gate checks (any terminal):
not json at all            # → server.error INVALID_PAYLOAD, connection stays open
{"v":2,"id":"x","type":"room.create","payload":{"nickname":"A"}}   # → server.error UNSUPPORTED_VERSION, then close
{"v":1,"id":"x","type":"pizza.order","payload":{}}                 # → server.error INVALID_PAYLOAD
```

Caveat: until doc 01 §2 Phase 2 (B2's real registry), the running binary
answers the gates but cannot really create rooms — for full-flow manual
smoke before Phase 2, run a tiny throwaway binary that serves
`network::router(FakeRegistry::new("TESTCODE1"))` (or coordinate with B4
for an `examples/` file). The two-terminal create/join/move part of the
script is for after Phase 2.

## 5. Coverage checklist (what must be green before doc 04's gate)

| Requirement (source) | Covered by |
|---|---|
| Every event serializes with the right `type`; envelope revision placement (docs §2.2, §4) | layer (a), tests 1–3 |
| Token never in public events; only in the two private replies (docs §8.1) | layer (a), tests 4–5 |
| Malformed input → structured `INVALID_PAYLOAD`, no panic (docs §8.4) | layer (a), tests 6–8; layer (b), malformed test |
| Unsupported version → `UNSUPPORTED_VERSION`, then close (docs §8.4) | layer (a), test 7; layer (b), version test |
| Nickname/room-code gate + normalization (docs §3) | `cargo test network::` units; layer (b), `bind_guest` normalization |
| Two clients see the same revision for the same accepted move (Master Plan §13.1) | layer (b), test 1 |
| Rejection reaches the mover only (docs §4.2) | layer (b), test 2 |
| A room code alone is not authorization (brief §2) | layer (b), unbound test + injected-identity assertion |
| Duplicate IDs / stale revisions (docs §5) | B2's actor + D1 decision (doc 02 §5.3) — add cases when D1 lands |
