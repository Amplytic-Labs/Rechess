# 02 — Integration suite: end-to-end scenarios

> **B4 doc set, part 2.** The system-level scenarios you automate as modules
> land, plus the manual smoke test for the real stack. You own this suite;
> owners supply the per-module fixtures (B1: chess edge cases, B2: lifecycle
> knobs, B3: wire-level cases). Owner: B4.

---

## 1. What this suite proves (and when)

The suite verifies the system **as one piece**: B3's socket layer → B2's
room actors → B1's engine, over the frozen protocol. It is built
incrementally: every scenario below is runnable **today** against the
contract testkits (scripted outcomes), and is re-pointed at the real
implementations as they land — the harness pattern in §2 makes that a
one-line swap per suite, not a rewrite.

Scenario coverage required by your brief (all ten are specced in §3):

| # | Scenario | Unlocked for full automation when |
|---|---|---|
| 1 | Full happy path → checkmate → `game.finished` | B1 + B2 (real moves), wire-checked when B3 lands |
| 2 | Stale revision conflict + reconciliation | B2 (real revision bookkeeping) |
| 3 | Illegal move rejection, delivered to the mover only | B1 (real rules) + B2 |
| 4 | Draw offer / accept | B1 + B2 |
| 5 | Resignation | B1 + B2 |
| 6 | Rematch | B2 (both-must-agree reset) |
| 7 | Reconnect via resume token | B2 (grace + tokens) |
| 8 | Opponent presence on disconnect + grace | B2 (grace deadline) + B3 (socket close notification) |
| 9 | Room expiry | B2 (expiry timers) |
| 10 | Concurrent joins, first-wins | B2 (registry race) |

Room-isolation (moves in room A never change room B) and the 50-room load
workload are separate suites — see §5 and [05-performance.md](05-performance.md).

## 2. The harness pattern (copy from `tests/testkit_demo.rs`)

The pattern that makes this possible today is in
`tests/testkit_demo.rs::generic_code_works_against_any_registry`: **write
every scenario generically over the registry type**, so the same test code
runs unchanged against the fakes (today) and B2's real registry (later):

```rust
use rechess_backend::game::{PlayerColor, ChessEngine as _};
use rechess_backend::rooms::{
    CommandOutcome, RoomCommand, RoomEvent, RoomEventKind,
    RoomHandle as _, RoomRegistry,
};

// The single switch point of the whole suite. TODAY: FakeRegistry.
// When B2 lands: `RealRegistry::new(...)` — one line, every scenario re-runs.
fn registry_under_test() -> impl RoomRegistry {
    // today (fake: exactly one room "TESTCODE1", host/guest tokens fixed):
    // rechess_backend::rooms::testkit::FakeRegistry::new("TESTCODE1")
    rechess_backend::rooms::testkit::FakeRegistry::new("TESTCODE1")
}
```

Rules for the harness (from your brief: "make test fixtures deterministic"):

- **No real time.** Lifecycle deadlines (60 s reconnect grace, 10 min waiting
  expiry, 10 min finished retention) are B2's constants — ask B2 for
  test-time knobs (e.g. an injectable clock or a `now()` fn) **before** you
  write scenario 9; use `tokio::time::pause()`/`advance()` where the room
  actor supports it; a real 60-second sleep in CI is a defect in the test.
- **Scripted engine outcomes** today: `FakeEngine::new(snapshot)` +
  `fail_next_with(GameError::…)` and `FakeHandle`'s
  `push_outcome(CommandOutcome::Rejected { error, message, revision })`;
  when B1 lands, replace scripted outcomes with real move sequences (the
  fool's-mate fixture `game::testkit::foolsmate_snapshot()` already encodes
  the finished position).
- **Socket-level harness** (when B3 lands): one shared `tests/common/mod.rs`
  that binds an ephemeral port and hands out connected raw WS clients
  (`tokio-tungstenite`), plus `tower::ServiceExt::oneshot()` on
  `rechess_backend::build_router()` for HTTP-level checks (`/health`):

  ```rust
  // tests/common/mod.rs — sketch (B4 creates this file when B3's /ws lands)
  use rechess_backend::build_router;
  use tokio_tungstenite::tungstenite::Message;

  pub async fn spawn_server() -> String {                 // returns ws://127.0.0.1:<port>/ws
      let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
      let addr = listener.local_addr().unwrap();
      let app = build_router();                           // the WS route is B3's to mount
      tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
      format!("ws://{addr}/ws")
  }

  pub async fn ws_client(url: &str) -> tokio_tungstenite::WebSocketStream<…> { /* connect + return */ }
  pub async fn send(ws: &mut …, v: &serde_json::Value) { /* send Message::Text(v.to_string()) */ }
  pub async fn next_event(ws: &mut …) -> serde_json::Value { /* recv, parse one frame as JSON */ }
  ```

