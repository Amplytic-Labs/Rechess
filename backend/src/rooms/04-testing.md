# 04 — Testing: The Suite You Ship

> **STATUS — read this first.** These tests are for **B2's FUTURE
> implementation**. Today only the **fakes'** tests exist and run green
> (`backend/tests/testkit_demo.rs` — 4 tests against
> `rooms::testkit::FakeRegistry`/`FakeHandle`). Nothing on this page compiles
> today because `InMemoryRegistry`, `ActorHandle`, `RoomConfig`,
> `EngineFactory` and `SharedEngine` are **the types you build** per
> [02-architecture.md](02-architecture.md) and
> [03-implementation-guide.md](03-implementation-guide.md). Your job: implement
> the module so this suite (adjust names/paths to your final ones) runs green.
> Use `game::testkit::{FakeEngine, starting_snapshot}` and `rooms::testkit`
> only as **reference for how fakes work** — your tests exercise **your REAL
> registry and actors** through the frozen public traits, with `FakeEngine`
> injected as *B1's engine seam*. If you prototyped anything against
> `FakeRegistry`/`FakeHandle`, switch those fixtures to your real types and
> delete the leftovers — testing the fakes is B3/B4's job, not yours.

## 1. Test infrastructure decisions (make these first)

### 1.1 One dev-dependency PR you will need

Paused-time tests (`#[tokio::test(start_paused = true)]`,
`tokio::time::advance`) need tokio's `test-util` feature. Open (or have B4
open) the PR adding to `backend/Cargo.toml`:

```toml
[dev-dependencies]
tokio = { version = "1.54.1", features = ["test-util"] }
```

Features are additive for test builds only — the shipped binary is unaffected.
If the PR is blocked, the fallback is real short durations via `RoomConfig`
(e.g. `disconnect_grace: Duration::from_millis(150)`) with generous
assertion windows — flakier, so prefer paused time.

### 1.2 `RoomConfig` short knobs

