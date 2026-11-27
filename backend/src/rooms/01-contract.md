# 01 — The Frozen Contract, Explained

> Everything in this file describes **`backend/src/rooms/contract.rs` as it
> exists today**. Signatures are copied verbatim — if this doc and the file
> disagree, the file wins and you fix this doc. You implement these traits;
> B3 consumes them; B4 fakes them (`rooms::testkit`). Dependency direction is
> one-way: `rooms` → `game` (engine trait) and `rooms` → `protocol` (wire
> structs, which **you** assemble). `game` and `protocol` never depend on
> `rooms`.

Note on the trait form: the methods return `impl Future<Output = …> + Send`
(RPITIT — return-position `impl Trait` in traits). **You simply write plain
`async fn` in your `impl` block**; the compiler proves the future is `Send`,
which is exactly what B3 needs for `tokio::spawn`. No `async_trait` crate, no
`Box<dyn Future>`. See `backend/tests/testkit_demo.rs` — the fakes implement
the traits this way and compile today.

---

## 1. Value types

```rust
pub type PlayerId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCredentials {
    pub player_id: PlayerId,
    pub player_token: String,   // SECRET — see §2.3
    pub color: PlayerColor,     // crate::game::PlayerColor
}
```

* `PlayerId` is opaque public data (safe to broadcast). You generate it; a
  UUID v4 is a good choice (§8).
* `PlayerCredentials` is created by `create_room` and `join_room` only.
  `player_token` goes **only** to the owning connection via B3's private
  replies (`room.created` / `room.joined`) — never in broadcasts, never in
  logs, never in a snapshot. `color` here is the **domain**
  `crate::game::PlayerColor` (the contract file imports
  `crate::game::{GameResult, PlayerColor, Promotion}`), not the protocol
  mirror. Same for `Resumed.color` and `RoomCommand::Move::promotion`.

### 1.1 Success payloads

```rust
pub struct RoomCreated<H> {
    pub room_code: String,
    pub credentials: PlayerCredentials,   // host seat — White
    pub handle: H,
    pub room: crate::protocol::RoomPublicState,   // waiting, host seated
}

pub struct RoomJoined<H> {
    pub credentials: PlayerCredentials,   // guest seat — Black
    pub handle: H,
    pub room: crate::protocol::RoomPublicState,   // both seats filled
}

pub struct Resumed<H> {
    pub player_id: PlayerId,
    pub color: PlayerColor,               // game::PlayerColor
    pub handle: H,
    pub snapshot: crate::protocol::GameSnapshot, // FULL current snapshot
}
```

`H` is your handle type (`RoomRegistry::Handle`). You already assemble the
`RoomPublicState` / `GameSnapshot` values when you build these structs — the
mapping job of §7.

## 2. `RoomRegistry` — what B3 calls

```rust
pub trait RoomRegistry: Send + Sync {
    type Handle: RoomHandle;

    fn create_room(&self, nickname: &str)
        -> impl Future<Output = Result<RoomCreated<Self::Handle>, CreateRoomError>> + Send;

    fn join_room(&self, room_code: &str, nickname: &str)
        -> impl Future<Output = Result<RoomJoined<Self::Handle>, JoinRoomError>> + Send;

    fn resume_room(&self, room_code: &str, token: &str)
        -> impl Future<Output = Result<Resumed<Self::Handle>, ResumeError>> + Send;

    fn lookup(&self, room_code: &str)
        -> impl Future<Output = Option<Self::Handle>> + Send;
}
```

Semantics you commit to:

1. **`create_room(nickname)` — host = White.** Generate a fresh room code
   (§8), spawn a new room actor, seat the host in White with an empty Black
   seat, status `waiting`, `revision = 0`. Return the code, the host's private
   credentials, your handle, and the initial public state. `nickname` is
   already validated by B3 (1–20 chars, no control characters) — you may
   still treat it as untrusted text.
2. **`join_room(room_code, nickname)` — guest = Black, first-wins.** Join is
   only legal into a `waiting` room with a free Black seat. Two simultaneous
   joins for the one remaining seat: **exactly one succeeds** — the room actor
   is the single serialized point of truth, so the race is decided by command
   order, not by lock-free luck (see [02-architecture.md §3](02-architecture.md)).
   `room_code` arrives **already normalized (uppercased/trimmed) by B3** —
   you do not re-normalize, but you may defensively reject codes that are not
   8 chars from the unambiguous alphabet.
