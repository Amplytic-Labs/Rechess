# Module `rooms` — Owner: **B2** (Room Management and Game Lifecycle)

> **Read this file completely before writing any code.**
> It tells you what you are expected to do, how to do it, what the result must
> be, and exactly where the boundary to your teammates' modules is — so that
> nobody steps on anyone else's work.

---

## 1. Your mission

Deliver the room layer: create rooms, generate room codes, manage seats,
tokens and reconnection, and give every room a **single actor task** that
serializes all state changes. You turn B3's validated protocol commands into
sequentially-processed `RoomCommand`s and produce outbound events both
players receive.

You are **not** implementing chess rules (B1 owns `src/game/`) and **not**
WebSockets or JSON (B3 owns `src/network/` and the wire types).

## 2. What you own / what you don't touch

| You own (`src/rooms/`) | You do NOT touch |
|---|---|
| Room registry (code → room handle) | `src/game/` (B1) — you call its public API only |
| One actor task per room owning its `RoomState` | `src/network/` (B3) |
| Room code generation + collision checks | Wire-format structs (B3 owns them) |
| Player seats, private reconnect tokens, token verification | `src/main.rs` (B3 asks, you expose) |
| Room revision counter, command-ID dedup | `frontend/` (F-team) |
| Disconnect grace, resume, expiry, rematch state | |
| The rooms module's unit tests | |

## 3. Core architecture rules (from the master plan)

1. **One room actor owns the mutable state of one room.** Commands are
   processed sequentially by that actor — there is never concurrent write
   access to the same board. You achieve this with a Tokio task + a command
   channel, **not** with a `Mutex<RoomState>` held across awaits.
2. **Registry locks are never held across network I/O or awaits.** A short
   `std::sync::Mutex`/`RwLock` over the code→handle map, cloned handle out,
   guard dropped, then await. Never `.lock().await` chains.
3. **All channels are bounded** (`tokio::sync::mpsc::channel(n)`, not
   `unbounded`). Define what happens on a full queue (e.g. disconnect the
   slow client); a slow client must not cause unbounded memory growth.
4. WebSocket reader/writer separation happens in B3; you only publish
   per-player outbound events.

## 4. Facts you need on Day 1

### The Tokio actor pattern (canonical shape)

```rust
use tokio::sync::{mpsc, oneshot};

// A command with a reply channel — the actor processes them ONE AT A TIME.
pub struct RoomCommand {
    pub from_player: PlayerId,
    pub action: RoomAction,          // your design: move/draw/resign/rematch/...
    pub reply: oneshot::Sender<RoomOutcome>,   // optional for fire-and-forget
}

pub struct RoomHandle {
    tx: mpsc::Sender<RoomCommand>,
    // your outbound event model (per-player and broadcast senders)
}

impl RoomHandle {
    pub async fn send(&self, cmd: RoomCommand) -> Result<RoomOutcome, RoomClosed> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(RoomCommand { action: cmd.action, reply: tx, ..cmd }).await?;
        rx.await.map_err(|_| RoomClosed)
    }
}

async fn room_actor(mut rx: mpsc::Receiver<RoomCommand>, mut state: RoomState) {
    while let Some(cmd) = rx.recv().await {
        state.apply(cmd);   // sequential — no locking needed inside the actor
    }
}
```

Reference implementations to read (the pattern is standard): the *Tokio
tutorial — "Shared state" and "Channels"* chapters, and Alice Ryhl's
*"Actor implementation"* / *"Actor with channels"* guides.

### Room codes and tokens (crates already pinned in `Cargo.toml`)

- `uuid = "1.29.3"` (feature `v4`): `Uuid::new_v4()` — OS-entropy based.
- `rand = "0.10.4"`: current API is `rand::rng()` (NOT the old
  `thread_rng()`), `rand::random_range(0..alphabet.len())`, and
  `rand::fill(&mut bytes)`. Verify on Day 1 that `rand::rng()` implements
  `CryptoRng`; if it does not, generate codes/tokens from OS entropy
  (e.g. via the `getrandom`/`OsRng` route) — your brief requires a
  *cryptographically secure* random source for codes and reconnect tokens.

```rust
// Room code: 8 chars, no visually ambiguous characters (no 0/O, 1/I/L).
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
fn generate_room_code() -> String {
    // sample 8 indices from CODE_ALPHABET with the secure rng;
    // then CHECK FOR COLLISION against active rooms before inserting.
    todo!("Day-1 PoC")
}

// Reconnect token: opaque, high-entropy, private.
// Store only a HASH/verifier in RoomState — never the token itself —
// and never include it in any broadcast, event or log line.
fn generate_player_token() -> String { todo!("uuid or 32 random bytes hex") }
```

## 5. Your contracts with neighbours

### From B1 (`src/game/`) you consume

```rust
let mut engine = ChessEngine::new_game();
let snap = engine.apply_move(from, to, promotion)?;   // typed GameError
let snap = engine.resign(side)?;                       // etc.
```