All deadline tests use the paused clock, but keep every knob in `RoomConfig`
anyway (B4's 50-room and expiry tests need millisecond knobs).

### 1.3 Scripting the engine inside the actor — `SharedEngine`

Your actor owns a `Box<dyn ChessEngine>`, so a plain `FakeEngine` made in the
test is unreachable. Wrap it in an `Arc<Mutex<…>>` adapter (exact code —
verified to compile against the real `ChessEngine` trait):

```rust
/// Test adapter: the actor owns the `SharedEngine`; the test keeps the
/// `Arc<Mutex<FakeEngine>>` to script failures and inspect calls.
struct SharedEngine(Arc<Mutex<FakeEngine>>);

impl rechess_backend::game::ChessEngine for SharedEngine {
    fn apply_move(&mut self, request: MoveRequest)
        -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().apply_move(request)
    }
    fn offer_draw(&mut self, by: DomainColor)
        -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().offer_draw(by)
    }
    fn accept_draw(&mut self, by: DomainColor)
        -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().accept_draw(by)
    }
    fn resign(&mut self, by: DomainColor)
        -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().resign(by)
    }
    fn reset_for_rematch(&mut self) { self.0.lock().unwrap().reset_for_rematch() }
    fn snapshot(&self) -> rechess_backend::game::GameSnapshot {
        self.0.lock().unwrap().snapshot()
    }
}
```

This satisfies the "must never panic on bad input" contract (all calls
delegate) and gives the test two levers: set `.snapshot` to what the next
successful engine call should return, and `fail_next_with(err)` to script a
rejection. Note for your actor design: **take the `Finished` result from the
engine's returned snapshot** (`snap.result`), so this scripting works.

### 1.4 The registry fixture

```rust
fn scripted_registry() -> (InMemoryRegistry, Arc<Mutex<FakeEngine>>) {
    let inner = Arc::new(Mutex::new(FakeEngine::new(starting_snapshot())));
    let engine_for_factory = inner.clone();
    let new_engine: EngineFactory =
        Arc::new(move || Box::new(SharedEngine(engine_for_factory.clone())));
    (InMemoryRegistry::new(RoomConfig::default(), new_engine), inner)
}
```

Caution: this factory hands **every room the same engine instance** — fine
for these single-room tests. A multi-room isolation test must use a factory
that builds a fresh `FakeEngine` per room (assert via events only).

## 2. In-module unit tests — `#[cfg(test)]` inside your files

Run with:

```bash
cargo test rooms::
```

Place each test next to the code it tests. Today this filter matches zero
tests; after your work it matches all of yours. Three to start with:

### 2.1 `mapping.rs` — the worked example (pure, no actor)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::testkit::starting_snapshot;
    use crate::game::{GameResult as DomainResult, GameResultReason as DomainReason,
        PlayerColor as DomainColor};
    use crate::protocol::{GameStatus, PlayerColor, RoomStatus};

    #[test]
    fn worked_example_after_e4_maps_to_the_wire_fixture() {
        let mut engine = starting_snapshot();       // after 1. e4 (fake domain snapshot)
        engine.turn = DomainColor::Black;
        engine.last_move = Some(crate::game::MoveRecord {
            from: "e2".into(), to: "e4".into(), san: "e4".into() });
        engine.history_san = vec!["e4".into()];
        engine.legal_moves = vec!["e7e5".into(), "e7e6".into()];

        let wire = build_snapshot(
            "TESTCODE1", RoomStatus::Active,
            &[SeatView { player_id: "p-host".into(), nickname: "Host".into(),
                         color: DomainColor::White, connected: true },
              SeatView { player_id: "p-guest".into(), nickname: "Guest".into(),
                         color: DomainColor::Black, connected: true }],
            &engine,
        );

        assert_eq!(wire.room_code, "TESTCODE1");
        assert_eq!(wire.status, RoomStatus::Active);
        assert_eq!(wire.game_status, GameStatus::InProgress);
        assert_eq!(wire.turn, PlayerColor::Black);
        assert_eq!(wire.last_move.map(|m| m.san), Some("e4".into()));
        assert_eq!(wire.move_history, vec!["e4".to_string()]);
        assert_eq!(wire.players.len(), 2);
        assert_eq!(wire.players[1].color, PlayerColor::Black);
        assert_eq!(wire.draw_offer, None);
        assert!(!format!("{wire:?}").contains("token"), "no secrets ever");
    }

    #[test]
    fn finished_domain_result_maps_to_wire() {
        let wire = DomainResult {
            winner: Some(DomainColor::Black),
            reason: DomainReason::Checkmate,
        }.into();
        assert_eq!(wire.winner, Some(PlayerColor::Black));
        assert_eq!(wire.reason, crate::protocol::GameResultReason::Checkmate);
    }
}
```

(Adapt `SeatView`/`build_snapshot`'s signature to your final `RoomState`
version — the real one should take `&RoomState` + `&game::GameSnapshot`.)

### 2.2 `registry.rs` / `room.rs` — code, token and dedup shapes

```rust
#[test]
fn room_codes_are_8_chars_from_the_unambiguous_alphabet() {
    for _ in 0..100 {
        let code = generate_room_code();
        assert_eq!(code.len(), 8);
        assert!(code.bytes().all(|b| CODE_ALPHABET.contains(&b)),
            "ambiguous char in {code}");
    }
}

