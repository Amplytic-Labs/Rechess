# 01 — The axum 0.8 WebSocket layer

> Prereqs: `src/network/README.md` (mission/boundaries) and
> `docs/protocol-v1.md` §1–§2. All type names below are copied from the real
> files — `axum 0.8.9` (with the `ws` feature), `tokio 1.54.1`,
> `futures-util 0.3.34` are already pinned in `backend/Cargo.toml`.

---

## 1. The route: `GET /ws` (not `/ws/:room_code`)

**Decision: mount a single endpoint, `GET /ws`.** Reasoning:

- `docs/protocol-v1.md` §1 is the frozen contract and it says: *"Single
  WebSocket endpoint per client: `ws://<host>/ws`"*. Choosing a parameterized
  path (`/ws/{room_code}`) would silently change the client-facing contract —
  and per the change rule (README §Boundaries) that requires a joint PR with
  F3 updating the doc and the TS client. Don't start your work with a
  protocol change.
- The path variant's real benefit is **resume routing** (the proxy/load
  balancer can route by room before reading a frame) and letting the server
  reject wrong-room connections at upgrade time. None of that is needed for
  the MVP: one backend process, all rooms in memory. The room code arrives
  in `room.join` / `session.resume` payloads anyway, and your auth gate
  (doc 02 §5) enforces "this connection may only act in its own room" at the
  command level, where it is exact.
- If multi-instance routing is ever needed, add the path variant then — it
  is an additive protocol change.

Two axum-0.8 specifics to remember while reading older tutorials:

- **Path parameters use `{param}` syntax, not `:param`** — if you ever do
  add `/ws/{room_code}`, write it with braces.
- **Text frames are `Message::Text(Utf8Bytes)`, not `Message::Text(String)`**
  — call `.as_str()` on the payload.

## 2. Where the route lives: `network::router()`, merged into `build_router()`

**Decision: your WS route lives in `src/network/routes.rs` as
`network::router::<R>(registry)`, and `build_router()` in `src/lib.rs` merges
it.** Do not paste the route directly into `build_router()`: keeping the
transport in `network/` preserves module ownership (B4's CI, other owners'
diffs stay clean) and — more importantly — lets your router stay **generic
over the registry**, so the same code serves `rooms::testkit::FakeRegistry`
in your tests and B2's real registry in production, with zero changes
(the pattern proven by `tests/testkit_demo.rs::generic_code_works_against_any_registry`).

### Phase 1 — today, while B2's registry does not exist yet

`build_router()` keeps its current signature (health only, `src/main.rs`
untouched); your tests build the WS router directly:

```rust
// src/network/routes.rs (skeleton — your implementation goes here)
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use crate::rooms::RoomRegistry;

pub fn router<R>(registry: R) -> Router
where
    R: RoomRegistry + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/ws", get(ws_upgrade::<R>))
        .with_state(registry)
}

async fn ws_upgrade<R>(State(registry): State<R>, ws: WebSocketUpgrade) -> Response
where
    R: RoomRegistry + Clone + Send + Sync + 'static,
{
    ws.max_message_size(crate::network::limits::MAX_MESSAGE_SIZE)
        .max_frame_size(crate::network::limits::MAX_FRAME_SIZE)
        .on_upgrade(move |socket| super::connection::run(socket, registry))
}
```

Why the bounds: axum's `State<R>` extractor requires `R: Clone + Send +
Sync + 'static`; `RoomRegistry` already implies `Send + Sync`, and its
associated `Handle: RoomHandle` (which itself implies `Clone + Send +
Sync`). Because the trait's methods return `impl Future + Send`, every
`.await` on them works fine inside `tokio::spawn`ed connection tasks.

### Phase 2 — the PR that brings B2's real registry

When B2's implementation lands, one coordinated PR (you + B4, who owns CI
wiring) changes `src/lib.rs` and the one-liner in `src/main.rs`:

```rust
// src/lib.rs (after Phase 2)
use crate::network;
use axum::{routing::get, Json, Router};

pub fn build_router<R>(registry: R) -> Router
where
    R: rooms::RoomRegistry + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health))          // keep B4's health gate
        .merge(network::router(registry))       // your WS route, state applied
}

// src/main.rs (after Phase 2) — replace `let app = build_router();`:
let app = build_router(registry); // registry = B2's real registry, built here
```

