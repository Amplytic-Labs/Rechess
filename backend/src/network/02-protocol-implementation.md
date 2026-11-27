# 02 — Protocol implementation: per-command flows

> Prereqs: `01-axum-websocket-guide.md` (the connection task skeleton — this
> doc fills in its `dispatcher::on_text` and `map_room_event`), and
> `docs/protocol-v1.md` §2–§6. Every type below is copied exactly from
> `src/protocol/` and `src/rooms/contract.rs` — do not paraphrase field
> names; a renamed field breaks the other side at runtime.

---

## 1. The command pipeline

Every inbound text frame goes through the same sequence:

```
Text frame
  └─ 1 parse gate        serde_json → Value → check v → ClientEnvelope
  └─ 2 version gate      v != PROTOCOL_VERSION → UNSUPPORTED_VERSION + close
  └─ 3 payload gate      nickname / room-code shape / frame rules / rate
  └─ 4 auth gate         UNBOUND? BOUND? allowed command in this state?
  └─ 5 dispatch          registry op (create/join/resume) or RoomCommand
  └─ 6 outcome           Accepted → silence; Rejected → error event
  └─ (async) events      RoomEvent stream → ServerEvent → outbound queue
```

Steps 1–4 are pure validation you own; step 5–6 call B2; the async arm is
§7.3. Steps 2–4 can be one `match` — keep them as separate functions so each
is unit-testable (doc 03).

## 2. The parse gate: text → `ClientEnvelope`

### 2.1 Why not `serde_json::from_str::<ClientEnvelope>` directly

A direct parse cannot distinguish "wrong protocol version" from "garbage":
with an unknown `type` or a v2 command shape, serde fails the whole envelope
and you would answer `INVALID_PAYLOAD` to a client whose only sin is being
v2. `docs/protocol-v1.md` §8.4 requires the *structured* error
`UNSUPPORTED_VERSION` for unknown versions. So parse in three steps:

```rust
// src/network/dispatcher.rs (skeleton)
use rechess_backend::protocol::{ClientEnvelope, PROTOCOL_VERSION};
use serde_json::Value;

pub enum Rejection { UnsupportedVersion, InvalidPayload }

pub fn parse_command(text: &str) -> Result<ClientEnvelope, Rejection> {
    // Step 1: it must be JSON at all.
    let value: Value =
        serde_json::from_str(text).map_err(|_| Rejection::InvalidPayload)?;
    // Step 2: `v` must be a number, and it must be ours — checked BEFORE
    // the command shape so a v2 client gets the right error even if its
    // v2 commands are also unknown to us.
    match value.get("v") {
        Some(Value::Number(n)) if n.as_u64() == Some(PROTOCOL_VERSION as u64) => {}
        Some(Value::Number(_)) => return Err(Rejection::UnsupportedVersion),
        _ => return Err(Rejection::InvalidPayload),
    }
    // Step 3: the command itself must deserialize (missing fields, wrong
    // types, unknown `type` ⇒ InvalidPayload).
    serde_json::from_value(value).map_err(|_| Rejection::InvalidPayload)
}
```

This exact gate is proven in doc 03's `tests/network_protocol.rs` (compiles
and passes green today — copy it in as your first commit).

### 2.2 Error handling: `INVALID_PAYLOAD`, never a panic, budget of 3

- **Every** parse failure answers one event, on the same connection:

  ```json
  { "v": 1, "type": "server.error", "payload": { "error": "INVALID_PAYLOAD",
      "message": "malformed command" } }
  ```

  Build it as `ServerEnvelope { v: PROTOCOL_VERSION, revision: None, event:
  ServerEvent::ServerError { error: ErrorCode::InvalidPayload, message: … } }`
  — never log the raw input at `info` level (it can contain anything; log a
  truncation at `debug` with the connection id).
- **Never panic**: `serde_json` returns `Result`s everywhere; a panic inside
  `on_upgrade`'s task kills only that connection, but it also skips the
  teardown that notifies B2 (doc 01 §4) — a seat would leak until its grace
  window expires.