#[test]
fn player_tokens_are_64_hex_chars_of_32_bytes() {
    let token = generate_player_token();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn dedup_window_is_bounded_and_recent() {
    let mut dedup = BoundedDedupCache::new(2);        // per-player cap, tiny for the test
    assert!(dedup.record_and_check("p1", "cmd-a"));
    assert!(!dedup.record_and_check("p1", "cmd-a"), "retry is a duplicate");
    assert!(dedup.record_and_check("p1", "cmd-b"));
    assert!(dedup.record_and_check("p1", "cmd-c"));
    assert!(dedup.record_and_check("p1", "cmd-a"),
        "older than the window → treated as fresh again");
}
```

(The dedup test compiles once the `command_id` contract PR from
[01-contract.md §10](01-contract.md) lands — write the cache behind the
internal type now, wire it into the actor in the PR.)

## 3. Integration tests — `backend/tests/rooms_actor.rs`

A new file owned by you (B4 adds their own integration tests separately).
Run with:

```bash
cargo test --test rooms_actor
```

Written **generically over the traits** where practical (B3's consumption
pattern) and driven only through the public API — registry methods, `send`,
`subscribe`. Revision numbers follow the
[01-contract.md §9](01-contract.md) policy table (create = 0, join = 1,
first accepted move = 2; rematch first vote does not bump).

```rust
//! backend/tests/rooms_actor.rs
//!
//! STATUS: for B2's FUTURE implementation — does not compile until the real
//! registry/actor exist. Then keep it green in every PR.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use rechess_backend::game::testkit::{starting_snapshot, FakeEngine};
use rechess_backend::game::{
    ChessEngine, GameError, GameResult as DomainResult, GameResultReason as DomainReason,
    GameStatus as DomainStatus, MoveRequest, PlayerColor as DomainColor,
};
use rechess_backend::protocol::{ErrorCode, RoomStatus};
use rechess_backend::rooms::config::RoomConfig;
use rechess_backend::rooms::registry::{EngineFactory, InMemoryRegistry};
use rechess_backend::rooms::{
    CommandOutcome, JoinRoomError, PlayerCredentials, ResumeError, RoomCommand, RoomEvent,
    RoomEventKind, RoomHandle as _, RoomRegistry,
};

// --- fixtures (see §1 above) ----------------------------------------------
struct SharedEngine(Arc<Mutex<FakeEngine>>);

impl ChessEngine for SharedEngine {
    fn apply_move(&mut self, request: MoveRequest) -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().apply_move(request)
    }
    fn offer_draw(&mut self, by: DomainColor) -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().offer_draw(by)
    }
    fn accept_draw(&mut self, by: DomainColor) -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().accept_draw(by)
    }
    fn resign(&mut self, by: DomainColor) -> Result<rechess_backend::game::GameSnapshot, GameError> {
        self.0.lock().unwrap().resign(by)
    }
    fn reset_for_rematch(&mut self) { self.0.lock().unwrap().reset_for_rematch() }
    fn snapshot(&self) -> rechess_backend::game::GameSnapshot { self.0.lock().unwrap().snapshot() }
}

fn scripted_registry() -> (InMemoryRegistry, Arc<Mutex<FakeEngine>>) {
    let inner = Arc::new(Mutex::new(FakeEngine::new(starting_snapshot())));
    let engine_for_factory = inner.clone();
    let new_engine: EngineFactory =
        Arc::new(move || Box::new(SharedEngine(engine_for_factory.clone())));
    (InMemoryRegistry::new(RoomConfig::default(), new_engine), inner)
}

/// Create + join. Returns (host, guest, host handle, room_code).
/// Revision is 1 here (join bumps 0 -> 1, per 01-contract.md §9).
async fn open_two_player_room<R: RoomRegistry>(
    registry: &R,
) -> (PlayerCredentials, PlayerCredentials, R::Handle, String) {
    let created = registry.create_room("Host").await.expect("create ok");
    let joined = registry.join_room(&created.room_code, "Guest").await.expect("join ok");
    (created.credentials, joined.credentials, created.handle, created.room_code)
}

fn move_cmd(player: &PlayerCredentials, revision: u64) -> RoomCommand {
    RoomCommand::Move {
        player: player.player_id.clone(),
        from: "e2".into(),
        to: "e4".into(),
        promotion: None,
        expected_revision: revision,
    }
}

// --- 1. create / join / resume / lookup ------------------------------------

#[tokio::test]
async fn create_seats_host_white_in_a_waiting_room() {
    let (registry, _engine) = scripted_registry();
    let created = registry.create_room("Host").await.expect("create ok");

    let code = &created.room_code;
    assert_eq!(code.len(), 8, "room code is 8 chars");
    assert!(code.bytes().all(|b| b"ABCDEFGHJKMNPQRSTUVWXYZ23456789".contains(&b)),
        "unambiguous alphabet only, got {code}");
    assert_eq!(created.credentials.color, DomainColor::White, "host is White");
    assert_eq!(created.room.status, RoomStatus::Waiting);
    assert_eq!(created.room.players.len(), 1);
    assert_eq!(created.room.players[0].color, rechess_backend::protocol::PlayerColor::White);
    assert!(created.room.players[0].connected, "the creating player is connected");
    // The raw token exists ONLY here and in the private reply B3 builds:
    assert!(!created.room_code.contains("token"));
}