`network::router()` calls `.with_state(registry)` internally, so it returns
a stateless `Router` that merges cleanly with the health route. Until B2
lands, `build_router()` stays exactly as it is today — do not break
`src/main.rs` or B4's `curl /health` smoke gate while your dependency is
missing. `tests/network_ws.rs` (doc 03) binds `network::router(FakeRegistry…)`
on an ephemeral port and never needs Phase 2.

## 3. The upgrade

`WebSocketUpgrade` is an axum extractor; calling `on_upgrade` returns a
`Response` that axum converts into the `101 Switching Protocols` handshake.
Everything after that is your code:

- `on_upgrade`'s callback must be `FnOnce(WebSocket) -> Fut` where
  `Fut: Future<Output = ()> + Send + 'static` — move the registry clone in
  (it is `Clone`, and cloning shares the same rooms).
- Apply limits **on the upgrade builder**, before `on_upgrade` (§6 below).
- This is the moment the HTTP request ends and the socket begins: nothing
  about the HTTP request (headers, path, query) is needed later — the
  protocol has no cookie/header auth; identity is established by commands
  (doc 02 §5).

## 4. The per-connection task

One task per client, plus one writer task. Shape (skeleton with the real
names — fill in the `todo!()`s; the dispatch details are doc 02):

```rust
// src/network/connection.rs (skeleton)
use std::time::Duration;
use axum::{body::Bytes, extract::ws::{CloseFrame, Message, WebSocket}};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use crate::rooms::{RoomEvent, RoomRegistry};

pub async fn run<R>(socket: WebSocket, registry: R)
where
    R: RoomRegistry + Clone + Send + Sync + 'static,
{
    // ---- the two halves -------------------------------------------------
    let (sink, mut stream) = socket.split();   // needs StreamExt in scope
    let (out_tx, out_rx) = mpsc::channel::<Message>(super::limits::OUTBOUND_QUEUE);
    let writer = tokio::spawn(writer_task(sink, out_rx));

    // ---- per-connection state (doc 02 §8) -------------------------------
    // UNBOUND until a successful room.create / room.join / session.resume.
    let mut handle: Option<R::Handle> = None;              // B2's handle
    let mut events: Option<mpsc::Receiver<RoomEvent>> = None; // our subscription
    let mut player_id: Option<String> = None;             // bound seat
    let mut room_code: Option<String> = None;
    // NOTE: player_token is deliberately NOT stored. It is used exactly
    // once (registry.resume_room) and then dropped; B2 keeps the hash.
    let mut malformed_budget: u32 = 0; // close after limits::MAX_MALFORMED
    let mut missed_pongs: u32 = 0;

    // ---- heartbeat timer --------------------------------------------------
    // interval() fires its first tick IMMEDIATELY — offset by one period.
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + super::limits::HEARTBEAT_INTERVAL,
        super::limits::HEARTBEAT_INTERVAL,
    );

    let close_reason = loop {
        tokio::select! {
            // inbound frames from the client
            frame = stream.next() => match frame {
                None => break "socket closed",
                Some(Err(_)) => break "socket error",
                Some(Ok(Message::Text(text))) => {
                    missed_pongs = 0;                 // any frame = liveness
                    // doc 02 §2–§6: gate -> validate -> auth -> dispatch -> outcome.
                    // Returns Some(reason) when the connection must close.
                    match super::dispatcher::on_text(
                        &text, &registry, &mut handle, &mut events,
                        &mut player_id, &mut room_code, &out_tx,
                        &mut malformed_budget,
                    ).await {
                        Ok(()) => {}
                        Err(reason) => break reason,
                    }
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {
                    missed_pongs = 0; // protocol-level; auto-handled below
                }
                Some(Ok(Message::Binary(_))) => {
                    // v1 is text-only: INVALID_PAYLOAD, budget++ (doc 02 §4.3)
                }
                Some(Ok(Message::Close(_))) => break "client closed",
                Some(Ok(other)) => { let _ = other; } // raw Frame(_): ignore
            },

            // outbound events from B2's room actor
            event = async {
                match events.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await, // UNBOUND: never ready
                }
            } => match event {
                Some(event) => {
                    // doc 02 §7: RoomEvent -> ServerEvent -> envelope -> JSON
                    let envelope = super::dispatcher::map_room_event(&event, &player_id);
                    let json = serde_json::to_string(&envelope).expect("envelope serializes");
                    if out_tx.try_send(Message::Text(json.into())).is_err() {
                        break "outbound queue full"; // §5 backpressure
                    }
                }
                None => break "room ended", // actor dropped all senders
            },

            // heartbeat
            _ = heartbeat.tick() => {
                if missed_pongs >= super::limits::MAX_MISSED_PONGS {
                    break "heartbeat timeout";
                }
                missed_pongs += 1;
                let _ = out_tx.try_send(Message::Ping(Bytes::from_static(b"hb")));
            }
        }
    };

    // ---- teardown (§5) ---------------------------------------------------
    let _ = out_tx.try_send(Message::Close(Some(CloseFrame {
        code: 1000,                       // or 1008 for policy violations
        reason: close_reason.into(),
    })));
    drop(out_tx);                          // let the writer drain + finish
    if let (Some(handle), Some(player_id)) = (handle, player_id) {
        // Tell B2 the seat dropped, so its grace deadline starts.
        // Best-effort: RoomClosed just means the room already ended.
        let _ = handle
            .send(crate::rooms::RoomCommand::Disconnected { player_id })
            .await;
    }
    let _ = writer.await; // writer ends when out_rx is drained and dropped
}

/// The writer half: owns the socket sink, drains the bounded outbound queue.
async fn writer_task(
    mut sink: futures_util::stream::SplitSink<WebSocket, Message>,
    mut rx: mpsc::Receiver<Message>,
) {
    while let Some(message) = rx.recv().await {
        if sink.send(message).await.is_err() {
            break; // socket gone
        }
    }
    let _ = sink.close().await;
}
```