- **Do not disconnect on the first bad frame.** Budget: **3 malformed frames
  per connection, then close.** Justification: a healthy, typed client sends
  *zero* bad frames; a single one can be a version-skew race (the client
  upgraded mid-game), a proxy glitch, or one lost race at app backgrounding —
  killing a connection mid-game costs a full reconnect cycle and eats B2's
  grace window. Three within one connection cannot be a race: the client is
  broken or hostile, so the socket goes. Count binary frames (§4.3) in the
  same budget. On budget exhaustion: send the final `server.error
  INVALID_PAYLOAD`, then `Message::Close(Some(CloseFrame { code: 1008, … }))`.

## 3. The version gate

`docs/protocol-v1.md` §8.4: *"Unknown protocol versions get a structured
`UNSUPPORTED_VERSION` error, not a dropped connection."* Translation for your
code: the error event MUST be sent before anything else happens to the
connection — a client that never learns why it was dropped cannot show a
"please update" screen. After the structured error, **close** (code 1008, policy
violation): a v2 client cannot speak v1; keeping the socket open only holds
resources. Sequence:

1. `parse_command` returns `Err(Rejection::UnsupportedVersion)`.
2. Send `ServerEvent::ServerError { error: ErrorCode::UnsupportedVersion,
   message: "protocol v1 supported only" }` (envelope `revision: None`).
3. Send Close(1008), end the connection task. Do NOT count this toward the
   malformed budget — it is a distinct, structured exit.

## 4. The payload gate (before B2, per docs/protocol-v1.md §3)

### 4.1 Nickname (`room.create`, `room.join`)

1. `let nickname = nickname.trim();` — trim FIRST.
2. Reject when empty, longer than **20 chars** (`chars().count()`, not bytes —
   Unicode nicknames are fine), or containing any control character
   (`nickname.chars().any(char::is_control)`).
3. On violation: `server.error INVALID_PAYLOAD`, keep the connection.
4. Pass the **trimmed** nickname to B2 (`create_room(&nickname)` /
   `join_room(&code, &nickname)` take `&str`).

### 4.2 Room code (`room.join`, `session.resume`)

1. Normalize: **trim + uppercase** — `contract.rs` documents
   `join_room`'s argument as *"already normalized (uppercased/trimmed) by
   B3"*. This is B3's job; the comment in `client_message.rs` saying
   "(B2 normalizes)" is stale — the frozen contract wins.
2. Shape-check: exactly **8 chars from `ABCDEFGHJKMNPQRSTUVWXYZ23456789`**
   (the unambiguous alphabet). Wrong shape → `INVALID_PAYLOAD`
   ("room code must be 8 letters/digits").
3. Existence / lifecycle is **B2's** domain: a well-shaped but unknown code
   reaches `join_room` and comes back as `JoinRoomError::NotFound` →
   `ROOM_NOT_FOUND` (§6). Never answer `ROOM_NOT_FOUND` from B3 — you would
   own room existence you do not have.

### 4.3 Frame types

`Message::Text` only. `Message::Binary` → `INVALID_PAYLOAD` + malformed
budget++ (v1 has no binary payloads). `Ping`/`Pong` are protocol-level
(doc 01 §7). `Close` ends the loop. `Frame(_)` (raw): ignore.

### 4.4 Rate limits (transport level, all commands)

Per-connection token bucket (constants in `limits.rs`):

| Limit | Value | Rationale |
|---|---|---|
| Commands | 10/s, burst 20 | Chess pace is a few commands per *minute*; 10/s is ~100x headroom while still catching runaway retry loops. |
| `room.create` / `room.join` / `session.resume` | 1 per 10 s per connection | Room-code guessing / room-hopping throttle (room codes are identifiers, not secrets). B2 re-guards too; the cheap B3 gate protects B2 first. |

Violation → `server.error` `RATE_LIMITED`, connection stays (first offenses
are recoverable); a client that keeps tripping the bucket after the error (5+
in a minute) is closed like the malformed case. Per-IP limits belong to the
reverse proxy — hand the numbers to B4 for the runbook.

## 5. The auth state machine

### 5.1 States and transitions

| State | Meaning | Allowed commands | Everything else |
|---|---|---|---|
| **UNBOUND** (start) | No room, no seat | `room.create`, `room.join`, `session.resume` | `server.error INVALID_PAYLOAD` ("bind with room.create / room.join / session.resume first") |
| **BOUND** to `(room_code, player_id)` | One seat in one room, proven by create/join/resume | `game.move`, `game.offer_draw`, `game.accept_draw`, `game.resign`, `game.rematch`, `room.leave` | `room.create` / `room.join` / `session.resume` again → `server.error INVALID_PAYLOAD` ("connection already bound to a room") |

