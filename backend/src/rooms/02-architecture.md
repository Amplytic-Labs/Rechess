# 02 — Architecture: One Actor per Room

> The design you are building. The skeleton code lives in
> [03-implementation-guide.md](03-implementation-guide.md); this file is the
> *why* behind every structural choice, so you can defend them in review and
> adjust them knowingly.

## 1. Big picture

```
                      RoomRegistry (you)                        B3's layer
        ┌──────────────────────────────────────────┐
        │  code → ActorHandle  (map, Mutex-guarded) │◄─── create_room / join_room /
        │  tombstones (recently expired codes)     │     resume_room / lookup
        └──────────────┬───────────────────────────┘
                       │ ActorHandle = clone of:
                       │   command sender (mpsc, bounded 32)
                       │   + Arc<subscriber registry>
        ┌──────────────▼───────────────────────────┐
        │  RoomActor  (one tokio task per room)     │
        │  owns: engine (Box<dyn ChessEngine>)      │
        │         seats + token verifiers           │
        │         revision, dedup window            │
        │         draw offer / rematch votes         │
        │         deadlines (waiting/grace/retention)│
        └──────┬────────────────────────┬───────────┘
     commands  │                        │ events (broadcast fan-out,
     (mpsc 32) │                        │  one bounded mpsc 64 per subscriber)
               ▼                        ▼
        actor loop: select! {  ...   every RoomEvent → every subscriber
          rx.recv(),                 (B3 filters by `target`)
          sleep_until(next_deadline)
        }
```

Two golden rules from the master plan, and everything below serves them:

1. **One room actor owns the mutable state of one room.** No
   `Mutex<RoomState>` shared with anyone, ever. All mutation happens inside
   the actor task, so commands are processed sequentially and there is never
   concurrent write access to a board.
2. **Registry locks are never held across an await.** A short lock over the
   code→handle map, handle cloned out, guard dropped, *then* await.

## 2. What the actor owns (its `RoomState`)

Sketch (types are yours to finalize; `RoomCommand` etc. are frozen):

```rust
pub(crate) struct RoomState {
    code: String,
    status: protocol::RoomStatus,           // Waiting | Active | Finished | Expired
    white: Option<Seat>,
    black: Option<Seat>,
    engine: Option<Box<dyn ChessEngine>>,  // Some from the moment a game exists
    revision: u64,                          // monotonic, starts at 0
    recent_command_ids: BoundedDedupCache,   // §10 of 01-contract.md (pending PR)
    draw_offer: Option<PlayerColor>,        // mirrors engine.draw_offered_by
    rematch_votes: [bool; 2],                // per color; cleared on new game
    waiting_deadline: Option<Instant>,      // created + 10 min
    grace_deadlines: [Option<Instant>; 2],  // per seat, 60 s from Disconnected
    finished_deadline: Option<Instant>,     // finish + 10 min
}

pub(crate) struct Seat {
    player_id: PlayerId,
    nickname: String,
    connected: bool,
    token_verifier: String,   // SHA-256 hex of the raw token — NEVER the token
}
```