Why this shape (it is the axum official websocket example's shape, matching
your brief's "reader validates and routes; writer drains a bounded outbound
queue"):

- **The writer owns the sink.** Only one task ever writes, so frames are
  serialized correctly and no `Mutex<SplitSink>` is needed. The reader
  (`stream` in the select loop) and the dispatch logic live in the
  connection task.
- **One select loop, three arms.** Inbound frames, B2's outbound events, and
  the heartbeat tick all race in one place; whichever fires first is
  handled, then the loop runs again. The `std::future::pending()` trick
  keeps the outbound arm parked while the connection is still UNBOUND.
- **All per-connection state is local to `run`** — no shared struct, no
  locks, impossible for two connections to see each other's room state.

**Ordering rule (subtle, matters for tests):** when binding via
`room.create` / `room.join` / `session.resume`, call
`handle.subscribe()` **before** sending the private reply
(`room.created` / …). A reply received by the client proves the subscription
exists, so a test that emits a room event right after seeing `room.created`
cannot race. Doc 03's harness relies on exactly this ordering.

## 5. Backpressure, slow clients, graceful close

- **Bounded outbound queue.** `mpsc::channel::<Message>(OUTBOUND_QUEUE)`
  (recommendation: 64). When `out_tx.try_send` fails, the client is not
  reading fast enough — the master plan forbids unbounded memory growth, so
  **disconnect it** (`break "outbound queue full"`). That is not data loss:
  the client reconnects and `session.resume` returns the full snapshot
  (docs/protocol-v1.md §7) — resume IS the resync path. Never block the
  connection task on a slow client (`send().await` on a full queue stalls
  your heartbeat and inbound handling).
- **Graceful close on room end.** `events.recv()` returning `None` means
  every `RoomEvent` sender was dropped — the room actor ended (finished,
  expired, or every seat left). Send a `Close` frame (code 1000, reason
  e.g. `"room ended"`), then end the task. Same for `handle.send(...)`
  returning `Err(RoomClosed)`: stop sending, close cleanly. After
  `game.finished` the connection normally stays open (rematch votes flow)
  until B2's retention ends the actor — you do not decide room lifetime,
  you react.
- **Client-side close.** A `Close` frame or `None`/`Err` from the inbound
  stream ends the loop; always run the teardown block so B2 receives the
  `RoomCommand::Disconnected` notification and starts the reconnect grace
  window.
- **On teardown, notify exactly once.** The teardown sends
  `RoomCommand::Disconnected { player_id }` for bound connections only —
  never for UNBOUND ones (no seat exists). If the heartbeat or the outbound
  queue already killed the connection, teardown still runs once because it
  is the single `break` exit of the loop.

## 6. Message size limits

On the upgrade builder (before `on_upgrade`), set:

| Constant (in `limits.rs`) | Recommended value | Why |
|---|---|---|
| `MAX_MESSAGE_SIZE` | `64 * 1024` (64 KiB) | The largest legitimate client message is `room.create` with a 20-char nickname — well under 200 bytes. 64 KiB is ~300x headroom (future protocol fields, UTF-8 nicknames) while still killing any memory-abuse frame. The brief asks for "a small documented max"; tungstenite's default is 64 MiB, which is not acceptable. |
| `MAX_FRAME_SIZE` | `64 * 1024` (64 KiB) | docs/protocol-v1.md §1: every message is a **single-frame** JSON text message, so frame == message; setting both equal makes continuation-frame games impossible. |
| `MAX_WRITE_BUFFER_SIZE` / `WRITE_BUFFER_SIZE` | axum defaults (128 KiB) | Outbound messages are server-generated: a full `game.state` snapshot with 100 SAN moves is ~2–3 KiB, far below the buffer. Leave defaults; revisit only if B4's benchmarks show otherwise. |

Over-size input is rejected by tungstenite itself (the inbound read errors),
so your loop sees it as `Some(Err(_))` — log it with the connection id and
close. Write every chosen number into `limits.rs` with a comment; B4 copies
them into the deployment docs.

## 7. Heartbeat policy

- **Server-initiated Ping every 30 s** (`HEARTBEAT_INTERVAL`), using real
  WebSocket protocol Ping frames (`Message::Ping`), **not** an application
  event — the v1 protocol has no app-level heartbeat message, and browser /
  Expo WebSocket stacks answer protocol Pings automatically.
- **Drop after 2 missed Pongs** (`MAX_MISSED_PONGS = 2`): every inbound
  frame (text, pong — anything) resets `missed_pongs`; the tick arm closes
  when the counter has hit 2 with nothing in between. Arithmetic: ping at
  t=30 s, t=60 s, close at t=90 s if the client sent nothing for a full
  60 s+ window — two pings unanswered.
- Inbound `Ping` frames from the client: the tungstenite layer under axum
  queues the Pong reply automatically while the stream is being read (your
  writer half flushes it) — you do not construct Pong frames by hand. What
  you must implement is only the **server side**: send the Ping, count
  misses, close.
- Why 30 s / 2 misses: mobile networks drop idle sockets aggressively; a
  dead-but-open socket holds a room seat and the grace deadline hostage, so
  ~90 s of silence is the longest you want to wait, while 30 s of cadence
  keeps NAT mappings alive for healthy clients. (If B2's reconnect grace is
  60 s, a dead seat is detected well inside it — confirm the exact pairing
  with B2.)
- When the heartbeat closes a BOUND connection, the teardown block notifies
  B2 (`RoomCommand::Disconnected`) exactly like any other close — the grace
  window then belongs to B2.

## 8. Rate limits (transport level)

Detail lives in doc 02 §4.4; the transport summary: a per-connection token
bucket (`limits::RATE_COMMANDS_PER_SEC` — recommend 10/s burst 20: chess has
at most a few commands per second per player), a create/join throttle
(recommend 1 per 10 s per connection), violations answered with
`server.error RATE_LIMITED`, repeated violations close the socket. Per-IP
limits are a reverse-proxy concern — note them in B4's runbook rather than
building IP tracking into the MVP server.

## 9. Tracing

Use `tracing` with a per-connection id (uuid), and where bound also
`room_code` and `player_id` — never the `player_token`. Example:

```rust
tracing::info!(conn = %conn_id, "ws connected");
tracing::debug!(conn = %conn_id, room = ?room_code, player = ?player_id,
               cmd = %envelope.id, kind = %envelope.command, "command accepted");
```

`tower-http`'s `TraceLayer` (already pinned, `trace` feature) can wrap the
HTTP upgrade request — but never log message bodies: they contain
`player_token`. Keep `RUST_LOG` as the only verbosity knob (already wired in
`src/main.rs`).
