# Module `network` — Owner: **B3** (WebSocket Server and Protocol)

> **Read this file completely before writing any code.**
> You also co-own `docs/protocol-v1.md` with F3 (frontend) — read that file
> first, it is the single source of truth for the wire format.

---

## 1. Your mission

Deliver the transport layer: an Axum app with a health endpoint and a
WebSocket endpoint, split each connection into a **reader task** (validate →
route) and a **writer task** (drain a bounded outbound queue), and translate
between the JSON wire protocol (v1) and B2's room API.

You are **not** implementing chess rules (B1) or room state/lifecycle (B2).
You route valid commands into B2's public actor interface; you never reach
into B2's internal structs.

## 2. What you own / what you don't touch

| You own (`src/network/`) | You do NOT touch |
|---|---|
| Axum app, routes, WebSocket upgrade | `src/game/` (B1) |
| Wire-format structs — put them in `src/protocol/` (see §5) | `src/rooms/` (B2) — call its public API only |
| Message validation, limits, rate limiting | The room actor's internals |
| Reader/writer connection tasks | The frontend protocol file — changes go through F3 + one PR (§5) |
| Auth gate: token/seat association checks | |
| Error payload mapping | |
| `docs/protocol-v1.md` **jointly with F3** | |
| Wire serialization tests | |

## 3. How your module fits the system

```
client WS ──▶ reader task ──validate──▶ B2 RoomHandle.send(command)
client WS ◀── writer task ◀── B2 outbound events (targeted + broadcast)
```

- B2's room actor serializes game changes; your tasks only move bytes.
- B3 and F3 (frontend) freeze protocol v1 **on Day 1** — envelope, event
  names, field names, revision semantics, reconnect flow — and record it in
  `docs/protocol-v1.md`.

## 4. Facts you need on Day 1 — axum 0.8 (already pinned in `Cargo.toml`)

Axum **0.8.9** is pinned with the `ws` feature. Key differences from older
axum tutorials you will find online (verified Nov 2026):

- **Path parameters use `{param}` syntax**, not `:param`:
  `.route("/room/{code}", get(...))`.
- Text frames arrive as `Message::Text(Utf8Bytes)` (not `String`) in recent
  versions — call `.as_str()` on it.
- Split-socket pattern (reader and writer as two tasks over one mpsc):

```rust
use axum::{extract::ws::{Message, WebSocket, WebSocketUpgrade}, response::Response};

async fn ws_route(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    // Split into sender/receiver halves, give the sender half to a writer
    // task that drains a BOUNDED mpsc of outbound events; read from the
    // receiver half in this task; validate; route into B2's RoomHandle.
    // On disconnect: tell B2 (so it can start the grace deadline) and abort
    // the writer task.
    todo!("your implementation — see the axum official websocket example")
}
```

Read the official axum WebSocket example (axum repo, `examples/websockets/`)
before writing your own — the reader/writer split there is exactly the shape
your brief requires. Use `tokio::select!` to interleave reading with writer
lifecycle events; use `futures_util::StreamExt/SinkExt` (pinned in
`Cargo.toml`) for the socket halves.

## 5. The wire-format structs: `src/protocol/` (yours)

The JSON contract is specified in **`docs/protocol-v1.md`** — it is the
source of truth. You implement it as versioned Serde types in a sibling
module `src/protocol/` (create it; it is part of your ownership):

```
src/protocol/        ← create this (owner: B3)
├── mod.rs           // re-exports, PROTOCOL_VERSION, shared enums (colors, statuses)
├── envelope.rs      // ClientEnvelope / ServerEnvelope  { "v", "id", "type", "payload" }
├── client_message.rs // ClientCommand enum (room.create/join, game.move, ...)
├── server_message.rs // ServerEvent enum (room.created, game.state, ...)
└── error_code.rs    // ErrorCode enum (ROOM_NOT_FOUND, ILLEGAL_MOVE, ...)
```