3. **`resume_room(room_code, token)` — seat by token.** Find the room, find
   the seat whose token verifier matches, cancel that seat's disconnect
   grace, mark it connected, and return the **full current `GameSnapshot`**
   (protocol v1 §7: the client replaces its whole local state with it).
4. **`lookup(room_code)` — routing hook.** `Some(handle)` for a live room
   (B3 then uses the handle to route in-room commands), `None` otherwise.
   This is a cheap map lookup — never spawn or wake the actor for it.

### 2.1 Error enums → wire error codes

B3 maps these; you return them. The mapping (see `docs/protocol-v1.md` §6):

| Your error | Wire `ErrorCode` | When you return it |
|---|---|---|
| `CreateRoomError::Internal(String)` | `INTERNAL` | Unexpected failure (code-space exhaustion etc.). Keep the string for logs only — never send internals to clients. |
| `JoinRoomError::NotFound` | `ROOM_NOT_FOUND` | No live room (and no recent tombstone) for this code. |
| `JoinRoomError::Full` | `ROOM_FULL` | Room exists but both seats are taken — **including an `active` room**: a room that already started is full; there is no separate "already active" variant. |
| `JoinRoomError::Expired` | `ROOM_EXPIRED` | Code matches a recently-expired room (needs tombstones, see [02-architecture.md §9](02-architecture.md)). |
| `JoinRoomError::Internal` | `INTERNAL` | Unexpected failure. |
| `ResumeError::NotFound` | `ROOM_NOT_FOUND` | Unknown room code. |
| `ResumeError::InvalidToken` | `INVALID_TOKEN` | Room exists, token matches no seat (wrong token, other room's token). |
| `ResumeError::Internal` | `INTERNAL` | Unexpected failure. |

## 3. `RoomHandle` — the door into one actor

```rust
pub trait RoomHandle: Clone + Send + Sync {
    fn send(&self, command: RoomCommand)
        -> impl Future<Output = Result<CommandOutcome, RoomClosed>> + Send;

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<RoomEvent>;
}
```

* **`Clone` shares the same room.** All clones are views onto one actor's
  command channel and event fan-out.
* **`send(command)`** resolves with the actor's per-command reply. It awaits
  when the actor's command queue is full (backpressure — that's by design,
  capacity discussion in [02-architecture.md §4](02-architecture.md)).
* **`Err(RoomClosed)`** means the room expired/finished and its actor ended —
  B3 turns this into a socket teardown / lobby return. `RoomClosed` is a
  unit struct implementing `Display` (`"room closed"`) and `Error`.
* **`subscribe()`** is synchronous (no await). One bounded receiver per
  subscriber; capacity and overflow policy are yours to pick and document
  (recommendation in [02-architecture.md §5](02-architecture.md)).

## 4. `RoomCommand` — what goes in (every variant)

B3 translates protocol commands into these; **your actor processes them
sequentially**. `expected_revision` is the client-supplied freshness guard —
a stale value is rejected with `CommandOutcome::Rejected` +
`ErrorCode::StaleRevision` (§9).

```rust
pub enum RoomCommand {
    Move   { player: PlayerId, from: String, to: String,
             promotion: Option<Promotion>, expected_revision: u64 }, // Promotion = game::Promotion
    OfferDraw  { player: PlayerId, expected_revision: u64 },
    AcceptDraw { player: PlayerId, expected_revision: u64 },
    Resign     { player: PlayerId, expected_revision: u64 },
    Rematch    { player: PlayerId, expected_revision: u64 },
    Leave      { player: PlayerId },
    Disconnected { player: PlayerId },   // B3 notifies you a socket dropped
}
```

Variant-by-variant notes (full state-transition table in §9):

* `Move` — squares are `"a1".."h8"` strings; `promotion` is the **domain**
  `game::Promotion` (`Queen|Rook|Bishop|Knight`). B3 converts the wire
  `"q"|"r"|"b"|"n"` for you. You convert to `game::MoveRequest` and call the
  engine (after checking the seat is allowed to submit — §6).
* `OfferDraw` / `AcceptDraw` — mirror `game.offer_draw` / `game.accept_draw`
  semantics (offering when the opponent has an active offer = accepting).
* `Resign` — the opponent wins by `Resignation`.
* `Rematch` — one vote per player; the new game starts when both voted.
* `Leave` — explicit exit; you close the room (see §9 and [02-architecture.md §9](02-architecture.md)).
* `Disconnected` — **not a client command**; B3's socket-closed notification.
  Starts the disconnect grace for that player. Carries no `expected_revision`
  (it is not a state-changing command).

