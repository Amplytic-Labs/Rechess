# Module `network` — Owner: **B3** (WebSocket Server and Protocol)

> **Read this file completely before writing any code.**
> This README is the index of the B3 onboarding doc set. The four numbered
> files below are your working guide — read them in order.

---

## Doc set index (read in this order)

| # | File | What it covers |
|---|---|---|
| — | `README.md` (this file) | Mission, boundaries, day-one quickstart |
| 1 | [01-axum-websocket-guide.md](01-axum-websocket-guide.md) | The axum 0.8 WebSocket layer: route, upgrade, connection task, limits, heartbeat |
| 2 | [02-protocol-implementation.md](02-protocol-implementation.md) | Per-command flows: parse gate, auth state machine, translation tables, outcome/event mapping, error mapping |
| 3 | [03-testing.md](03-testing.md) | Two test layers (full code): serialization tests (run today) + two-client WS harness (needs your router) + websocat smoke test |
| 4 | [04-acceptance.md](04-acceptance.md) | Definition of done, merge gate, handoff notes for F3 and B4 |

The single source of truth for the **wire format** is
[`docs/protocol-v1.md`](../../../docs/protocol-v1.md) — read it end to end
before doc 02. The exact Rust types implementing it already exist and are
frozen in [`src/protocol/`](../protocol/) — you consume them, you do not
rewrite them.

## Your mission

You own the **transport**: everything between a raw TCP WebSocket connection
and B2's room actors. Concretely:

1. **Axum router** — mount the WebSocket route (recommendation: `GET /ws`,
   see doc 01 §1) in `network::router()`, merged into `build_router()` in
   `src/lib.rs`; keep `GET /health` working.
2. **WebSocket upgrade** — `WebSocketUpgrade` extractor, message-size limits,
   `on_upgrade` into your per-connection task.
3. **Per-connection task** — split the socket into
   `(SplitSink, SplitStream)`, a bounded outbound channel, and one
   `tokio::select!` loop over inbound frames vs outbound room events vs
   heartbeat (doc 01 §4).
4. **Envelope parse/serialize** — `serde_json::from_str::<ClientEnvelope>` /
   `to_string(&ServerEnvelope)`, the version gate, malformed-input policy
   (doc 02 §3).
5. **Dispatch to B2's registry** — translate each `ClientCommand` into the
   right `RoomRegistry` call or `RoomCommand` (doc 02 §5–§6), route every
   `RoomEvent` back out as the right `ServerEvent` (doc 02 §7).
6. **Auth gate** — a connection is UNBOUND until `room.create` /
   `room.join` / `session.resume` succeeds; then it is BOUND to
   `(room_code, player_id)` and may only act in its own room (doc 02 §4).
7. **Limits** — max message size, allowed frame types, nickname validation,
   rate limits, bounded outbound queue (doc 01 §6, doc 02 §3).
8. **Heartbeat** — server Ping every 30 s, close after 2 missed Pongs,
   notify B2 with `RoomCommand::Disconnected` (doc 01 §7).

You deliver a **safe, typed, versioned pipe**. You are a courier, not a
judge: chess rules belong to B1, room state and lifecycle to B2. The room
actor is the authority on everything (revision, legality, seats, expiry);
you only move typed bytes in and out of it.

## Boundaries

### You consume (never re-implement)

- [`src/rooms/contract.rs`](../rooms/contract.rs) — the traits `RoomRegistry`
  (`create_room` / `join_room` / `resume_room` / `lookup`) and `RoomHandle`
  (`send → CommandOutcome`, `subscribe → mpsc::Receiver<RoomEvent>`), plus
  the `RoomCommand` / `RoomEvent` / error enums. Consume them **generically**
  (`R: RoomRegistry`), exactly like `tests/testkit_demo.rs` does — code
  written against the trait works unchanged against
  `rooms::testkit::FakeRegistry` (your tests, today) and B2's real registry
  (prod, when it lands).
- [`src/protocol/`](../protocol/) — the ENTIRE wire layer you own at runtime:
  `ClientEnvelope` / `ServerEnvelope` / `ClientCommand` / `ServerEvent` /
  `ErrorCode` / `PROTOCOL_VERSION`, all already written, tested and frozen.
- [`src/rooms/testkit.rs`](../rooms/testkit.rs) — `FakeRegistry` /
  `FakeHandle` for your tests before B2's implementation lands.