#[tokio::test]
async fn join_seats_guest_black_and_activates_the_room() {
    let (registry, _engine) = scripted_registry();
    let created = registry.create_room("Host").await.expect("create ok");
    let joined = registry
        .join_room(&created.room_code, "Guest")
        .await
        .expect("join ok");
    assert_eq!(joined.credentials.color, DomainColor::Black, "guest is Black");
    assert_ne!(created.credentials.player_id, joined.credentials.player_id);
    assert_eq!(joined.room.status, RoomStatus::Active, "waiting -> active on join");
    assert_eq!(joined.room.players.len(), 2);
    // Both handles reach the same room:
    let _events = joined.handle.subscribe();
    assert!(registry.lookup(&created.room_code).await.is_some());
}

#[tokio::test]
async fn join_unknown_code_is_not_found() {
    let (registry, _engine) = scripted_registry();
    let err = registry.join_room("NOPE9999", "Guest").await.unwrap_err();
    assert_eq!(err, JoinRoomError::NotFound);
}

#[tokio::test]
async fn second_join_into_a_full_room_is_rejected() {
    let (registry, _engine) = scripted_registry();
    let (_host, _guest, _handle, code) = open_two_player_room(&registry).await;
    // The room is active/full now — a second guest must not overwrite the seat.
    let late = registry.join_room(&code, "Late").await;
    assert_eq!(late, Err(JoinRoomError::Full), "first join wins, room full");
}

#[tokio::test]
async fn resume_with_the_right_token_returns_the_full_snapshot() {
    let (registry, _engine) = scripted_registry();
    let (host, _guest, _handle, code) = open_two_player_room(&registry).await;
    let resumed = registry
        .resume_room(&code, &host.player_token)
        .await
        .expect("resume ok");
    assert_eq!(resumed.player_id, host.player_id);
    assert_eq!(resumed.color, DomainColor::White);
    assert_eq!(resumed.snapshot.room_code, code);
    assert_eq!(resumed.snapshot.status, RoomStatus::Active);
    assert_eq!(resumed.snapshot.players.len(), 2);
}

#[tokio::test]
async fn resume_with_a_wrong_token_is_invalid_token() {
    let (registry, _engine) = scripted_registry();
    let (_host, _guest, _handle, code) = open_two_player_room(&registry).await;
    let err = registry.resume_room(&code, "not-a-real-token").await.unwrap_err();
    assert_eq!(err, ResumeError::InvalidToken);
}

#[tokio::test]
async fn resume_with_an_unknown_code_is_not_found() {
    let (registry, _engine) = scripted_registry();
    let err = registry.resume_room("NOPE9999", "whatever").await.unwrap_err();
    assert_eq!(err, ResumeError::NotFound);
}

#[tokio::test]
async fn lookup_hits_live_rooms_and_misses_others() {
    let (registry, _engine) = scripted_registry();
    let created = registry.create_room("Host").await.expect("create ok");
    assert!(registry.lookup(&created.room_code).await.is_some(), "hit");
    assert!(registry.lookup("NOPE9999").await.is_none(), "miss");
}

// --- 2. commands through the actor -----------------------------------------