You wrap B1's `ChessEngine` as the room's authoritative game state. **Your
room actor** must validate that the sending player is allowed to submit the
requested action (correct seat, correct turn via B1's error) before calling
the engine. Before B1's final API is agreed (Day 1), you can stub the engine
behind a tiny trait of your own so your tests run without B1's code.

### To B3 you hand off (this is the critical contract for linking)

B3 must never access or mutate `RoomState` directly. Publish exactly this
shape of API and agree the final names with B3 **on Day 1**:

```rust
pub struct RoomRegistry { /* code → handle map, insert/expiry logic */ }

impl RoomRegistry {
    /// Create a room; host sits White. Returns the public room code, the
    /// host's player id + PRIVATE reconnect token, and a RoomHandle.
    pub async fn create_room(&self, host_nickname: String) -> CreateRoomResult;
    /// Find a room by normalized (uppercased, trimmed) room code.
    pub fn lookup(&self, code: &str) -> Option<RoomHandle>;
}

pub enum JoinError { Full, NotFound, AlreadyActive, Expired }
pub enum ResumeError { NotFound, InvalidToken, /* ... */ }

pub struct CreateRoomResult {
    pub room_code: String,
    pub player_id: PlayerId,
    pub player_token: String,        // PRIVATE — B3 returns it only to the creator
    pub handle: RoomHandle,
    // B2 owns how outcomes distinguish private responses from public broadcasts
}
```

Requirements on this boundary:

- Join/Resume results **must distinguish** the private credential response
  from the public room snapshot, so B3 can send the token to exactly one
  connection and broadcast only public data.
- Outbound event model: the room actor publishes per-player events (targeted
  responses) and public room/game broadcasts. Design the sender side; B3 owns
  the receiver→socket side.

## 6. Required behaviour checklist (your brief, expanded)

- Room codes: 8 chars, unambiguous alphabet, collision-checked, unique among
  *active* rooms.
- `create` → host in White seat, Black empty, status `waiting`.
- `join` → only into a `waiting` room with a free seat; exactly one of two
  simultaneous joins wins. Malformed/full/active/expired codes → stable typed
  errors mapped by B3 to `ROOM_NOT_FOUND` / `ROOM_FULL` / ...
- Reconnect token: stored hashed; a resume with a valid token cancels the
  disconnect deadline and returns the **full current snapshot** (revision
  included).
- Revision: monotonically increasing, bumped on every accepted
  state-changing command; commands with stale `expected_revision` are
  rejected (mapped by B3 to `STALE_REVISION`).
- Command-ID dedup: remember recent command IDs per room/player within a
  bounded window so client retries never apply a move twice.
- Status transitions: `waiting → active → finished → (retention) → expired`.
  Expiry removes the room and its actor without leaking tasks/channels
  (drop all senders; the actor's `recv()` returns `None` and ends the task).
- Rematch: both players must vote; then reset board/history/draw state via
  B1's `reset_for_rematch()` and restart with a fresh game identity.
- Room isolation: activity in room A can never affect room B (B4 tests this
  with 50+ rooms).

## 7. Suggested module layout (yours to adjust)

```
src/rooms/
├── mod.rs        // re-exports; pub use RoomRegistry, RoomHandle
├── registry.rs   // code → handle map, code generation, expiry sweep
├── room.rs       // RoomState, seats, status transitions, revision
├── room_actor.rs // the actor loop + command handling
├── player.rs     // PlayerId, seat, token verifier (hashed)
├── errors.rs     // JoinError / ResumeError / RoomError (typed, stable)
└── events.rs     // outbound event model for B3
```

## 8. How to test your module independently

No sockets needed. Use `#[tokio::test]` with real channels:

```bash
cd backend
cargo test rooms::
```

Test list (from your brief — turn each into a test before B4 asks):

1. Create → unique code, host in White seat.
2. Join valid waiting room; reject invalid/full/active/expired codes.
3. Two simultaneous joins for one remaining seat → exactly one succeeds
   (`tokio::join!` two `send`s).
4. Wrong-room or wrong-seat token cannot move or resume.
5. A second player cannot overwrite an existing seat.
6. One room actor serializes concurrent move requests (spawn many commands,
   assert order/revision).
7. Stale revision and duplicate command ID are rejected safely.
8. Disconnect starts the grace deadline; resume cancels it and returns the
   latest state (use `tokio::time::pause()` or short deadlines in tests).
9. Expiry removes room + registry entry without leaking the task
   (`tokio::task::yield_now()` then assert the actor ended).
10. Rematch requires both votes and resets the game consistently.

For engine-dependent tests, inject B1's engine — or a minimal fake behind
the trait — so your tests don't block on B1's progress, then swap in the
real one when their vertical slice lands (Day 3–4).

## 9. Deliverables (definition of done)

- [ ] `RoomRegistry` + `RoomHandle` + actor task per room, channels bounded
- [ ] Typed, stable errors for every join/resume/lifecycle failure
- [ ] Revision counter + command-ID dedup window
- [ ] Disconnect grace → resume → full snapshot flow
- [ ] Status transitions with retention expiry and no resource leaks
- [ ] Rematch by mutual agreement resetting via B1's engine
- [ ] Unit tests for §8 (all 10 scenarios)
- [ ] This folder's README updated with your final public API (§5)

## 10. Key coordination

- **B1** daily: engine/room boundary, seat authorization before engine calls.
- **B3** from Day 1: registry/handle API, outbound event model, revision and
  dedup semantics (B3 maps them to the wire; B3 owns `docs/protocol-v1.md`).
- **B4** from Day 1: they will test your races and lifecycle — give them
  deterministic fixtures and small delays/deadline knobs early.