- `game` domain types appear in B2's contract (`RoomCommand::Move` uses
  `game::Promotion`, `RoomEventKind::Finished` carries a `game::GameResult`)
  — you convert between those and the wire types (doc 02 §7.3). You never
  call B1's engine directly.

### You do NOT touch

- `src/game/` (B1) and `src/rooms/` (B2) — no edits, no reaching into room
  internals. `RoomCommand`s are the only thing you put into B2's actor.
- Chess rules, room lifecycle, expiry, rematch semantics.
- `src/main.rs` config (RECHESS_HOST/PORT) beyond the one-line registry
  wiring when B2 lands (doc 01 §2, coordinated with B4).
- The frontend — except through the protocol change rule below.

### The protocol change rule (co-owned with F3)

`src/protocol/` types are **frozen**. They are co-owned by B3 (Rust) and F3
(TypeScript mirror in `frontend/src/types/protocol.ts`). Any change to a
type, field name, or wire value must update **in the same reviewed PR**:

1. `backend/src/protocol/` (the Rust types),
2. `frontend/src/types/protocol.ts` (F3's mirror — F3 reviews),
3. `docs/protocol-v1.md` (the spec).

Never rename a field silently — the other side compiles and then breaks at
runtime. If a change touches `rooms::contract` as well, B2 is in the PR too
(per the change rule in `contract.rs`).

## Day-one quickstart

**1. Read, in this order** (all files are short — a focused afternoon):

1. `docs/protocol-v1.md` — the wire spec, end to end.
2. This doc set: `01` → `02` → `03` → `04`.
3. `backend/src/protocol/mod.rs` + the four files next to it — the types you
   will serialize all day.
4. `backend/src/rooms/contract.rs` (traits you consume) and
   `backend/src/rooms/testkit.rs` (fakes you test against).
5. `backend/tests/protocol_contract.rs` and `backend/tests/testkit_demo.rs`
   — the two green test files your tests are modeled on.

**2. Toolchain + build + tests** (Rust is not on the default PATH here):

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo build && cargo test
```

Expect GREEN: 15 tests across the `protocol` unit tests (4),
`tests/protocol_contract.rs` (7) and `tests/testkit_demo.rs` (4). If this is
not green before you start, stop and tell B4.

**3. See the server run** (two terminals):

```bash
# terminal 1 — start the server
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo run

# terminal 2 — the deployment health endpoint (B4's gate)
curl http://127.0.0.1:3000/health
# → {"status":"ok","service":"rechess-backend"}
```

`src/main.rs` binds `RECHESS_HOST` (default `0.0.0.0`) and `PORT` (default
`3000`) from the environment and serves `build_router()` from `src/lib.rs`.
Today that router has only `/health` — your WS route is what goes in next.

**4. Then start implementing**: doc 01 gives the router/upgrade/connection
skeleton; doc 02 the exact dispatch logic; doc 03 the tests you can write
TODAY (`tests/network_protocol.rs` compiles against the frozen contracts
before any of your code exists — start there).

## Where your code goes

```
backend/src/network/
├── README.md        ← this doc set (keep the index current)
├── mod.rs           ← module re-exports + invariants note
├── routes.rs        ← router() + ws upgrade handler + limits  (doc 01)
├── connection.rs    ← per-connection task: select loop, writer task,
│                      heartbeat, per-connection state           (doc 01 §4–§7)
├── dispatcher.rs    ← parse gate, validation, translation, mapping (doc 02)
└── limits.rs        ← every tunable constant in one place        (doc 01 §6)
```

(The brief's suggested layout had `websocket.rs` / `read_loop.rs` /
`write_loop.rs` as separate files; this set consolidates them into
`connection.rs` — the reader half is the select loop's inbound arm, the
writer half is `writer_task`. Same responsibilities, fewer files. Adjust if
you prefer; keep `limits.rs` constants in one place either way.)

## Key coordination (from the brief, still true)

- **F3** (frontend) — from Day 1: they implement the same protocol in
  TypeScript; every protocol change is a joint PR (rule above). They are
  your most important counterpart.
- **B2** — daily: the `RoomRegistry`/`RoomHandle` surface, the outbound
  event model, and the **two open decisions** flagged in doc 02 §5.3
  (command-id dedup location) and §7.4 (single-channel rejection
  agreement). Settle both on Day 1.
- **B4** — they own `backend/tests/` and CI; coordinate file placement of
  `tests/network_protocol.rs` / `tests/network_ws.rs` and the
  `tokio-tungstenite` dev-dependency PR (doc 03 §3).
