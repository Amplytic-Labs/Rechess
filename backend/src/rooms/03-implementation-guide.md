# 03 — Implementation Guide: Build Order and Skeletons

> A concrete path from empty module to done, with partial skeleton code.
> **The skeletons are shapes, not a copy-paste module** — the real work
> (filling in every `todo!`, the state transitions of
> [01-contract.md §9](01-contract.md), the deadline handlers) is yours.
> Names marked `pub(crate)` are internal; only what `contract.rs` exports is
> public. Every contract type below is copied verbatim from the frozen files.

## 1. Suggested file layout (yours to adjust)

```
backend/src/rooms/
├── README.md            # doc set index (exists)
├── contract.rs          # FROZEN — do not edit outside a B2+B3 PR
├── testkit.rs           # shared fakes for B3/B4 — do not edit outside a PR
├── 01-contract.md … 05-acceptance.md   # this doc set
├── mod.rs               # add: pub mod registry; pub mod room; … + re-exports
├── mapping.rs           # From impls + build_snapshot (01-contract.md §7)
├── config.rs            # RoomConfig: deadlines, capacities, code length
├── registry.rs          # InMemoryRegistry: map, tombstones, code gen, RoomRegistry impl
├── room.rs              # RoomState, Seat, status transitions, revision
├── room_actor.rs        # ActorMessage, ActorHandle, actor loop, RoomHandle impl
└── lifecycle.rs         # deadline computation + handlers (waiting/grace/retention)
```

`mod.rs` additions (this file is yours):

```rust
pub mod config;
pub mod mapping;
pub mod registry;
pub mod room;
pub mod room_actor;
// contract + testkit stay exactly as they are
```

## 2. Build order (checkpoint after every stage)

| Stage | Build | Green checkpoint |
|---|---|---|
| 0 | Read the doc set; `cargo build && cargo test` (15 green) | base confirmed |
| 1 | `config.rs` (RoomConfig) + `mapping.rs` (From impls + `build_snapshot`) + unit tests for the worked example | `cargo test rooms::` — mapping tests green |
| 2 | Registry shell: `ActorHandle` (send/subscribe stubs), map + tombstones, `create_room` + `lookup` only; actor loop that only `recv`s | create/lookup tests green |
| 3 | Actor loop + `Move` command + revision + `Snapshot` broadcast (join/resume still stubbed) | move/revision tests green |
| 4 | `Join`/`Resume` internal messages wired into `join_room`/`resume_room` + seats + tokens | join/resume/token tests green |
| 5 | Draw offer/accept, resign, rematch, `Leave`, `Disconnected` + grace, presence events | command tests green |
| 6 | Deadlines (lifecycle.rs): waiting expiry, grace forfeit, retention, tombstones, no-leak shutdown | time-based tests green; `cargo test` all green |

Stage 1 needs no actor at all — that is deliberate: the mapping is the part
B3's wire tests depend on most, so prove it first.

## 3. `RoomConfig` — every knob injected (B4 needs them small)

```rust
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RoomConfig {
    pub code_len: usize,                 // 8
    pub waiting_expiry: Duration,        // 10 min
    pub disconnect_grace: Duration,      // 60 s
    pub finished_retention: Duration,    // 10 min
    pub cmd_channel_capacity: usize,     // 32
    pub event_channel_capacity: usize,   // 64
    pub dedup_window_per_player: usize,  // 32 command ids
    pub tombstone_retention: Duration,   // how long expired codes answer `Expired`
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            code_len: 8,
            waiting_expiry: Duration::from_secs(10 * 60),
            disconnect_grace: Duration::from_secs(60),
            finished_retention: Duration::from_secs(10 * 60),
            cmd_channel_capacity: 32,
            event_channel_capacity: 64,
            dedup_window_per_player: 32,
            tombstone_retention: Duration::from_secs(10 * 60),
        }
    }
}
```

## 4. Engine injection — the factory convention with B1

`ChessEngine` constructors are deliberately **not** on the trait
(dyn-compatibility — see `game/contract.rs`). B1's engine type provides its
own constructor (agree the name, expected: `new_game() -> Self`); your
registry takes a **factory** and calls it once per room:

```rust
pub type EngineFactory = std::sync::Arc<dyn Fn() -> Box<dyn ChessEngine> + Send + Sync>;

pub struct InMemoryRegistry {
    config: RoomConfig,
    new_engine: EngineFactory,   // ← inject here
    map: std::sync::Mutex<std::collections::HashMap<String, ActorHandle>>,
    tombstones: std::sync::Mutex<std::collections::VecDeque<(String, std::time::Instant)>>,
}

// Production wiring (main.rs, B3) — agree the exact constructor with B1:
let new_engine: EngineFactory = std::sync::Arc::new(|| Box::new(<B1sEngine>::new_game()));

// Test wiring (you):
let new_engine: EngineFactory = std::sync::Arc::new(|| Box::new(SharedEngine::new(...)));
```

Rules of the convention (write them into the B1+B2 interface agreement):

* The factory is called **once per room**, at `create_room`. It must return
  a fresh engine per call (never share one engine between rooms).