- Every raw JSON fixture used at the socket level is the **exact text from
  `docs/protocol-v1.md`** (already proven round-tripping by
  `tests/protocol_contract.rs::doc_section_2_1_example_round_trips`) — do not
  hand-write a second, divergent copy.

## 3. The ten scenarios

Notation: `rev` = the room's current revision. Protocol rule (§5 of the spec):
revision starts at 0 and +1 for every **accepted** state-changing command; a
rejected command returns the **current** revision; `game.state`-class
broadcasts carry the new revision at the envelope level. With the fake
handle, outcomes you did not script default to `Accepted { revision: 0 }` —
script explicit revisions to assert the protocol rule. B2's exact
create/join bumping is defined when `rooms::` lands; assert the *rule*
(revision-after-accept = expected + 1), not magic numbers.

### 3.1 Full happy path: create → join → moves to checkmate → `game.finished`

- **Preconditions**: harness registry (§2); seat fixtures: host = White,
  guest = Black. Game script: fool's mate — `f2f3`, `e7e5`, `g2g4`, `Qh4#`
  (`d8h4`); final state must equal `game::testkit::foolsmate_snapshot()`
  (FEN `rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3`,
  Black wins by checkmate).
- **Driving sketch** (today: scripted outcomes; after B1+B2: real engine —
  same code, remove `push_outcome`):

  ```rust
  let registry = registry_under_test();
  let created = registry.create_room("Host").await.expect("create ok");      // White + token
  assert_eq!(created.credentials.color, PlayerColor::White);
  let joined = registry.join_room(&created.room_code, "Guest").await.expect("join ok"); // Black
  let handle = created.handle;
  let mut events = handle.subscribe();                                      // both seats subscribe

  for (player, mv, rev) in [
      ("host", "f2f3", 0), ("guest", "e7e5", 1), ("host", "g2g4", 2), ("guest", "d8h4", 3),
  ] {
      let outcome = handle.send(RoomCommand::Move {
          player: player.into(), from: mv[..2].into(), to: mv[2..].into(),
          promotion: None, expected_revision: rev,
      }).await.expect("send ok");
      assert!(matches!(outcome, CommandOutcome::Accepted { revision } if revision == rev + 1));
  }
  let event = events.recv().await.expect("event");
  assert!(matches!(event.kind, RoomEventKind::Finished { .. }));            // see expected events
  ```

- **Expected events with revisions**: each accepted move →
  `CommandOutcome::Accepted { revision: rev + 1 }` for the mover + broadcast
  `RoomEventKind::Snapshot { revision: rev + 1, snapshot }` (B3 wire form:
  `game.state`, envelope `revision` = rev + 1, payload flattened snapshot).
  After `d8h4`: broadcast `RoomEventKind::Finished { revision: 4, result:
  GameResult { winner: Some(Black), reason: Checkmate }, snapshot }` → wire
  `game.finished` with `{"result":{"winner":"black","reason":"checkmate"}}`,
  and further `Move` commands must be rejected with
  `ErrorCode::GameFinished` (until a rematch resets, §3.6).

### 3.2 Stale revision conflict + reconciliation

- **Preconditions**: room at revision `r` (e.g. 3 after §3.1's first three
  moves); mover has an older view.
- **Driving sketch**:

  ```rust
  handle.push_outcome(CommandOutcome::Rejected {
      error: ErrorCode::StaleRevision, message: "stale revision".into(), revision: 3,
  });
  let outcome = handle.send(RoomCommand::Move {
      player: "host".into(), from: "a2".into(), to: "a3".into(),
      promotion: None, expected_revision: 1,               // stale — room is at 3
  }).await.unwrap();
  assert!(matches!(outcome, CommandOutcome::Rejected { error: ErrorCode::StaleRevision, revision: 3, .. }));
  // Reconciliation: the rejection carries the current revision; the client
  // falls back to the newest broadcast snapshot:
  handle.emit(RoomEvent::broadcast(RoomEventKind::Snapshot { revision: 3, snapshot: sample_protocol_snapshot() }));
  // …client retries the move with expected_revision: 3 → Accepted { revision: 4 }
  ```