## 5. `CommandOutcome` — the reply to the sender

```rust
pub enum CommandOutcome {
    Accepted { revision: u64 },            // revision AFTER this command
    Rejected { error: crate::protocol::ErrorCode, message: String, revision: u64 },
}
```

* `Accepted.revision` is the room revision **after** applying the command
  (i.e. the bumped value). A client that sent `expected_revision: 12` and
  gets `Accepted { revision: 13 }` knows exactly where it stands.
* `Rejected` carries the current (unchanged) revision so the client can
  reconcile from the newest snapshot. `error` is the **protocol**
  `ErrorCode` (you pick it — mapping tables in §6.1/§6.2); `message` must be
  safe for display (no internals, no tokens).
* Rejections are also published as the targeted `MoveRejected` event (§6.2).

## 6. `RoomEvent` — what comes out, and who gets it

```rust
pub struct RoomEvent {
    pub target: Option<PlayerId>,   // None = broadcast, Some(id) = one player
    pub kind: RoomEventKind,
}

impl RoomEvent {
    pub fn broadcast(kind: RoomEventKind) -> Self;   // target: None
    pub fn to(player: PlayerId, kind: RoomEventKind) -> Self;
}
```

```rust
pub enum RoomEventKind {
    Snapshot { revision: u64, snapshot: crate::protocol::GameSnapshot },
    Presence { player_id: PlayerId, connected: bool },
    Finished { revision: u64, result: GameResult, snapshot: crate::protocol::GameSnapshot },
    MoveRejected { player_id: PlayerId, error: crate::protocol::ErrorCode,
                   message: String, revision: u64,
                   snapshot: Option<crate::protocol::GameSnapshot> },
}
```

**Target routing — the rule you implement on the way out:**

| `target` | B3 delivers as | You use it for |
|---|---|---|
| `None` (broadcast) | `game.state` / `player.presence` / `game.finished` to every room connection | `Snapshot`, `Presence`, `Finished` |
| `Some(player)` | delivered only on that player's connection (`game.move_rejected`) | `MoveRejected` — **always targeted at the mover** |

How B3 actually routes targeted events: every connection holds one
`subscribe()` receiver **and** knows its own `player_id` (from create/join/
resume). B3 forwards an event when `target.is_none() ||
target == Some(my_player_id)`. Consequence for you: **do not assume the
subscriber registry knows who is who** — fan out every event to every
subscriber and let B3 filter. With two players this is trivially cheap.

Careful with the types: `Finished` carries the **domain**
`game::GameResult` (B3 maps it to the wire mirror), while `MoveRejected`
carries the **protocol** `ErrorCode` and protocol snapshot. This asymmetry is
verbatim from `contract.rs` — do not "fix" it.

### 6.1 Room-side rejections → `ErrorCode` (you choose these)

| Situation in the actor | `ErrorCode` for the `Rejected` outcome / `MoveRejected` event |
|---|---|
| `expected_revision != revision` | `StaleRevision` |
| Command `player` matches no seated player | `InvalidToken` |
| Move while the room is still `waiting` (game not started) | `NotYourTurn` with message "game has not started yet" (open item — see [05-acceptance.md](05-acceptance.md) decisions) |
| Rematch while the game is still in progress | `GameFinished` with message "game is still in progress" (open item) |

### 6.2 Engine errors → `ErrorCode` (after the engine rejects)