* **Rematch does NOT re-factory** — call `engine.reset_for_rematch()` on the
  same instance (that is what the method exists for; it clears board,
  history and draw state).
* The engine is owned exclusively by the actor task; nothing else ever sees
  it. No interior locking is required (the contract says so).

## 5. The actor — struct, loop, and a first handler

`ActorHandle` + its shared state (shapes; `ActorMessage` from
[02-architecture.md §3](02-architecture.md)):

```rust
#[derive(Clone)]
pub struct ActorHandle {
    cmd_tx: tokio::sync::mpsc::Sender<ActorMessage>,
    shared: std::sync::Arc<HandleShared>,
}

pub(crate) struct HandleShared {
    /// Closed flag + subscriber registry behind ONE std Mutex.
    pub(crate) closed: bool,
    pub(crate) subscribers: Vec<tokio::sync::mpsc::Sender<RoomEvent>>,
}
```

The actor:

```rust
pub(crate) struct RoomActor {
    state: RoomState,                       // 02-architecture.md §2
    config: RoomConfig,
    registry: RegistryRef,                  // to remove itself at shutdown
    shared: std::sync::Arc<HandleShared>,   // same Arc the handle holds
}

impl RoomActor {
    pub(crate) async fn run(mut self, mut rx: tokio::sync::mpsc::Receiver<ActorMessage>) {
        loop {
            match self.next_deadline() {
                Some(deadline) => {
                    tokio::select! {
                        msg = rx.recv() => match msg {
                            Some(msg) => self.handle(msg),        // sync! see below
                            None => break,                       // all handles dropped
                        },
                        _ = tokio::time::sleep_until(deadline) => self.handle_deadline(),
                    }
                }
                None => match rx.recv().await {
                    Some(msg) => self.handle(msg),
                    None => break,
                },
            }
        }
        self.shutdown().await;
    }
}
```

`handle` can be **entirely synchronous** (engine calls, revision bumps,
broadcasts via `try_send`, oneshot replies). That is a feature: an actor
with no awaits between messages cannot interleave state mutations. If you
do need an await inside a handler, keep the invariants local to that
handler and re-derive nothing after the await.

One handler, fully worked (`Move`) — the pattern for all of them:

```rust
fn handle_move(&mut self, player: &PlayerId, from: String, to: String,
               promotion: Option<Promotion>, expected_revision: u64,
               reply: oneshot::Sender<Result<CommandOutcome, RoomClosed>>) {
    // 1. Authorization BEFORE the engine (your brief): seated?
    let Some(seat_color) = self.state.seat_of(player) else {
        return self.reply_rejected(reply, ErrorCode::InvalidToken,
            "player is not in this room", None);
    };
    // 2. Freshness guard.
    if expected_revision != self.state.revision {
        return self.reply_rejected(reply, ErrorCode::StaleRevision,
            "stale revision", None);   // snapshot optional; Some heals faster
    }
    // 3. Room/engine state guards, then the engine call.
    let outcome = match (&mut self.state.engine, self.state.status) {
        (Some(engine), RoomStatus::Active) => {
            engine.apply_move(MoveRequest { from, to, promotion })
        }
        _ => return self.reply_rejected(reply, ErrorCode::NotYourTurn,
            "game has not started yet", None),
    };
    match outcome {
        Ok(snap) => {
            self.state.revision += 1;                     // single writer: the actor
            let wire = build_snapshot(&self.state, &snap);
            self.broadcast(RoomEvent::broadcast(RoomEventKind::Snapshot {
                revision: self.state.revision, snapshot: wire,
            }));
            self.send_reply(reply, Ok(CommandOutcome::Accepted {
                revision: self.state.revision,
            }));
        }
        Err(err) => {
            let (code, message) = map_engine_error(&err);   // 01-contract.md §6.2
            let snapshot = Some(build_snapshot(&self.state, &engine.snapshot()));
            let player_id = player.clone();
            self.reply_rejected(reply, code, &message, None);
            self.broadcast(RoomEvent::to(player_id.clone(), RoomEventKind::MoveRejected {
                player_id, error: code, message: message.into(),
                revision: self.state.revision, snapshot,
            }));
        }
    }
}
```

`broadcast` (never awaits — the slow-subscriber rule):

```rust
fn broadcast(&self, event: RoomEvent) {
    let mut subs = self.shared.subscribers.lock().unwrap(); // std guard, no await
    subs.retain(|tx| tx.try_send(event.clone()).is_ok());   // closed → prune;
    drop(subs);                                              // full → dropped event (§5 policy)
}
```

Deadline plumbing (lifecycle.rs):

```rust
impl RoomActor {
    pub(crate) fn next_deadline(&self) -> Option<tokio::time::Instant> {
        [self.state.waiting_deadline,
         self.state.grace_deadlines[0], self.state.grace_deadlines[1],
         self.state.finished_deadline]
            .into_iter().flatten().min()
    }

    fn handle_deadline(&mut self) {
        // whichever deadline(s) passed, in order — implement per
        // 02-architecture.md §9: grace expiry → forfeit via engine.resign(color)
        // + Finished broadcast; waiting/retention expiry → close & remove.
    }
}
```