The engine is `Option<Box<dyn ChessEngine>>` because a room exists for a few
microseconds-to-minutes before/after a game does (waiting room, expired
room). `ChessEngine: Send` is enough — the actor task owns it exclusively,
no `Sync` needed (that is why B1's trait is `Send` only).

## 3. Why `join`/`resume` are not `RoomCommand`s — the internal message enum

The frozen `RoomCommand` enum has no `Join`/`Resume` variants: those go
through the **registry** (`join_room`/`resume_room`), which returns full
credentials/snapshots. But the registry must not mutate room state itself —
that would break single-writer. Solution: your actor's channel carries an
**internal superset**:

```rust
pub(crate) enum ActorMessage {
    /// A frozen RoomCommand from B3, with the reply channel the
    /// RoomHandle::send future is awaiting.
    Command { command: RoomCommand, reply: oneshot::Sender<Result<CommandOutcome, RoomClosed>> },
    /// Registry-initiated join (from join_room).
    Join { nickname: String, reply: oneshot::Sender<Result<JoinReply, JoinRoomError>> },
    /// Registry-initiated resume (from resume_room).
    Resume { token: String, reply: oneshot::Sender<Result<ResumeReply, ResumeError>> },
}
```

`JoinReply`/`ResumeReply` are your internal structs carrying what the actor
knows (credentials for the new seat, the assembled snapshot, etc.). The
registry sends these, awaits the oneshot, and wraps the reply into the
frozen `RoomJoined`/`Resumed`.

This is also **how the first-wins join race is solved**: two `join_room`
calls both send `Join` messages; the actor processes them sequentially; the
first seats the guest and flips `waiting → active`; the second sees a full
room and gets `JoinRoomError::Full`. No locks, no `compare_exchange` tricks —
the actor *is* the single serialized point of truth.

## 4. The command channel — bounded, capacity **32**

```rust
let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<ActorMessage>(32);
```

**Why 32:** a command is tiny and the actor processes one in microseconds
(engine calls are pure, synchronous logic). 32 buffers the realistic worst
burst (both players double-submitting, plus retries after a reconnect, plus
a `Disconnected` notification racing a `Leave`) while capping memory per
room at a few KB. There is no throughput argument for more: the actor is
never the bottleneck at human chess speed.

**Queue-full behaviour (documented, deterministic):** when the queue is
full, `RoomHandle::send` **awaits** inside `mpsc::Sender::send` — i.e. the
caller (B3's connection task for *that one player*) is suspended until the
actor catches up. The actor is never blocked by a full queue, other rooms
are unaffected, and memory stays bounded. This is standard backpressure and
exactly what the master plan's "bounded channels, defined queue-full
behaviour" clause asks for.

**Actor death:** if the actor has ended and all its senders were dropped,
`send` fails with `SendError` → map to `Err(RoomClosed)`. For the clean-shutdown
case (actor ends while senders still exist) see the drain pattern in
[03-implementation-guide.md §6](03-implementation-guide.md).

## 5. The subscriber registry — one bounded channel per subscriber, capacity **64**

```rust
pub(crate) struct Subscribers {
    inner: std::sync::Mutex<Vec<tokio::sync::mpsc::Sender<RoomEvent>>>,
}
```

* `subscribe()` (sync, from the trait): create `mpsc::channel::<RoomEvent>(64)`,
  push the sender, return the receiver.
* The actor **broadcasts with `try_send` only** — it never awaits a
  subscriber. A slow subscriber must never stall the room.
* On `try_send` error: if the channel is **closed**, prune that sender
  (its subscriber is gone); if it is **full**, apply the drop policy below.

**Why 64:** events are the bursty side — every accepted move produces one
`Snapshot` for *each* of 2 subscribers, plus `Presence` flaps on mobile
networks. 64 keeps a healthy client far from the edge at negligible cost
(the fake in `rooms/testkit.rs` uses the same number).

**Drop policy on overflow (document this in code too):** if a subscriber's
queue is full, **drop that event for that subscriber and emit a
`tracing::warn!`** with room code + revision (never tokens). Rationale: a
client that is 64 events behind is resyncing anyway; snapshots are
idempotent full state, so the *next* snapshot (or a resume with the full
snapshot) heals it. The alternative — dropping the subscriber — punishes a
temporarily slow phone with a forced reconnect; prefer healing.
If you ever see the warning in production logs, that is your signal the
client is wedged; B3 can decide to disconnect it.

## 6. Registry storage: `std::sync::Mutex<HashMap>` (and why not the others)

You need `RoomCode → ActorHandle` plus recently-expired codes. Options:

| Choice | Pro | Con |
|---|---|---|
| `dashmap::DashMap` | lock-free reads, concurrent access | **adds a dependency** (not pinned in `Cargo.toml`); sharded locks are overkill — you touch the map twice per room lifetime (insert, remove) and once per create/join/resume/lookup |
| `tokio::sync::Mutex<HashMap>` | `.await`-able, hard to misuse in async code | buys nothing when the guard is never held across an await; strictly slower than std under contention; *invites* holding it across awaits |
| **`std::sync::Mutex<HashMap>`** ✅ | zero new deps; the guard is non-`Send`-across-await by design (the compiler *enforces* rule 2 of §1) | must clone the handle out and drop the guard before awaiting — a two-line discipline, checked by `clippy::await_holding_lock` |

**Recommendation: `std::sync::Mutex<HashMap<String, ActorHandle>>`.** The
registry lock is only ever taken for a map get/insert/remove (nanoseconds).
Every `async fn` in your `RoomRegistry` impl must follow the same shape:

```rust
async fn lookup(&self, room_code: &str) -> Option<Self::Handle> {
    let handle = self.map.lock().unwrap().get(room_code).cloned();  // guard dies here
    handle   // await nothing — but if you did, it would be AFTER the guard is gone
}
```

`await_holding_lock` (clippy) catches violations — keep it enabled.

## 7. Room-code generation (rand 0.10.4 — verified API)

```rust
/// No 0/O/1/I (and no L): visually unambiguous, per the brief.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";   // 32 chars

fn generate_room_code() -> String {
    let mut code = String::with_capacity(8);
    for _ in 0..8 {
        let idx = rand::random_range(0..CODE_ALPHABET.len());
        code.push(CODE_ALPHABET[idx] as char);
    }
    code
}

fn fresh_room_code(map: &HashMap<String, ActorHandle>,
                   tombstones: &Tombstones) -> String {
    loop {
        let code = generate_room_code();
        if !map.contains_key(&code) && !tombstones.contains(&code) {
            return code;
        }
    }
}
```

* `rand::rng()` returns a `ThreadRng` that **is** a crypto RNG
  (`ThreadRng: TryCryptoRng` in rand 0.10, `Error = Infallible`), seeded
  from OS entropy — your brief's "cryptographically secure source" is
  satisfied. The free functions `rand::random_range(..)` and `rand::fill(..)`
  are the 0.10 API (no `thread_rng()`, no `Rng::gen_range`).
* Collision math: 32^8 ≈ 1.1e12 codes; with, say, 10^4 live rooms the
  collision probability per attempt is ~1e-8 — the retry loop is a formality,
  but keep it (and cap its iterations, e.g. 10, then `CreateRoomError::Internal`).
* Tombstones matter for the retry because reusing a *just-expired* code
  would resurrect a stale join.

## 8. Reconnect tokens: 32 bytes, store only the hash

Generation (verified against rand 0.10):

```rust
fn generate_player_id() -> String {
    format!("p-{}", uuid::Uuid::new_v4().simple())   // uuid 1.29.3, v4 feature
}

fn generate_player_token() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);                            // OS entropy
    bytes.iter().map(|b| format!("{b:02x}")).collect() // 64 hex chars
}
```

Storage rule (brief + master plan §6): **store only a verifier**. The seat
holds `token_verifier`, never `player_token`. Resume lookup: hash the
presented token, compare against each seat's verifier
((player, token) → seat is equivalent to "which seat's verifier matches");
constant-time comparison is not worth the complexity here (verifiers are
not low-entropy secrets), but never log or broadcast either form.

**Hashing choice — open dependency question.** The tokio/axum stack pins no
crypto crate (`shakmaty` is rules-only; `rand`/`uuid` are generation-only):

* **Recommendation: open a PR adding `sha2 = "0.10"` (RustCrypto)** and use
  `Sha256::digest(token)`. It is the standard, audited, pure-Rust choice,
  and the master plan *requires* hashed token storage.
* Interim fallback if the PR is blocked: `uuid::Uuid::new_v5(&Uuid::NIL,
  token)` (SHA-1 under the hood) or a documented in-memory-only store with a
  `// TODO(B2): hashed verifiers, see 05-acceptance.md` — acceptable for a
  demo only; **make the interim choice explicit in writing to B3/B4**
  (your brief: "make assumptions about token storage explicit"). Do NOT
  invent a homemade hash.

## 9. Deadlines via tokio time (in-actor `sleep_until`)

All three deadlines plus the "empty room" rule are **`RoomConfig`
constants** (never hardcoded) and are driven by the actor loop:

* **Recommended: deadline in the actor loop.** After every message the actor
  computes `next_deadline()` = the earliest of (waiting expiry, both grace
  timers, finished retention). If there is one, `select!` between
  `rx.recv()` and `tokio::time::sleep_until(deadline)`; if none, plain
  `recv()`. When the timer fires, run the deadline handler (forfeit / expire
  / close). Precise, zero polling, and the whole lifecycle lives where the
  state lives. Skeleton in [03-implementation-guide.md §5](03-implementation-guide.md).
* Alternative (not recommended as primary): a registry sweeper task that
  scans the map every N seconds. Simpler loop, but it must *message* the
  actor to close a room anyway (registry never mutates state), so you end up
  building the same machinery plus a poller. Worth keeping only as a
  belt-and-braces sweep for tombstone eviction.

Grace-expiry semantics (decide once, document in
[05-acceptance.md](05-acceptance.md)): grace expiry during an **active**
game = that seat **forfeits** → engine `resign(color)`, `Finished` broadcast,
room transitions to finished-retention. Protocol v1 has no `timeout`
`GameResultReason`, so it surfaces as `resignation` — flagged as a candidate
protocol extension. Grace expiry in a **waiting** room with the host gone,
or a room where both seats are gone, just closes and removes the room.

Expiry must remove the room **and** its actor without leaks: the actor
removes its own registry entry (the registry hands each actor a shared
reference to the map + tombstones at spawn time), drops all subscriber
senders, and then drains (see [03-implementation-guide.md §6](03-implementation-guide.md))
so nobody's `send` future hangs forever.

## 10. Rematch

Both players must vote. When the second `Rematch` arrives (revision already
bumped by the finish — votes carry the finish-time `expected_revision`):

1. `engine.reset_for_rematch()` — one call on the **same** engine instance
   (that is why B1's trait has the method; no re-factory needed), clears
   board, history, draw state. Your room side clears `draw_offer` and
   `rematch_votes` itself.
2. `revision += 1`, status → `Active`, broadcast `Snapshot` at the new
   revision (fresh starting position — the clients' "new game" signal).
3. **Revision continues — never resets.** Protocol v1 §5 makes `revision` a
   per-room counter; a rematch is the same room. Clients ignore
   older-revision snapshots, so continuation is also the safe choice.
4. First vote: `Accepted { revision: current }`, no broadcast (nothing in
   `GameSnapshot` changed — note there is no rematch-votes field on the
   wire; flagged in [05-acceptance.md](05-acceptance.md)).

## 11. Lifecycle summary

```
create_room        join_room (first-wins)      game ends (mate/draw/resign)
   waiting ───────────► active ───────────────────► finished ──► expired
   │  10 min               │ 60 s grace               │ 10 min retention      + removed
   │  no join → expired    │ disconnect → resume ok    │
   │                       │ grace expiry → forfeit    │ rematch vote #2
   │                       │      (→ finished)         └──► active (revision continues)
   └── host gone (grace) → expired
```

Server restart: all rooms are lost (MVP, in-memory only) — document it, do
not imply persistence anywhere in logs or events.