- **Expected events**: rejected outcome `Rejected { error: StaleRevision,
  revision: 3 (current), … }`; the wire form is `game.move_rejected`
  **targeted at the mover** with `"error": "STALE_REVISION"`, envelope
  `revision` = 3, payload `snapshot` optional; reconciliation via the latest
  `game.state` broadcast. No board change for the stale attempt; a retry with
  `expected_revision: 3` accepts and returns revision 4.

### 3.3 Illegal move rejection, delivered to the mover only

- **Preconditions**: active game, White to move; engine scripted (today) /
  real (B1) to reject `e2e5` as illegal.
- **Driving sketch**:

  ```rust
  engine.fail_next_with(GameError::IllegalMove);            // today; B1: just play a real illegal move
  handle.emit(RoomEvent::to("host".into(), RoomEventKind::MoveRejected {
      player_id: "host".into(), error: ErrorCode::IllegalMove,
      message: "not a legal move".into(), revision: 3, snapshot: None,
  }));
  let mut opponent = handle.subscribe();                    // guest's receiver
  ```

- **Expected events**: the mover gets `RoomEventKind::MoveRejected` with
  `target: Some("host")` (wire: `game.move_rejected`, targeted — **never
  broadcast**, asserted today by
  `tests/testkit_demo.rs::fake_handle_records_commands_and_emits_events`);
  the opponent's receiver sees **nothing** (no event for that command);
  revision unchanged (the rejection carries the current revision, no bump).
  Out-of-turn moves reject identically with
  `GameError::WrongTurn` → `ErrorCode::NotYourTurn` to the mover only.

### 3.4 Draw offer / accept

- **Preconditions**: active game, both seats connected; engine's
  `offer_draw(by)` / `accept_draw(by)` contract (a second-side offer ACCEPTS
  a pending offer — see `game/contract.rs`).
- **Driving sketch**:

  ```rust
  // White offers:
  let out = handle.send(RoomCommand::OfferDraw { player: "host".into(), expected_revision: 5 }).await.unwrap();
  // Black accepts (its own offer would also auto-accept White's pending one):
  let out = handle.send(RoomCommand::AcceptDraw { player: "guest".into(), expected_revision: 6 }).await.unwrap();
  ```

- **Expected events**: offer → `Accepted { revision: r+1 }` for the offerer +
  broadcast `Snapshot` whose `draw_offered_by == Some(White)` (wire
  `"draw_offer": "white"`); accept → engine returns a finished snapshot
  (`GameStatus::Draw`, `result: Some(GameResult { winner: None, reason:
  DrawAgreed })`), broadcast `Finished { revision: r+2, result, snapshot }`
  → wire `game.finished` `{"result":{"winner":null,"reason":"draw_agreed"}}`.

### 3.5 Resignation

- **Preconditions**: active game; engine `resign(by)` contract (opponent wins
  by `Resignation`).
- **Driving sketch**:

  ```rust
  let out = handle.send(RoomCommand::Resign { player: "host".into(), expected_revision: 8 }).await.unwrap();
  assert!(matches!(out, CommandOutcome::Accepted { .. }));
  ```

- **Expected events**: `Accepted { revision: 9 }` to the resigner; broadcast
  `Finished { revision: 9, result: GameResult { winner: Some(Black), reason:
  Resignation }, snapshot }` → wire `game.finished`
  `{"result":{"winner":"black","reason":"resignation"}}`; subsequent moves
  reject with `GameFinished`.

### 3.6 Rematch

- **Preconditions**: finished game (§3.1 or §3.5); both seats still in the
  room (finished-room retention, 10 min default).
- **Driving sketch**:

  ```rust
  let out1 = handle.send(RoomCommand::Rematch { player: "host".into(),  expected_revision: 9 }).await.unwrap();
  let out2 = handle.send(RoomCommand::Rematch { player: "guest".into(), expected_revision: 10 }).await.unwrap();
  assert_eq!(engine.reset_count, 1);        // fake; real B1 engine: fresh starting snapshot
  ```