| `game::GameError` | `ErrorCode` | Typical message |
|---|---|---|
| `InvalidSquare { .. }` | `IllegalMove` | `"invalid square"` (B3's payload gate usually catches this first as `InvalidPayload`) |
| `IllegalMove` | `IllegalMove` | `"illegal move"` |
| `WrongTurn` | `NotYourTurn` | `"not your turn"` |
| `PromotionRequired` | `IllegalMove` | `"promotion piece required"` |
| `UnexpectedPromotion` | `IllegalMove` | `"promotion given on a non-promotion move"` |
| `GameAlreadyOver` | `GameFinished` | `"game already over"` |

## 7. THE mapping responsibility — you assemble the wire snapshot

The contract file says it and this is the core integrator job: **B2 maps
`game::GameSnapshot` (domain) + room data (code, status, seats) →
`protocol::GameSnapshot` (wire).** `game/` knows nothing about rooms; the wire
structs know nothing about the engine. You are the bridge.

Write the `From` impls **in your module** (e.g. `backend/src/rooms/mapping.rs`)
— they are legal there because `game::PlayerColor` and `protocol::PlayerColor`
are both local to this crate (orphan rule; the same impls would be rejected in
any other crate). If B1 or B3 ever want their own `From` impls between these
types, that is a coherence conflict — resolve it in a PR, one set of impls
must win.

```rust
use crate::game::{
    GameResult as DomainResult, GameResultReason as DomainReason,
    GameSnapshot as DomainSnapshot, GameStatus as DomainStatus,
    MoveRecord as DomainMove, PlayerColor as DomainColor,
};
use crate::protocol::{
    GameResult, GameResultReason, GameSnapshot, GameStatus, MoveRecord, PlayerColor,
    PlayerPublic, RoomStatus,
};

impl From<DomainColor> for PlayerColor {
    fn from(c: DomainColor) -> Self {
        match c {
            DomainColor::White => PlayerColor::White,
            DomainColor::Black => PlayerColor::Black,
        }
    }
}

impl From<DomainStatus> for GameStatus {
    fn from(s: DomainStatus) -> Self {
        match s {
            DomainStatus::InProgress => GameStatus::InProgress,
            DomainStatus::Check => GameStatus::Check,
            DomainStatus::Checkmate => GameStatus::Checkmate,
            DomainStatus::Stalemate => GameStatus::Stalemate,
            DomainStatus::Draw => GameStatus::Draw,
            DomainStatus::Resigned => GameStatus::Resigned,
        }
    }
}

impl From<DomainReason> for GameResultReason {
    fn from(r: DomainReason) -> Self {
        match r {
            DomainReason::Checkmate => GameResultReason::Checkmate,
            DomainReason::Stalemate => GameResultReason::Stalemate,
            DomainReason::Resignation => GameResultReason::Resignation,
            DomainReason::DrawAgreed => GameResultReason::DrawAgreed,
            DomainReason::InsufficientMaterial => GameResultReason::InsufficientMaterial,
            DomainReason::ThreefoldRepetition => GameResultReason::ThreefoldRepetition,
            DomainReason::FiftyMoveRule => GameResultReason::FiftyMoveRule,
        }
    }
}

impl From<DomainResult> for GameResult {
    fn from(r: DomainResult) -> Self {
        GameResult {
            winner: r.winner.map(PlayerColor::from),
            reason: GameResultReason::from(r.reason),
        }
    }
}

impl From<DomainMove> for MoveRecord {
    fn from(m: DomainMove) -> Self {
        MoveRecord { from: m.from, to: m.to, san: m.san }
    }
}
```

The snapshot builder — in the real module it takes `&RoomState` and calls
`engine.snapshot()`; shown here with plain arguments so the shape is
unambiguous (this exact body compiles against the real crates):

```rust
/// Domain engine snapshot + room data -> wire snapshot. Never fails, never
/// sees a token: the ONLY place player secrets could leak is `players`,
/// which is built from seats with no token fields at all.
pub fn build_snapshot(
    room_code: &str,
    room_status: RoomStatus,
    seats: &[SeatView],          // your seat view: id, nickname, color, connected
    engine: &DomainSnapshot,
) -> GameSnapshot {
    GameSnapshot {
        room_code: room_code.to_string(),
        status: room_status,                       // room status is YOUR state
        game_status: GameStatus::from(engine.status),
        result: engine.result.map(GameResult::from),
        fen: engine.fen.clone(),
        turn: PlayerColor::from(engine.turn),
        last_move: engine.last_move.clone().map(MoveRecord::from),
        move_history: engine.history_san.clone(),  // SAN list, oldest first
        legal_moves: engine.legal_moves.clone(),    // UCI-style, server-generated only
        players: seats
            .iter()
            .map(|s| PlayerPublic {
                id: s.player_id.clone(),
                nickname: s.nickname.clone(),
                color: PlayerColor::from(s.color),
                connected: s.connected,
            })
            .collect(),
        draw_offer: engine.draw_offered_by.map(PlayerColor::from),
    }
}
```

Room status: store `protocol::RoomStatus` directly in your `RoomState`
(`Waiting | Active | Finished | Expired`). There is no domain twin — the
room status is *your* concept, so using the wire enum avoids a third mirror.

### 7.1 Complete worked example — a room after `1. e4`

Inputs the actor has after the host's move is accepted:

```
RoomState {
    code: "TESTCODE1", status: Active, revision: 2,
    white: Some(Seat { player_id: "p-host", nickname: "Host",   connected: true, .. }),
    black: Some(Seat { player_id: "p-guest", nickname: "Guest", connected: true, .. }),
    engine: <snapshot from B1 after 1. e4>,
}
```

```
game::GameSnapshot {                    // B1's domain snapshot
    fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
    turn: White→Black after the move: PlayerColor::Black,
    status: GameStatus::InProgress, result: None,
    last_move: Some(MoveRecord { from: "e2", to: "e4", san: "e4" }),
    history_san: ["e4"],
    legal_moves: ["e7e5", "e7e6", …],
    draw_offered_by: None,
}
```

`build_snapshot` produces exactly this wire object (matches the fixture in
`rooms::testkit::sample_protocol_snapshot` / `protocol::tests::sample_snapshot`):

```
protocol::GameSnapshot {
    room_code:    "TESTCODE1",            // from YOUR RoomState
    status:       RoomStatus::Active,     // from YOUR RoomState
    game_status:  GameStatus::InProgress, // ← engine (mapped)
    result:       None,                    // ← engine (mapped)
    fen:          "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
    turn:         PlayerColor::Black,     // ← engine (mapped)
    last_move:    Some(MoveRecord { from: "e2", to: "e4", san: "e4" }),  // ← engine
    move_history: ["e4"],                  // ← engine.history_san
    legal_moves:  ["e7e5", "e7e6", …],     // ← engine.legal_moves
    players: [
        PlayerPublic { id: "p-host",  nickname: "Host",   color: White, connected: true },
        PlayerPublic { id: "p-guest", nickname: "Guest", color: Black, connected: true },
    ],                                    // ← YOUR seats (no token fields!)
    draw_offer:   None,                   // ← engine.draw_offered_by
}
```

Serialization note: on the wire this becomes the `game.state` payload with
`revision` on the **envelope** (B3's job, `protocol::envelope.rs` — the
`RoomEventKind::Snapshot` field you fill is the same number).

## 8. Room codes and credentials (what you generate)

* **Room code**: 8 chars from `ABCDEFGHJKMNPQRSTUVWXYZ23456789` (no `0/O`,
  no `1/I`, also no `L` — visually ambiguous). Generate with the secure RNG
  and regenerate on collision against active rooms **and** recent tombstones.
  Codes are identifiers, not secrets. Full recipe (rand 0.10 API, verified)
  in [02-architecture.md §7](02-architecture.md).
* **`player_id`**: opaque public id, e.g. `format!("p-{}", Uuid::new_v4().simple())`.
  Not a secret, but unique per seat.
* **`player_token`**: 32 random bytes hex-encoded (64 chars). SECRET. You
  store **only a hash/verifier** per seat — the raw token exists in memory
  only between generation and the one private reply. See
  [02-architecture.md §8](02-architecture.md) for the hashing choice
  (SHA-256 via a `sha2` PR is the recommendation).

## 9. Revision rules (protocol v1 §5 — your bible)

* `revision` is **per-room, monotonic, starts at 0** at `create_room`, and is
  bumped by exactly 1 for every **accepted state-changing command**.
* State-changing commands carry `expected_revision`; if it does not equal the
  room's current revision → `Rejected { error: StaleRevision, revision: current }`
  (+ targeted `MoveRejected` if it was a `Move`), **state untouched**.
* `revision` is **never reset by a rematch** — it is a per-room counter, not
  per-game (protocol v1 §5). The new game after a rematch continues from the
  current revision.
* Only the actor reads/writes the revision (single writer —
  [02-architecture.md §2](02-architecture.md)).

The bump table (your decision set, agreed with B3 — flagged in
[05-acceptance.md](05-acceptance.md) as open items where marked):

| Event | Bump? | Outbound |
|---|---|---|
| `create_room` | revision = 0 | none (host gets `RoomCreated.room`) |
| `join_room` fills Black, `waiting → active` | **+1** ⚠ | broadcast `Snapshot` at the new revision (the host's "game starts now" signal) |
| `Move` accepted | +1 | broadcast `Snapshot` |
| `OfferDraw` accepted | +1 | broadcast `Snapshot` (`draw_offer` field changed) |
| `AcceptDraw` accepted → `DrawAgreed` | +1 | broadcast `Finished { result, snapshot }` |
| `Resign` accepted | +1 | broadcast `Finished` (winner = opponent, reason `Resignation`) |
| `Rematch` — first vote | no bump (nothing observable changed) | none; `Accepted { revision: current }` ⚠ |
| `Rematch` — second vote → new game | +1 | broadcast `Snapshot` at the new revision (fresh board, status `active`, draw/votes cleared) |
| `Disconnected` | no bump | broadcast `Presence { connected: false }`; start grace |
| resume | no bump | broadcast `Presence { connected: true }`; resumer's full snapshot goes in `Resumed` |
| `Leave` | room closes | actor shuts down (see below) |
| Any rejected command | no bump | `Rejected { …, revision: current }` + targeted `MoveRejected` for rejected moves |

⚠ = open decision to confirm with B3 before B4 writes wire-level tests
(alternatives: join without bump; first rematch vote broadcasting a
snapshot; a dedicated rematch-vote event — a protocol-v1 extension).

## 10. Dedup window (recent command IDs)

Protocol v1 §5: *every client command `id` is unique; the room actor
remembers recent IDs per player in a bounded window so a retried `game.move`
is not applied twice (idempotent-accepted with the original outcome or
`STALE_REVISION`, agreed by B2+B3).*

**Contract gap you must resolve first:** the frozen `RoomCommand` variants
carry **no command id** — the envelope `id` (B3's parse layer) is dropped
before your actor sees the command. So today you *cannot* dedup in the
actor. Options, decide with B3 in a joint PR:

1. **(Recommended)** Add `command_id: String` to the state-changing
   `RoomCommand` variants (`Move`, `OfferDraw`, `AcceptDraw`, `Resign`,
   `Rematch`). B3 passes the envelope `id` through; your actor keeps a
   bounded per-player window (e.g. last 32 ids: `HashMap<PlayerId,
   VecDeque<String>>` plus a cap) and returns the recorded outcome for a
   retry.
2. Dedup entirely in B3's connection task (it sees the ids); your actor
   stays id-free. Simpler for you, but retries that cross a reconnect land in
   a *new* connection task — B3 must then keep per-player id memory somewhere
   anyway.

Until the PR lands, implement the window behind a small internal type
(`BoundedDedupCache`) so wiring it in is a one-line change; document in
[05-acceptance.md](05-acceptance.md).

## 11. Deadlines (lifecycle defaults from the brief)

| Deadline | Default | Starts on | Effect at expiry |
|---|---|---|---|
| Waiting-room expiry | 10 min | `create_room` | Room `expired`, closed, removed from the registry |
| Disconnect grace | **60 s** | `Disconnected` (per player) | Seat forfeits (game in progress: that player loses by `Resignation` — ⚠ no `Timeout` reason in protocol v1, see open items); room with no live game just closes |
| Finished-room retention | 10 min | game finish | Room `expired`, closed, removed |
| Empty room | promptly | last seat gone | Close and remove |

All of these must be **config constants injected via `RoomConfig`**
(`docs` master plan §5.3), never hardcoded — B4's tests need millisecond
knobs. Driving mechanism (tokio time, `sleep_until` in the actor loop) is
[02-architecture.md §9](02-architecture.md).

## 12. How B3 consumes all of this

B3 is generic over your registry — the same code must work against
`rooms::testkit::FakeRegistry` in tests and your real registry in production
(proven by `tests/testkit_demo.rs::generic_code_works_against_any_registry`):

```rust
// B3's shape (already compiles today against the fakes):
async fn create_and_subscribe<R: RoomRegistry>(
    registry: &R,
    nickname: &str,
) -> (R::Handle, bool) {
    let created = registry.create_room(nickname).await.expect("create ok");
    let events = created.handle.subscribe();     // per-connection receiver
    (created.handle, events.is_empty())
}

// B3 spawns per-connection tasks like this — this is WHY every trait
// future must be Send (it is, by the RPITIT signatures):
tokio::spawn(async move {
    let outcome = handle.send(RoomCommand::Leave { player: my_id }).await;
    // Err(RoomClosed) => close socket / return to lobby
});
```

Your side of the deal: keep every future you produce `Send` (write plain
`async fn`; never hold a `std::sync::MutexGuard` across an await —
[03-implementation-guide.md §7](03-implementation-guide.md)), and make sure
`send`/`subscribe` are cheap when the room is healthy so B3's connection
tasks never block on each other.