## 6. `send` / `subscribe` — the trait impl sketches

```rust
impl RoomHandle for ActorHandle {
    async fn send(&self, command: RoomCommand) -> Result<CommandOutcome, RoomClosed> {
        // 1. Fast check under the std lock (guard dropped before any await).
        {
            let shared = self.shared.lock().unwrap();      // 02-architecture.md §6
            if shared.closed {
                return Err(RoomClosed);
            }
        }
        // 2. Oneshot reply + send. If the actor died without clean shutdown,
        //    send() errors OR the oneshot sender is dropped → RoomClosed.
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .send(ActorMessage::Command { command, reply: tx })
            .await
            .map_err(|_| RoomClosed)?;
        rx.await.map_err(|_| RoomClosed)   // actor replied by dropping
    }

    fn subscribe(&self) -> tokio::sync::mpsc::Receiver<RoomEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel::<RoomEvent>(/* config: */ 64);
        let mut shared = self.shared.lock().unwrap();
        if shared.closed {
            return rx;                     // a dead channel: recv() → None immediately
        }
        shared.subscribers.push(tx);
        rx
    }
}
```

**Clean shutdown — the drain pattern** (so no `send` future ever hangs):
when the actor decides to close (expiry, `Leave`, empty room):

```rust
async fn shutdown(mut self) {
    { let mut s = self.shared.lock().unwrap(); s.closed = true; s.subscribers.clear(); }
    // (a) every receiver gets None → B3 sees the room die on the event side;
    // (b) reply Err(RoomClosed) to everything still queued, keep draining
    //     until all sender clones are dropped, THEN the task ends:
    while let Some(msg) = self.rx.drain_to_closed().await { /* reply Err(RoomClosed) */ }
    self.registry.remove_room(&self.state.code);           // + tombstone if expired
}
```

i.e. after `closed = true`, the actor keeps consuming the command channel
and answering `Err(RoomClosed)` (or its internal equivalents for
Join/Resume: `NotFound`/`Expired`) until every `ActorHandle` clone is
dropped — only then does `recv()` yield `None` and the task truly exits.
No leaked task, no hung future, no lost reply. (Implement
`drain_to_closed` as a plain `while let Some(m) = rx.recv().await` loop.)

## 7. Pitfalls — read before you write code

1. **Never hold a `std::sync::MutexGuard` across an `.await`.** The guard is
   `!Send`, the future stops being `Send`, the trait impl stops compiling —
   and where it does compile, it deadlocks. Shape: clone out, drop guard,
   then await. `clippy::await_holding_lock` enforces this; keep it on.
2. **Never block the actor on a slow subscriber.** Broadcast with
   `try_send` only ([02-architecture.md §5](02-architecture.md)). Awaiting a
   full subscriber queue would freeze the room for both players.
3. **The first-wins join race needs no lock.** Both joins funnel through the
   actor; the second sees `Active` and gets `Full`. Never "pre-check" the
   seat in the registry and trust it across an await (TOCTOU).
4. **Single writer only.** The revision, seats, engine — everything in
   `RoomState` — is touched by the actor task alone. If you find yourself
   adding `Arc<Mutex<RoomState>>`, stop: that is the design you are replacing.
5. **RPITIT means plain `async fn` in impls.** Do not reach for
   `#[async_trait]` or `Pin<Box<dyn Future>>`; the frozen signatures already
   work on stable, exactly as `testkit.rs` implements them.
6. **Never log or broadcast `player_token`** (raw OR verifier). The master
   plan's logging rule: room code + command id are fine, tokens never.
7. **The actor must not panic on bad input.** `apply_move` is documented
   never to panic; your side (unknown player, weird state) answers
   `Rejected`. A panic kills the room and surfaces as `RoomClosed` — legal,
   but a bug. Keep `unwrap()` for "the map invariant is broken" only; for
   the two registry mutexes, `.lock().unwrap()` is the accepted policy here
   (poisoning means a panic already broke the process invariant).
8. **Registry removal is the actor's job at shutdown**, not the registry's —
   give the actor a `RegistryRef` (an `Arc` to the map + tombstones) at
   spawn time. Otherwise an expired room lingers in the map and `lookup`
   resurrects dead handles.
9. **Don't forget tombstones in the code-generation retry** — recycling a
   just-expired code resurrects stale joins (`Expired` vs `NotFound`
   confusion).
10. **`tokio::spawn` your actor from a sync context** is fine
    (`create_room` is async anyway — `tokio::spawn` works from any thread
    with a runtime handle).
11. **Test the shapes, not the fakes.** Your unit tests use YOUR registry +
    B1's `FakeEngine` as the engine seam only (see [04-testing.md](04-testing.md)).

## 8. Naming note

Your concrete types must not collide with the re-exported trait names
(`mod.rs` does `pub use contract::*`): `RoomRegistry`/`RoomHandle` are taken.
This doc set uses `InMemoryRegistry` (impl of `RoomRegistry`, associated
`Handle = ActorHandle`) and `ActorHandle` (impl of `RoomHandle`) — rename
freely, but keep the *traits* as the public surface; B3 and the testkit
never name your concrete types.