#[tokio::test]
async fn accepted_move_bumps_revision_and_broadcasts_the_snapshot() {
    let (registry, _engine) = scripted_registry();
    let (host, _guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();          // subscribe AFTER join: next event is ours

    let outcome = handle.send(move_cmd(&host, 1)).await.expect("send ok");
    assert_eq!(outcome, CommandOutcome::Accepted { revision: 2 }, "1 (join) + 1 (move)");

    let event = events.recv().await.expect("snapshot broadcast");
    assert_eq!(event.target, None, "snapshots are broadcast");
    match event.kind {
        RoomEventKind::Snapshot { revision, snapshot } => {
            assert_eq!(revision, 2);
            assert_eq!(snapshot.status, RoomStatus::Active);
            assert_eq!(snapshot.move_history, vec!["e4".to_string()], "from the engine's snapshot");
            assert_eq!(snapshot.players.len(), 2);
        }
        other => panic!("expected Snapshot, got {other:?}"),
    }
}

#[tokio::test]
async fn stale_expected_revision_is_rejected_without_state_change() {
    let (registry, _engine) = scripted_registry();
    let (host, _guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    let outcome = handle.send(move_cmd(&host, 5)).await.expect("send ok"); // room is at 1
    assert!(matches!(outcome,
        CommandOutcome::Rejected { error: ErrorCode::StaleRevision, revision: 1, .. }));

    let event = events.recv().await.expect("targeted move_rejected");
    assert_eq!(event.target.as_deref(), Some(host.player_id.as_str()));
    match event.kind {
        RoomEventKind::MoveRejected { error, revision, snapshot, .. } => {
            assert_eq!(error, ErrorCode::StaleRevision);
            assert_eq!(revision, 1, "state untouched");
            let _ = snapshot; // Some(current) recommended — heals a stale client
        }
        other => panic!("expected MoveRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn engine_rejection_targets_only_the_mover() {
    let (registry, engine) = scripted_registry();
    let (host, _guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    engine.lock().unwrap().fail_next_with(GameError::IllegalMove);
    let outcome = handle.send(move_cmd(&host, 1)).await.expect("send ok");
    assert!(matches!(outcome,
        CommandOutcome::Rejected { error: ErrorCode::IllegalMove, revision: 1, .. }));

    let event = events.recv().await.expect("move_rejected is delivered");
    assert_eq!(event.target, Some(host.player_id.clone()), "TARGETED at the mover");
    match event.kind {
        RoomEventKind::MoveRejected { player_id, error, revision, snapshot, .. } => {
            assert_eq!(player_id, host.player_id);
            assert_eq!(error, ErrorCode::IllegalMove);
            assert_eq!(revision, 1);
            assert!(snapshot.is_some(), "rejection carries a healing snapshot");
        }
        other => panic!("expected MoveRejected, got {other:?}"),
    }
}

#[tokio::test]
async fn a_player_not_in_the_room_cannot_act() {
    let (registry, _engine) = scripted_registry();
    let (_host, _guest, handle, _code) = open_two_player_room(&registry).await;
    let intruder = PlayerCredentials {
        player_id: "p-intruder".into(),
        player_token: String::new(),
        color: DomainColor::White,
    };
    let outcome = handle.send(move_cmd(&intruder, 1)).await.expect("send ok");
    assert!(matches!(outcome,
        CommandOutcome::Rejected { error: ErrorCode::InvalidToken, .. }));
}

// --- 3. finishing the game ---------------------------------------------------

#[tokio::test]
async fn draw_offer_then_accept_finishes_with_draw_agreed() {
    let (registry, engine) = scripted_registry();
    let (host, guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    // Host offers (fake returns its snapshot — accepted, draw_offer field comes from it).
    let outcome = handle.send(RoomCommand::OfferDraw {
        player: host.player_id.clone(), expected_revision: 1 }).await.unwrap();
    assert_eq!(outcome, CommandOutcome::Accepted { revision: 2 });
    let _offer_snapshot = events.recv().await.expect("snapshot broadcast");

    // Script what the engine returns on the accepting call: a finished draw.
    let mut finished = starting_snapshot();
    finished.status = DomainStatus::Draw;
    finished.result = Some(DomainResult { winner: None, reason: DomainReason::DrawAgreed });
    engine.lock().unwrap().snapshot = finished;

    let outcome = handle.send(RoomCommand::AcceptDraw {
        player: guest.player_id.clone(), expected_revision: 2 }).await.unwrap();
    assert_eq!(outcome, CommandOutcome::Accepted { revision: 3 });

    let event = events.recv().await.expect("game.finished broadcast");
    assert_eq!(event.target, None, "finishes are broadcast");
    match event.kind {
        RoomEventKind::Finished { revision, result, snapshot } => {
            assert_eq!(revision, 3);
            assert_eq!(result.winner, None);
            assert_eq!(result.reason, DomainReason::DrawAgreed);
            assert_eq!(snapshot.game_status, rechess_backend::protocol::GameStatus::Draw);
        }
        other => panic!("expected Finished, got {other:?}"),
    }
}

#[tokio::test]
async fn resign_finishes_with_the_opponent_as_winner() {
    let (registry, engine) = scripted_registry();
    let (host, _guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    let mut finished = starting_snapshot();
    finished.status = DomainStatus::Resigned;
    finished.result = Some(DomainResult { winner: Some(DomainColor::Black),
                                         reason: DomainReason::Resignation });
    engine.lock().unwrap().snapshot = finished;

    let outcome = handle.send(RoomCommand::Resign {
        player: host.player_id.clone(), expected_revision: 1 }).await.unwrap();
    assert_eq!(outcome, CommandOutcome::Accepted { revision: 2 });

    let event = events.recv().await.expect("game.finished broadcast");
    match event.kind {
        RoomEventKind::Finished { result, .. } => {
            assert_eq!(result.winner, Some(DomainColor::Black), "opponent of the resigner");
            assert_eq!(result.reason, DomainReason::Resignation);
        }
        other => panic!("expected Finished, got {other:?}"),
    }
}

// --- 4. rematch --------------------------------------------------------------

#[tokio::test]
async fn rematch_needs_both_votes_and_resets_the_engine() {
    let (registry, engine) = scripted_registry();
    let (host, guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    // Finish quickly: resign.
    let mut finished = starting_snapshot();
    finished.status = DomainStatus::Resigned;
    finished.result = Some(DomainResult { winner: Some(DomainColor::Black),
                                         reason: DomainReason::Resignation });
    engine.lock().unwrap().snapshot = finished;
    handle.send(RoomCommand::Resign {
        player: host.player_id.clone(), expected_revision: 1 }).await.unwrap();
    let _finish = events.recv().await.expect("game.finished");
    assert_eq!(engine.lock().unwrap().reset_count, 0);

    // First vote: accepted, NO revision bump, NO broadcast (nothing observable changed).
    let outcome = handle.send(RoomCommand::Rematch {
        player: host.player_id.clone(), expected_revision: 2 }).await.unwrap();
    assert_eq!(outcome, CommandOutcome::Accepted { revision: 2 },
        "first rematch vote does not bump");
    assert!(events.try_recv().is_err(), "no snapshot after a single vote");
    assert_eq!(engine.lock().unwrap().reset_count, 0, "engine not reset yet");

    // Second vote: reset + new game at the NEXT revision (revision never resets).
    engine.lock().unwrap().snapshot = starting_snapshot(); // what a fresh game looks like
    let outcome = handle.send(RoomCommand::Rematch {
        player: guest.player_id.clone(), expected_revision: 2 }).await.unwrap();
    assert_eq!(outcome, CommandOutcome::Accepted { revision: 3 });

    let event = events.recv().await.expect("new game snapshot");
    match event.kind {
        RoomEventKind::Snapshot { revision, snapshot } => {
            assert_eq!(revision, 3, "per-room counter continues across rematches");
            assert_eq!(snapshot.status, RoomStatus::Active, "back to active");
            assert_eq!(snapshot.game_status, rechess_backend::protocol::GameStatus::InProgress);
            assert!(snapshot.move_history.is_empty(), "history cleared");
        }
        other => panic!("expected Snapshot, got {other:?}"),
    }
    assert_eq!(engine.lock().unwrap().reset_count, 1, "reset_for_rematch was called once");
}

// --- 5. presence, grace, closure ---------------------------------------------

#[tokio::test(start_paused = true)]
async fn disconnect_broadcasts_presence_and_grace_expires_to_a_forfeit() {
    let (registry, engine) = scripted_registry();
    let (host, _guest, handle, _code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    // Script the forfeit result BEFORE the deadline fires: grace expiry calls
    // engine.resign(White) — the fake returns its scripted snapshot.
    let mut forfeit = starting_snapshot();
    forfeit.status = DomainStatus::Resigned;
    forfeit.result = Some(DomainResult { winner: Some(DomainColor::Black),
                                       reason: DomainReason::Resignation });
    engine.lock().unwrap().snapshot = forfeit;

    handle.send(RoomCommand::Disconnected { player: host.player_id.clone() })
        .await
        .expect("disconnect acknowledged");

    let event = events.recv().await.expect("presence broadcast");
    match event.kind {
        RoomEventKind::Presence { player_id, connected } => {
            assert_eq!(player_id, host.player_id);
            assert!(!connected);
        }
        other => panic!("expected Presence, got {other:?}"),
    }

    // 60 s default grace — with paused time this is instant and deterministic.
    tokio::time::advance(Duration::from_secs(61)).await;
    let event = events.recv().await.expect("grace expiry forfeits the game");
    match event.kind {
        RoomEventKind::Finished { result, .. } => {
            assert_eq!(result.winner, Some(DomainColor::Black), "disconnected host lost");
            assert_eq!(result.reason, DomainReason::Resignation);
        }
        other => panic!("expected Finished, got {other:?}"),
    }
}

#[tokio::test]
async fn leave_closes_the_room_and_the_actor() {
    let (registry, _engine) = scripted_registry();
    let (host, _guest, handle, code) = open_two_player_room(&registry).await;
    let mut events = handle.subscribe();

    let outcome = handle.send(RoomCommand::Leave { player: host.player_id.clone() })
        .await
        .expect("leave acknowledged");
    assert!(matches!(outcome, CommandOutcome::Accepted { .. }));

    // The actor has ended: send -> RoomClosed, lookup -> gone, events -> closed.
    let closed = handle
        .send(RoomCommand::Leave { player: host.player_id.clone() })
        .await;
    assert!(matches!(closed, Err(rechess_backend::rooms::RoomClosed)));
    assert!(registry.lookup(&code).await.is_none(), "registry entry removed");
    assert!(events.recv().await.is_none(), "subscriber channel closed");
}
```

### 3.1 What each test proves (map back to the brief)

| Test | Brief item |
|---|---|
| `create_seats_host_white_in_a_waiting_room` | Create → unique code, host in White seat |
| `join_seats_guest_black_and_activates_the_room` | Join valid waiting room |
| `join_unknown_code_is_not_found` | Reject invalid code → `ROOM_NOT_FOUND` |
| `second_join_into_a_full_room_is_rejected` | Full/active room; a second player cannot overwrite a seat |
| `resume_with_the_right_token_returns_the_full_snapshot` | Reconnect returns a full current snapshot |
| `resume_with_a_wrong_token_is_invalid_token` / `…unknown_code…` | Wrong-room/wrong-token cannot resume |
| `lookup_hits_live_rooms_and_misses_others` | Routing lookup |
| `accepted_move_bumps_revision_and_broadcasts_the_snapshot` | Move accepted → revision + snapshot broadcast |
| `stale_expected_revision_is_rejected_without_state_change` | Stale revision rejected safely |
| `engine_rejection_targets_only_the_mover` | Illegal move → targeted `MoveRejected` only |
| `a_player_not_in_the_room_cannot_act` | Wrong-room/wrong-seat token cannot move |
| `draw_offer_then_accept_finishes_with_draw_agreed` | Agreed draw → `game.finished` broadcast |
| `resign_finishes_with_the_opponent_as_winner` | Resignation with winner |
| `rematch_needs_both_votes_and_resets_the_engine` | Rematch requires both votes, resets consistently |
| `disconnect_broadcasts_presence_and_grace_expires_to_a_forfeit` | Disconnect starts deadline; expiry resolves the room |
| `leave_closes_the_room_and_the_actor` | Leave → closure; expiry/leave leaks nothing (`RoomClosed`, registry entry gone, channels closed) |

Not in this file but expected by the brief — B4's integration suite adds them
against your registry: two simultaneous joins for the last seat (exactly one
wins — your actor serializes, so it passes by construction), the actor
serializing many concurrent moves (revision strictly increments), room
isolation (activity in room A never affects room B), waiting-room and
finished-room expiry with no task/channel leaks, and queue-saturation
behaviour. Discuss fixtures for those with B4 early
([05-acceptance.md](05-acceptance.md)).

## 4. Running and expected state

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

cargo test                  # full suite: the 15 existing tests + all of yours
cargo test rooms::          # your in-module unit tests
cargo test --test rooms_actor   # your integration tests
cargo fmt --check           # PR gate (team operating rules)
cargo clippy -- -D warnings # PR gate — keep await_holding_lock enabled
```

Today: full `cargo test` = **15 green**, `rooms::` = 0, `--test rooms_actor`
fails (no such file). At merge: everything green, including all 15 existing
tests — if your work breaks a fake-based test, your implementation deviates
from the contract; fix your side, never the fakes.

## 5. Two rules while porting these tests to real types

1. **Never `pub` something only a test needs.** These tests use only the
   frozen public traits plus your registry constructor. If a test needs
   `pub(crate)`, it belongs in the in-module `#[cfg(test)]` section instead.
2. **No sleeps to "wait" for the actor.** `send().await` returning IS the
   proof the actor processed the command (the reply is sent from inside the
   handler). If you find yourself adding `tokio::time::sleep(…)` to make a
   test pass, you have a race — fix the design, not the test.