- **Expected events**: first vote → `Accepted` + broadcast `Snapshot` (room
  still finished / waiting-for-opponent — B2's semantics); second vote →
  engine `reset_for_rematch()` called exactly once, broadcast `Snapshot
  { revision: r+2, snapshot: starting position }` (FEN
  `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`,
  `draw_offered_by: None`, empty history — the reset must clear board,
  history and draw state; both-must-agree is asserted by exactly this
  ordering).

### 3.7 Reconnect via resume token

- **Preconditions**: a live seat with its `player_token` (from
  `create_room`/`join_room` credentials); the token is the only credential —
  room code alone must not resume.
- **Driving sketch** (today's fake: host token is fixed `"host-token"`, any
  other token → `InvalidToken`):

  ```rust
  let resumed = registry.resume_room(&created.room_code, &created.credentials.player_token)
      .await.expect("resume ok");
  assert_eq!(resumed.color, PlayerColor::White);
  assert_eq!(resumed.snapshot.room_code, created.room_code);  // full snapshot at the current state
  assert!(matches!(registry.resume_room(&created.room_code, "bad-token").await,
                   Err(ResumeError::InvalidToken)));
  ```

- **Expected events**: valid token → `Resumed { player_id, color, handle,
  snapshot }` and B3 emits **targeted** `session.ready` with the full
  snapshot; the client replaces its entire local state. Invalid token →
  `ResumeError::InvalidToken` → wire `server.error`
  `{"error":"INVALID_TOKEN"}` and the client returns to the home screen.
  `player_token` never appears in any broadcast (asserted today by
  `tests/protocol_contract.rs::only_room_created_joined_carry_player_token`).

### 3.8 Opponent presence on disconnect + grace

- **Preconditions**: both seats connected; B3 notifies B2 of the socket drop;
  B2's reconnect-grace deadline (default 60 s) starts; a valid resume (§3.7)
  cancels it.
- **Driving sketch**:

  ```rust
  handle.send(RoomCommand::Disconnected { player: "guest".into() }).await.unwrap();
  // broadcast presence to the opponent; after the grace deadline expires,
  // B2 resolves the room (forfeit / close per its lifecycle rules):
  handle.emit(RoomEvent::broadcast(RoomEventKind::Presence { player_id: "guest".into(), connected: false }));
  // later, before the deadline:
  let resumed = registry.resume_room(&code, &guest_token).await;    // Ok → grace cancelled
  ```

- **Expected events**: on drop, broadcast `Presence { player_id: guest,
  connected: false }` (wire `player.presence`), **no** snapshot change, no
  revision bump (presence is not a state-changing command — B2 confirms at
  freeze); on resume, broadcast `Presence { connected: true }` + targeted
  `session.ready`. If grace expires: room resolves per B2's rules and the
  actor ends — later `send` returns `Err(RoomClosed)` (§3.9).

### 3.9 Room expiry

- **Preconditions**: a room past its lifecycle deadline — waiting-expiry
  (10 min default) or grace expiry after a disconnect. Use B2's test-time
  knobs, never a real sleep.
- **Driving sketch**:

  ```rust
  handle.close();                                              // fake: room actor ended
  assert!(matches!(handle.send(RoomCommand::Leave { player: "host".into() }).await,
                   Err(RoomClosed)));
  // and at the registry level:
  assert!(matches!(registry.join_room("EXPIREDC", "Guest").await, Err(JoinRoomError::Expired)));
  assert!(registry.lookup("EXPIREDC").await.is_none());        // cleaned up, not lingering
  ```

- **Expected events**: the actor is gone: in-room commands → `Err(RoomClosed)`
  (wire: `server.error` `ROOM_EXPIRED` or a documented close code);
  `join_room` on the expired code → `JoinRoomError::Expired` → wire
  `ROOM_EXPIRED`; `lookup` returns `None` (actor and entry cleaned up, no
  leak). Empty-room deletion is prompt, not after another timeout.

### 3.10 Concurrent joins, first-wins

- **Preconditions**: one waiting room with one free seat; two joiners racing
  for it.
- **Driving sketch**:

  ```rust
  let (a, b) = tokio::join!(
      registry.join_room(&code, "Fast"),
      registry.join_room(&code, "Slow"),
  );
  let results = [a, b];
  assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);          // exactly one seat
  assert_eq!(results.iter().filter(|r| matches!(r, Err(JoinRoomError::Full))).count(), 1);
  ```

- **Expected events**: exactly one `Ok(RoomJoined)` (guest seat, Black, own
  token) and exactly one `Err(JoinRoomError::Full)` → wire `ROOM_FULL`
  (`server.error` to the loser only). The winner's `room.joined` carries
  `RoomPublicState` with both players seated. No double-occupancy: the
  seat count in every subsequent snapshot is exactly 2.

## 4. Manual end-to-end smoke test (the real stack)

Run after every merge that touches `src/network/`, before release tags, and
before every demo. Two browser tabs or windows (the two "devices"), plus one
raw WS client (websocat or Postman) for protocol-level checks.

**Setup**

```bash
# Terminal 1 — the server (debug build is fine for a smoke test):
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo run                        # binds 0.0.0.0:3000 — log prints the address

# Terminal 2 — health check (this works TODAY; everything below the line
# marked [after B3 lands] 404s until the /ws route exists):
curl -s http://localhost:3000/health     # expect {"service":"rechess-backend","status":"ok"}

# Terminal 3 — the web client for the two "devices" (each a browser tab):
cd frontend && npm run dev               # → http://localhost:3001 — open it in
                                         # two tabs (or two browsers / one
                                         # normal + one incognito window)
# (default NEXT_PUBLIC_API_URL=ws://localhost:3000/ws already targets the
#  local backend — see frontend/RULES.md §5)
```

**Raw-protocol checks** (websocat; Postman: New → WebSocket Request → same URL):

```bash
# [after B3 lands] connect a raw client to the WS endpoint:
websocat ws://127.0.0.1:3000/ws
# then paste, one line per message:
{"v":1,"id":"cmd-001","type":"room.create","payload":{"nickname":"SmokeHost"}}
# expect a targeted room.created with room_code (8 chars), player_id,
# player_token, "color":"white" and room.status "waiting"
# malformed input must not kill the connection:
{"v":1,
# expect a structured server.error (INVALID_PAYLOAD), socket stays open
```

**Manual checklist** — perform on the two devices, expected result each step:

| # | Step | Expected |
|---|---|---|
| 1 | Device 1 creates a room with nickname | Waiting room shown, room code displayed, White assigned |
| 2 | Device 2 joins with that code | Both seats fill; both clients show the starting position |
| 3 | Device 1 plays `e2–e4` | Both boards update from the same revision; last-move highlight on both |
| 4 | Device 2 tries an illegal move (e.g. `e7–e5` while it is White's turn is rejected as out-of-turn; a rook-through-pieces as illegal) | Only the mover sees the rejection UI; the board does not change |
| 5 | Play the fool's mate sequence `f3, e5, g4, Qh4#` | Both clients show checkmate and the result screen (`game.finished`) |
| 6 | One player offers a draw, the other accepts | Both show draw-agreed result (use a fresh game) |
| 7 | One player resigns | Opponent wins by resignation on both clients |
| 8 | Both vote rematch | A fresh game starts on both, history cleared |
| 9 | Airplane-mode device 2 for ~5 s, back online | Opponent sees "disconnected" (presence), device 2 auto-resumes via token and gets the full current snapshot (`session.ready`) |
| 10 | Airplane-mode device 2 past the grace window | Opponent's client resolves per lifecycle rules; device 2 gets `INVALID_TOKEN`-style message on return |
| 11 | Kill and restart the server (`Ctrl-C`, `cargo run`) | Rooms are GONE — clients land on the home screen with a reconnect failure. This is the documented in-memory limitation, verify it is messaged, not crashed |
| 12 | Watch the server logs during 1–11 | Structured logs with room code and command ID; **no `player_token` anywhere** |

Record the run (date, commit, devices, what passed) in the PR description of
the merge that was smoke-tested — this checklist is also the release
acceptance input ([04-deployment.md](04-deployment.md) §8).

## 5. Beyond the ten: isolation and load suites

- **Room isolation** (`room_isolation.rs` when you add it): two rooms driven
  in parallel via the harness; assert every snapshot's `room_code` matches its
  own room, and no event from room A reaches room B's subscribers.
- **Load / benchmark**: [05-performance.md](05-performance.md) (50 rooms /
  100 players) — its harness shares `tests/common/mod.rs`.
- **Slow client / backpressure**: when B2 ships bounded queues, add a client
  that never reads; assert the queue-full policy (drop + disconnect/resync)
  rather than unbounded growth.
- **Duplicate command retry**: same `id` re-sent after a reconnect; assert
  at-most-once application (protocol §5 dedup window).