A connection binds at most once in its lifetime. Rebinding on the same
socket (after `room.leave` or a finished room) is deliberately **not**
supported in v1 — a new room needs a new connection; see open decision D3.

### 5.2 Identity comes from binding, never from the wire

The wire commands carry **no** `player_id` and (except `session.resume`) no
token: your `ClientCommand` variants have neither field. The player's
identity is established exactly once, by B2, inside
`create_room`/`join_room`/`resume_room` (the token is verified against B2's
stored hash), and comes back to you in
`PlayerCredentials.player_id` / `Resumed.player_id`. From then on:

- You inject that `player_id` into every `RoomCommand` you build (§7.1).
- A room code alone is never authorization: in-room commands go only to the
  connection's **bound** handle; there is no path where a client names a
  different room, because the wire has no room field on in-room commands.
- `RoomRegistry::lookup` exists on the contract, but v1's in-room flow never
  needs it (the bound handle is the routing key). Use it only if you ever
  re-resolve a handle (e.g. after B2 exposes per-lookup semantics).

### 5.3 OPEN DECISION D1 — where the command `id` is consumed (dedup)

**This is a real gap between the frozen contract and the frozen wire spec.
Settle it with B2 on Day 1, in writing.** The facts:

- `ClientEnvelope.id` is documented in `envelope.rs` as *"Unique command id
  (client-generated UUID) — **used for dedup by B2**"*, and
  `docs/protocol-v1.md` §5 says: *"The room actor remembers recent IDs per
  player in a bounded window: a retried `game.move` with an
  already-processed ID is not applied twice (either idempotent-accepted with
  the original outcome or `STALE_REVISION`, agreed by B2+B3)."*
- BUT `rooms/contract.rs`'s `RoomCommand` variants (`Move`, `OfferDraw`, …,
  `Leave`, `Disconnected`) carry **no command id**, and `RoomHandle::send`
  takes only the `RoomCommand`. As frozen, **the id never reaches B2** — the
  actor cannot dedup by id through this contract.