Suggested serde shape for the envelope (adjacently-tagged command flattened
into the envelope — verify it round-trips with a unit test; if the flatten
does not support this, hand-implement `Serialize`/`Deserialize` for the two
envelope structs and keep the same JSON):

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ClientEnvelope {
    pub v: u32,
    pub id: String,                        // unique command id (uuid) from the client
    #[serde(flatten)]
    pub command: ClientCommand,            // tag = "type", content = "payload"
}
```

**Change rule:** any protocol change updates, in the *same* PR: (1) your
Rust types, (2) `frontend/src/types/protocol.ts` (F3's file — F3 is
co-reviewer), (3) `docs/protocol-v1.md`. Never rename a field silently —
the other side compiles and breaks at runtime otherwise.

## 6. Required behaviour (your brief, expanded)

- Endpoints: keep `GET /health` from `src/main.rs`; add the WebSocket
  endpoint (suggest `/ws`). Route registration stays in `src/main.rs` —
  coordinate the one-line change with B4 (they own `main.rs` wiring for CI).
- Reject unknown protocol versions (`v != 1`) and malformed payloads with a
  structured `server.error` event — never drop the connection silently for
  recoverable errors.
- Route `room.create` / `room.join` to B2's registry; route in-room commands
  to the correct `RoomHandle` by normalized room code.
- **Auth gate:** player-specific commands are authenticated by the private
  room token / seat association established via B2. A room code alone is
  **not** authorization. Never trust client-claimed identity.
- Enforce: max message size (commands are tiny — set `max_message_size` low,
  e.g. a few KiB, on the upgrade builder), allowed message types, nickname
  length (1–20 chars, no control chars), rate limits, bounded outbound queue.
  Document every limit you choose in `docs/protocol-v1.md`.
- Heartbeat/idle detection: WebSocket Ping/Pong from the server; close dead
  connections and inform B2 so the grace deadline starts.
- Duplicate command IDs / stale revisions: pass through to B2 (they own
  dedup) and translate their typed errors into wire error codes.
- Private credentials (`player_token`) are returned ONLY in the
  `room.created`/`room.joined` response to the initiating connection.
  Broadcasts carry public data only; logs never contain tokens.
- Error payload: `{ "type": "server.error", "payload": { "error": "<CODE>", "message": "<safe text>" } }`
  — safe client-facing message, details in `tracing` logs (no secrets).
- Deployed environments terminate TLS at a reverse proxy and speak **WSS**
  from the client — document the proxy requirements with B4.

## 7. How to test your module independently

```bash
cd backend
cargo test network::    # unit tests live in your module
cargo test protocol::    # serialization tests (same crate, your module)
```

Required test coverage (your brief):

1. Envelope round-trip tests for every command and event (serialize →
   parse → equal). Include the exact examples from `docs/protocol-v1.md` as
   fixtures — they are the frozen contract.
2. Malformed payloads: invalid UTF-8 JSON, missing fields, wrong types,
   unknown `type`, unknown `v` → each yields a structured error, no panic.
3. **Two-client integration test** (write it with B4 — they own the harness
   in `tests/`): connect two clients, create, join, move, both receive the
   same revision; rejected move returns `game.move_rejected`; private
   credentials never appear in broadcasts.

For integration tests without real network, spawn the router with
`Router::oneshot` / a real `TcpListener` on an ephemeral port
(`tokio::net::TcpListener::bind("127.0.0.1:0")`) — B4's `tests/README.md`
describes the setup.

## 8. Deliverables (definition of done)

- [ ] Axum WebSocket endpoint + health endpoint, reader/writer split, bounded outbound delivery
- [ ] `src/protocol/` Serde types matching `docs/protocol-v1.md`, versioned
- [ ] Auth gate, payload limits, rate limits, heartbeat handling
- [ ] `docs/protocol-v1.md` frozen with F3 on Day 1, with examples and state diagrams
- [ ] Serialization tests + two-client integration test (with B4)
- [ ] This folder's README updated with your final route/module layout

## 9. Key coordination

- **F3** (frontend) from Day 1: freeze the protocol together — you own the
  Rust side, they own the TS side, the doc is shared.
- **B2** daily: registry/handle API and the outbound event model.
- **B4**: they build the integration harness; ask them for the two-client
  test scaffold early instead of writing your own harness in duplicate.