- The Master Plan also assigns dedup to B2 ("Command-ID dedup window" is in
  `src/rooms/README.md` §6, B2's checklist).

**Options:**

- **Option A (recommended): B2 dedups — extend `RoomCommand` to carry the
  id.** One PR agreed by B2 + B3 (the contract's own change rule; doc
  §9 updates if the wire changes — it does not). Concretely: either wrap the
  enum — `struct RoomCommand { pub id: String, pub action: RoomAction }` —
  or add `command_id: String` to the state-changing variants. The actor keeps
  a bounded per-player window of processed ids and returns the remembered
  outcome (idempotent-accept) or `STALE_REVISION` — exactly what doc §5
  promises. Why A: the wire spec (source of truth) assigns the memory to the
  room actor; dedup at the actor is the only variant that survives
  reconnects (a resumed connection is a NEW connection with fresh state);
  one dedup owner means one place to test.
- **Option B (no contract change): B3 dedups per connection.** Keep a
  bounded set of seen ids per connection; drop retries silently. Gap: after
  `session.resume` the set is empty, so a retried id on the new connection
  reaches the actor. The `expected_revision` guard makes double-*application*
  rare (an applied move bumps the revision, so the retry is stale), but the
  "command accepted, response lost, client reconnects and retries with the
  same `expected_revision`" window remains — doc §5 explicitly wants that
  closed, at the actor.

**Recommendation: Option A.** Until it lands, B3 forwards every parsed
command and relies on B2's revision guard — and must NOT quietly build its
own per-connection id cache as a permanent substitute (two dedup layers hide
each other's bugs; if you build it as a stopgap, remove it in the PR that
lands A). Independently of the decision: **always log the id**
(`tracing::debug!(cmd = %envelope.id, …)`) — your brief's logging rule.

**State precisely for the Day-1 meeting with B2:** the id is currently
consumed nowhere. Target state (A): consumed inside `RoomHandle::send`'s
implementation (B2's actor), arriving inside `RoomCommand`.

## 6. Registry operations (UNBOUND → BOUND)

The three binding flows. All replies are **private** — `out_tx` of the
initiating connection only — and all are `ServerEnvelope { v: PROTOCOL_VERSION,
revision: None, … }`.

### 6.1 `room.create`

1. Gate: parse (§2), version (§3), nickname (§4.1), must be UNBOUND (§5.1).
2. `let created = registry.create_room(&nickname).await;` — returns
   `Result<RoomCreated<Self::Handle>, CreateRoomError>`.
3. On `Ok(RoomCreated { room_code, credentials, handle, room })`:
   - **Order matters:** store `handle`, set `player_id =
     credentials.player_id`, set `room_code`, and call
     `let events = handle.subscribe();` **before** replying (doc 01 §4) —
     the reply proves the subscription exists.
   - Reply `ServerEvent::RoomCreated { room_code, player_id:
     credentials.player_id, player_token: credentials.player_token, color:
     to_wire_color(credentials.color), room }` — the **only** place the
     token ever appears on the wire.
4. On `Err(CreateRoomError::Internal(detail))`: `tracing::error!` the detail,
   reply `ServerError { error: ErrorCode::Internal, message: "room creation
   failed" }`. The detail string NEVER reaches the wire.

### 6.2 `room.join` — error mapping (copied from `contract.rs` ↔ docs §6)

Same shape as create, plus normalization: `room_code.trim().to_uppercase()`
+ shape check (§4.2) before `registry.join_room(&code, &nickname).await`.
`Ok(RoomJoined { credentials, handle, room })` → subscribe, bind, reply
`ServerEvent::RoomJoined { player_id, player_token, color, room }` (private).
`Err` mapping — `contract.rs` documents each arm with its wire code:

| `JoinRoomError` (contract) | Wire `ErrorCode` | Safe message |
|---|---|---|
| `NotFound` | `ROOM_NOT_FOUND` | "no room with that code" |
| `Full` | `ROOM_FULL` | "room already has two players" |
| `Expired` | `ROOM_EXPIRED` | "room has expired" |
| `Internal` | `INTERNAL` | "join failed" (detail to logs) |

All four are delivered as `ServerError` events on the same envelope shape —
connection stays open (the client can retry with another code).

### 6.3 `session.resume` — error mapping (copied from `contract.rs` ↔ docs §6)

1. Normalize the code (§4.2). **Do not log the token** (`player_token` is a
   secret; log only `resume attempted`, room code and the connection id).
2. `registry.resume_room(&code, &token).await` →
   `Result<Resumed<Self::Handle>, ResumeError>`.
3. On `Ok(Resumed { player_id, color, handle, snapshot })`: subscribe, bind,
   reply `ServerEvent::SessionReady { player_id, color: to_wire_color(color),
   snapshot }` — the full authoritative snapshot; the client replaces its
   whole local state with it (docs §7.3). `Resumed.snapshot` is already the
   wire type `protocol::GameSnapshot` — pass it through unchanged.
4. On `Err`:

| `ResumeError` (contract) | Wire `ErrorCode` | Safe message |
|---|---|---|
| `NotFound` | `ROOM_NOT_FOUND` | "no room with that code" |
| `InvalidToken` | `INVALID_TOKEN` | "session expired — please start a new game" |
| `Internal` | `INTERNAL` | "resume failed" (detail to logs) |

Per docs §7.4 the client returns to the home screen on `INVALID_TOKEN`.

**Revision note (see D5 in §9):** `session.ready` carries no revision, but
the client's next command needs `expected_revision`. Agreed resolution to
confirm with B2: the actor emits a `RoomEventKind::Snapshot { revision, … }`
to the resumed connection right after the private reply, so the client sees a
`game.state` with the envelope revision and can move.

## 7. In-room dispatch (BOUND)

### 7.1 Command translation table — exact enums

`ClientCommand` (from `src/protocol/client_message.rs`) → `RoomCommand`
(from `src/rooms/contract.rs`). Both enums, verbatim, with the translation:

```rust
// src/rooms/contract.rs — the target enum (verbatim, minus doc comments)
pub enum RoomCommand {
    Move { player: PlayerId, from: String, to: String,
           promotion: Option<Promotion>, expected_revision: u64 },
    OfferDraw { player: PlayerId, expected_revision: u64 },
    AcceptDraw { player: PlayerId, expected_revision: u64 },
    Resign { player: PlayerId, expected_revision: u64 },
    Rematch { player: PlayerId, expected_revision: u64 },
    Leave { player: PlayerId },
    Disconnected { player: PlayerId },   // B3-initiated, never from the wire
}
```

| `ClientCommand` (wire) | Construction you build (BOUND state) |
|---|---|
| `MakeMove { from, to, promotion, expected_revision }` | `RoomCommand::Move { player: player_id.clone(), from, to, promotion: promotion.map(to_domain_promotion), expected_revision }` |
| `OfferDraw { expected_revision }` | `RoomCommand::OfferDraw { player: player_id.clone(), expected_revision }` |
| `AcceptDraw { expected_revision }` | `RoomCommand::AcceptDraw { player: player_id.clone(), expected_revision }` |
| `Resign { expected_revision }` | `RoomCommand::Resign { player: player_id.clone(), expected_revision }` |
| `Rematch { expected_revision }` | `RoomCommand::Rematch { player: player_id.clone(), expected_revision }` |
| `LeaveRoom` | `RoomCommand::Leave { player: player_id.clone() }` |

Note the two conversions the table hides:

- `player` never comes from the wire — it is the bound `player_id`
  (§5.2).
- `promotion` flips wire → domain:
  `protocol::Promotion { Q, R, B, N }` → `game::Promotion { Queen, Rook,
  Bishop, Knight }` (§7.3).

Then: `let outcome = handle.send(command).await;` where
`Ok(CommandOutcome)`, `Err(RoomClosed)` means the room ended mid-flight —
close the connection gracefully (doc 01 §5).

### 7.2 `CommandOutcome` mapping

```rust
// src/rooms/contract.rs — verbatim
pub enum CommandOutcome {
    Accepted { revision: u64 },
    Rejected { error: crate::protocol::ErrorCode, message: String, revision: u64 },
}
```

| Outcome | What B3 sends | Why |
|---|---|---|
| `Accepted { revision }` | **Nothing. Log only.** | The authoritative state arrives as the actor's broadcast `Snapshot` event → `game.state` with the envelope revision (§7.3). If B3 also sent an ack, the mover would receive *two* state messages for one command with no defined order between them, because the outcome future and the event stream are different channels. One source of ordering — the actor's event stream — is the whole point of the design; the client sets a "pending" indicator and waits for `game.state` (docs §5). The outcome's `revision` is for your tracing only. |
| `Rejected { error, message, revision }` — when the command was `game.move` | `ServerEvent::MoveRejected { error, message, snapshot: None }`, envelope `revision: Some(revision)`, **to the mover's connection only** (`out_tx` of the sender) | docs §4.2: a failed move answers `game.move_rejected`, targeted. The client reconciles from the newest snapshot it has (or from the current revision, which the envelope carries). |
| `Rejected { … }` — any other command (`offer_draw`, `accept_draw`, `resign`, `rematch`, `leave`) | `ServerEvent::ServerError { error, message }`, envelope `revision: None`, to the sender only | Non-move rejections have no dedicated event; `server.error` is the generic rejection channel (docs §4.2). |

### 7.3 `RoomEvent` routing — target filtering + converters

Your connection holds one subscription (`handle.subscribe()`) created at
bind time. Every `RoomEvent` you pull from it is mapped like this:

| `RoomEventKind` (contract) | `target` | → `ServerEvent` (wire) | Envelope `revision` |
|---|---|---|---|
| `Snapshot { revision, snapshot }` | `None` → all subscribers | `GameState { snapshot }` — the payload IS the snapshot (`#[serde(flatten)]`: snapshot fields sit directly inside `payload`) | `Some(revision)` |
| `Presence { player_id, connected }` | `None` → broadcast | `PlayerPresence { player_id, connected }` | `None` |
| `Finished { revision, result, snapshot }` | `None` → broadcast | `GameFinished { result: to_wire_result(result), snapshot }` | `Some(revision)` |
| `MoveRejected { player_id, error, message, revision, snapshot }` | `Some(player)` → that player's connection ONLY | `MoveRejected { error, message, snapshot }` | `Some(revision)` |

**The target filter is mandatory.** A room's subscription stream delivers
*every* event to *every* subscriber; your per-connection loop must drop
events whose `target` is `Some(other)` than your bound `player_id`:

```rust
if let Some(target) = &event.target {
    if target != &player_id { continue; }   // not for this connection
}
```

Without it, the opponent would see the mover's `game.move_rejected` — doc
03's harness asserts exactly this non-delivery.

`Finished` needs a domain → wire conversion (the contract carries
`game::GameResult`; the wire wants `protocol::GameResult` — note how
`Snapshot` is already the wire type but `Finished`'s `result` is not; you
bridge):

```rust
use crate::{game, protocol};

pub fn to_domain_promotion(p: protocol::Promotion) -> game::Promotion {
    match p {
        protocol::Promotion::Q => game::Promotion::Queen,
        protocol::Promotion::R => game::Promotion::Rook,
        protocol::Promotion::B => game::Promotion::Bishop,
        protocol::Promotion::N => game::Promotion::Knight,
    }
}

pub fn to_wire_color(c: game::PlayerColor) -> protocol::PlayerColor {
    match c {
        game::PlayerColor::White => protocol::PlayerColor::White,
        game::PlayerColor::Black => protocol::PlayerColor::Black,
    }
}

pub fn to_wire_result(r: game::GameResult) -> protocol::GameResult {
    protocol::GameResult { winner: r.winner.map(to_wire_color), reason: to_wire_reason(r.reason) }
}

fn to_wire_reason(r: game::GameResultReason) -> protocol::GameResultReason {
    match r {
        game::GameResultReason::Checkmate => protocol::GameResultReason::Checkmate,
        game::GameResultReason::Stalemate => protocol::GameResultReason::Stalemate,
        game::GameResultReason::Resignation => protocol::GameResultReason::Resignation,
        game::GameResultReason::DrawAgreed => protocol::GameResultReason::DrawAgreed,
        game::GameResultReason::InsufficientMaterial => protocol::GameResultReason::InsufficientMaterial,
        game::GameResultReason::ThreefoldRepetition => protocol::GameResultReason::ThreefoldRepetition,
        game::GameResultReason::FiftyMoveRule => protocol::GameResultReason::FiftyMoveRule,
    }
}
```

`to_domain_promotion` and `to_wire_color` are used in §6 (credentials) and
§7.1 (moves). These four functions are your first unit tests after the
serialization suite (doc 03 §2.3).

### 7.4 OPEN DECISION D2 — one rejection channel, not two

The contract offers **two** ways a rejection can reach you:

1. synchronously, as `CommandOutcome::Rejected` from `handle.send` (mapped in
   §7.2), and
2. asynchronously, as a targeted `RoomEventKind::MoveRejected` (mapped in
   §7.3, optionally carrying a snapshot).

Both map to `game.move_rejected`. If B2 uses both for the same command, the
mover gets **two** rejection events. B3 implements both mappers regardless
(robust to either convention), but agree with B2 **in writing** on Day 1:

- **Recommended v1 convention:** synchronous rejections travel as
  `CommandOutcome::Rejected` (§7.2 path, `snapshot: None`); the
  `RoomEventKind::MoveRejected` event is reserved for actor-side detections
  where a snapshot is worth attaching, and whenever B2 uses it, the
  corresponding `send` must NOT also return `Rejected` (e.g. it returns
  `Accepted`, or the rejection was noticed outside a command).
- Record the agreement as a comment on `CommandOutcome::Rejected` /
  `RoomEventKind::MoveRejected` in `contract.rs` (that comment edit is part
  of the B2+B3 PR, per the contract's change rule).

## 8. What B3 stores per connection

Everything lives as locals of the connection task (doc 01 §4) — nothing is
shared between connections, nothing is global:

| Field | Type | When |
|---|---|---|
| `handle` | `Option<R::Handle>` | set at bind (create/join/resume), used for every in-room command + teardown `Disconnected` |
| `events` | `Option<mpsc::Receiver<RoomEvent>>` | set at bind, right before the private reply |
| `player_id` | `Option<String>` | set at bind — injected into every `RoomCommand` |
| `room_code` | `Option<String>` | set at bind — logging + tracing only |
| connection id | `uuid::Uuid` | created at socket start, on every log line |
| `malformed_budget`, `missed_pongs`, rate-bucket state | counters | doc 01 §6–§8, doc 02 §2.2/§4.4 |

**Never store `player_token`.** It is read exactly once — inside the
`session.resume` flow, passed by reference to
`registry.resume_room(&code, &token)` — and then dropped. B2 keeps the hash
server-side; the client (F3) owns the token in secure storage for
reconnection. Your in-room commands need no token: the binding IS the
authorization (§5.2). This is the invariant that makes "token never in a
broadcast, log or game.state event" (`docs/protocol-v1.md` §8.1) hold by
construction: after bind, no code path can even produce a token.

## 9. Open decisions register (status day 0)

Everything above is settled except these — each is small, each has an owner
and a next action. Do not silently pick one: each crosses a module boundary.

| # | Decision | Status | Recommendation | Who agrees |
|---|---|---|---|---|
| D1 | Command-id dedup location (`RoomCommand` carries no id; doc §5 wants actor-side dedup) | **OPEN — Day 1** | Option A: B2 dedups; extend `RoomCommand` with the id in a B2+B3 PR (§5.3) | B2 + B3 |
| D2 | Single rejection channel (`CommandOutcome::Rejected` vs `RoomEventKind::MoveRejected`) | **OPEN — Day 1** | v1: outcome path for sync rejections; event path only when B2 attaches a snapshot; never both (§7.4) | B2 + B3 |
| D3 | `room.leave` success behavior (no ack event exists in v1) | **OPEN** | On accepted `Leave`: send Close(1000, "left room") and end the connection; on rejected: `server.error`. F3 must treat that close as success (not as a reconnect trigger). Alternative rejected: keeping the socket unbound for re-binding invites room-hopping abuse. | B3 + F3 |
| D4 | `room.leave` payload shape — **verified divergence** | **OPEN — Day 1 freeze review** | see below | B3 + F3 |
| D5 | How a resumed client learns the current `expected_revision` (`session.ready` carries no revision; `Resumed` carries none either) | **OPEN** | B2 emits `RoomEventKind::Snapshot { revision, … }` to the resumed connection right after the private reply — zero wire change, the client reads the envelope revision from that `game.state` (§6.3) | B2 + B3, F3 informed |
| D6 | Unknown extra fields in `payload` | **KNOWN GAP, acceptable v1** | serde's default *ignores* unknown fields (doc §3 says `INVALID_PAYLOAD`); missing/wrong-typed fields are rejected. Strict rejection would need `deny_unknown_fields` on the payload variants — a protocol PR (and serde's `flatten`+`deny` interactions must be re-tested). Harmless in v1 (extra fields carry no semantics); park it. | B3 + F3, low priority |

### D4 in detail — `room.leave` payload `{}` vs no payload (verified)

Empirically verified against the frozen types (probe + doc 03 tests):

- Today's Rust: `ClientCommand::LeaveRoom` is a **unit variant**; with
  `#[serde(tag = "type", content = "payload")]`, it serializes as
  `{"v":1,"id":"i","type":"room.leave"}` — **no `payload` key** — and
  deserialization accepts `payload` **absent or `null`** but **rejects
  `"payload": {}`** (`invalid type: map, expected unit variant`).
- `docs/protocol-v1.md` §2.1 says `payload` is **Required ✔** and §3 lists
  `room.leave | {}`; F3's `frontend/src/types/protocol.ts` mirrors that
  (`payload: EmptyPayload`). **A TS client sending `{}` would today get
  `INVALID_PAYLOAD` from the Rust server.**

**Recommended fix (one small protocol PR, both sides, per the change rule):**
change the unit variant to a zero-field struct variant in
`backend/src/protocol/client_message.rs`:

```rust
// before (unit variant):  LeaveRoom,
// after  (struct variant):
    /// Leave the room explicitly.
    #[serde(rename = "room.leave")]
    LeaveRoom {},
```

Verified behavior of the struct-variant form: serializes
`"payload":{}` (matching the doc and TS), accepts `{}`, rejects
`null`/absent. Two mechanical follow-ups in the same PR: construction sites
must write `ClientCommand::LeaveRoom {}` (brace-less unit syntax no longer
compiles — `backend/src/protocol/mod.rs`'s
`every_client_command_round_trips` constructs it), and F3 confirms the TS
side. Alternative (rejected): keep the unit variant and rewrite doc §2.1/§3
to "payload omitted for `room.leave`" — more doc surgery, and the
"payload always present" rule is nicer for clients. Either way, update
`tests/network_protocol.rs::leave_wire_shape_matches_the_frozen_types`,
which pins today's behavior with the divergence marked.
