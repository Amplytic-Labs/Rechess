# 04 — Acceptance: definition of done, merge gate, handoffs

> Prereqs: docs 01–03. This file is your finish line: every box here maps to
> an item in your brief (`04_B3_WebSocket_Protocol.md`) or the Master Plan;
> every handoff is what the next owner integrates against.

---

## 1. Definition of done — checklist mapped to the brief

The brief's §8 deliverables and §9 acceptance criteria, item by item. Tick a
box only when the "satisfied by" column is real (code merged, test green,
doc updated) — not "should work".

### 1.1 Deliverables (brief §8)

- [ ] **Working Axum WebSocket endpoint and health endpoint.**
  Satisfied by: `network::router()` mounts `GET /ws` (doc 01 §1–§2);
  `GET /health` keeps returning
  `{"status":"ok","service":"rechess-backend"}`; after Phase 2
  `build_router(registry)` merges both and `cargo run` + `curl` prove it
  (README quickstart).
- [ ] **Versioned Serde message types and stable error codes.**
  Already exists and is frozen: `src/protocol/` (envelope, client_message,
  server_message, error_code, mod with `PROTOCOL_VERSION`) — 11 round-trip
  tests green today. Your job is to keep them unchanged (change rule,
  README §Boundaries) and enforce `PROTOCOL_VERSION` at the gate.
- [ ] **docs/protocol-v1.md with examples and state diagrams.**
  Exists as DRAFT; your job: the Day-1 freeze review with F3 — close the
  register of doc 02 §9 (D1–D6) in that review, update the doc, and publish
  the same-PR type updates if any decision changes a wire shape (D4 — a
  resolved example — is the cautionary tale).
- [ ] **Connection reader/writer and bounded outbound delivery.**
  Satisfied by: `connection::run` (doc 01 §4): split socket, writer task
  owning the sink, `OUTBOUND_QUEUE`-bounded mpsc, slow-client disconnect
  (doc 01 §5) with `session.resume` as the resync path.
- [ ] **Authorization, payload limits, rate limits and heartbeat handling.**
  Satisfied by: the auth state machine (doc 02 §5), the payload gate
  (doc 02 §4: nickname, room code, frame types, rate limits), size limits
  (doc 01 §6), heartbeat close + `RoomCommand::Disconnected` notification
  (doc 01 §7).
- [ ] **Two-client integration tests and protocol serialization tests.**
  Satisfied by: `tests/network_protocol.rs` (doc 03 §2 — green from day 0)
  and `tests/network_ws.rs` (doc 03 §3 — lands with your router +
  the `tokio-tungstenite` dev-dep PR).

### 1.2 Acceptance criteria (brief §9, verbatim)

> *"Two independent clients can create/join the same room, make a move,
> receive the same authoritative snapshot, and handle malformed/unauthorized/
> stale requests without crashing or leaking credentials. Rust and TypeScript
> protocol types match the frozen v1 document."*

| Clause | Proof |
|---|---|
| two clients create/join the same room | layer (b) test 1 (doc 03 §3) |
| make a move, receive the same snapshot | layer (b) test 1: identical `payload` + `revision` on both sockets |
| malformed handled without crashing | layer (a) gate tests + layer (b) malformed test (connection survives, answers `INVALID_PAYLOAD`) |
| unauthorized handled | layer (b) unbound-connection test + injected-identity assertion |
| stale handled | B2's `STALE_REVISION` via `expected_revision` — you forward it; layer (b) gains a case when D1 lands |
| no credential leaks | layer (a) token tests (4–5) + layer (b) broadcast assertions |
| Rust and TS types match the frozen doc | the resolved D4 case (doc 02 §9) + F3's typed fixtures in `protocol.fixtures.ts` matching the same examples |

### 1.3 Required-work items from the brief §2 not covered above

- [ ] WSS in production through TLS termination — your part is only the
  note in the B4 handoff (§3): the app itself is transport-agnostic; the
  reverse proxy terminates TLS and forwards the upgrade.
- [ ] "Route room.create and room.join to B2 registry operations; route
  room/game actions to the correct RoomHandle; never reach into B2 internal
  structs" — doc 02 §6–§7 (translation uses `RoomRegistry`/`RoomHandle`
  only; the compile check IS the guarantee: you cannot name a B2 internal).
- [ ] "Handle duplicate command IDs and stale revisions consistently with
  B2. Do not silently retry non-idempotent commands" — you never retry
  (each wire command maps to at most one `handle.send`); the dedup split is
  open decision D1 (doc 02 §5.3) — it is NOT done until D1 is closed in
  writing.
- [ ] Day-1 protocol freeze with F3 (brief §7 integration plan: Day 1
  freeze, Day 2 publish, Day 3 create/join + errors, Day 4 two-client move
  sync, Days 5–8 remaining commands + reconnect, Days 9–10 compatibility
  tests + fixes).

**Master Plan definition of done** (Table 19): *"a task is done only when
its code is merged, its relevant tests pass, its interface is documented,
and the next owner can integrate it without reverse-engineering
assumptions."* Sections §3–§4 below are the "next owner" part.

## 2. Merge gate

Every PR you open (B4 runs the same list in CI — `tests/README.md` §5):

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

cargo fmt --check          # formatting
cargo clippy -- -D warnings # lint policy (confirm with the team before -D)
cargo test                  # ALL tests green, not just yours:
                            #   protocol unit tests (4) + protocol_contract (7)
                            #   + testkit_demo (4) + network_protocol (14)
                            #   + network_ws (5) + your network:: units
cargo build --release       # what B4 ships
```

Plus the process gates:

- [ ] No file outside `src/network/` is edited unless the change is the
  agreed one-liners (`src/lib.rs` / `src/main.rs` in the Phase-2 PR;
  `Cargo.toml` in the dev-dep PR) or a protocol change-rule PR.
- [ ] Protocol changes: same PR updates `backend/src/protocol/`,
  `frontend/src/types/protocol.ts`, `docs/protocol-v1.md`, with F3 as
  reviewer (README §Boundaries).
- [ ] Contract changes (`rooms/contract.rs`): B2 (+ B1 if engine-adjacent)
  reviews, per the change rule in that file.
- [ ] No known-failing or `#[ignore]`d test without a tracked issue —
  B4's rule: never make main green by disabling tests silently.
- [ ] No secrets in logs (grep your own diff for `player_token`).

## 3. Handoff notes for F3 (the client contract)

Everything F3's client may rely on, in one place — mirror this in
`docs/protocol-v1.md` review comments where it belongs:

**Transport & connection lifecycle**

- One endpoint: `ws://<host>/ws` in dev, `wss://<host>/ws` in production.
  One message = one single-frame JSON **text** frame, UTF-8. Keep
  `player_token` ONLY in secure storage; reconnect with capped exponential
  backoff + jitter (0.5 s → 30 s) and `session.resume`.
- The server closes the socket: on `UNSUPPORTED_VERSION` (after the error),
  after ~90 s of silence (heartbeat, below), when the room ends, after an
  accepted `room.leave` (D3 — treat that close as SUCCESS, not a reconnect
  trigger), or when the client is too slow (bounded queue) — in every case
  the recovery is the same: `session.resume` returns the full snapshot.
- Heartbeat: the server sends a WebSocket protocol Ping every 30 s; browser
  and Expo WebSocket stacks answer automatically — the client app does
  nothing, no app-level heartbeat event exists in v1. Two unanswered pings
  (~60 s silence) ⇒ server closes.
- Limits enforced server-side: 64 KiB message/frame size (larger frames are
  dropped with a connection close — never send anything bigger), text
  frames only (binary ⇒ `INVALID_PAYLOAD`), ~10 commands/s (burst 20),
  1 create/join/resume per 10 s per connection (⇒ `RATE_LIMITED`).

**Envelope shapes (exact)**

- Client: `{ "v": 1, "id": "<uuid>", "type": "game.move", "payload": { … } }`.
  The `id` must be unique per client (UUID). It is currently echoed nowhere
  (docs §2.1) — do not display or key UI on it.
- Server: `{ "v": 1, "type": "game.state", "revision": 13, "payload": { … } }`
  — `revision` is ENVELOPE-level and appears ONLY on `game.state`,
  `game.move_rejected`, `game.finished`; other events have no `revision`
  key at all.
- `game.state`'s payload IS the snapshot (flattened — `fen`, `turn`, … sit
  directly in `payload`); apply it atomically and ignore snapshots older
  than the newest revision you have seen.
- `session.ready` carries the full snapshot and NO revision — the
  `expected_revision` for the next command comes from the `game.state`
  that follows (open decision D5, doc 02 §9); do not send state-changing
  commands until you have seen a revision.
- `room.leave` payload shape: **resolved (D4, in the base)** — both sides
  send `"payload": {}` (TS `EmptyPayload`, Rust `LeaveRoom {}` struct
  variant); `null` and absent are rejected. Pinned by the round-trip tests.

**The 12 stable error codes** (docs §6, verbatim — map each to a themed UI
message):

`ROOM_NOT_FOUND` · `ROOM_FULL` · `ROOM_EXPIRED` · `INVALID_TOKEN` ·
`NOT_YOUR_TURN` · `ILLEGAL_MOVE` · `STALE_REVISION` · `GAME_FINISHED` ·
`INVALID_PAYLOAD` · `UNSUPPORTED_VERSION` · `RATE_LIMITED` · `INTERNAL`

- Every error arrives either as `server.error` (generic rejections,
  always with a display-safe `message`) or `game.move_rejected` (failed
  move, mover only, envelope carries the current `revision`).
- `STALE_REVISION` ⇒ reconcile from the newest snapshot (never replay the
  move automatically; show the updated board, let the user re-move).
- `INVALID_TOKEN` on resume ⇒ home screen with a friendly message
  (docs §7.4).

**Privacy invariant**: `player_token` appears ONLY in the `room.created` /
`room.joined` private replies. If a client ever sees a token in any other
event, it is a server bug — report it to B3/B2, do not log the token.

**TS fixtures**: `frontend/src/types/protocol.ts` +
`protocol.fixtures.ts` + `protocol.test.ts` mirror the same examples as
`tests/protocol_contract.rs` — keep them passing against the same JSON.

## 4. Handoff notes for B4 (ops/deployment)

**Ports, env vars, health**

- `PORT` (default `3000`), `RECHESS_HOST` (default `0.0.0.0`) — already
  wired in `src/main.rs`; `RUST_LOG` controls verbosity
  (`RUST_LOG=rechess_backend=debug,tower_http=info`).
- `GET /health` → `200 {"status":"ok","service":"rechess-backend"}` — the
  deployment health check; the WS route is `GET /ws` (upgrade).
- Nothing else is exposed; no config files, no secrets in env beyond these.

**Reverse proxy / TLS (your WSS requirement)**

- Terminate TLS at the proxy; forward to the plain-WS backend. Enable the
  WebSocket upgrade (pass `Upgrade`/`Connection` headers), and do NOT time
  out idle connections below ~2 min — server heartbeats every 30 s keep
  healthy connections alive, but the proxy's own idle timeout must not be
  tighter than the heartbeat policy (doc 01 §7).
- Proxy-level rate limiting by IP is the layer for per-IP throttles (B3
  enforces per-connection limits only — doc 02 §4.4).
- In-memory rooms: a restart or redeploy ends every game — keep it in the
  runbook's known-limitations section (Master Plan §14). Single process
  only; multi-instance needs sticky routing + shared room state (out of
  MVP scope).

**Limits for the runbook** (all constants live in `network/limits.rs`):

| Limit | Value |
|---|---|
| Max message / frame size | 64 KiB each |
| Heartbeat | server Ping every 30 s; close after 2 missed (~60–90 s silence) |
| Outbound queue per connection | 64 messages, then disconnect (resume = resync) |
| Malformed frames | 3 per connection, then Close(1008) |
| Command rate | 10/s per connection, burst 20 |
| Create/join/resume | 1 per 10 s per connection |

**Logging**

- `tracing` structured logs carry: connection id (uuid), and once bound,
  room code, player id, command id. NEVER a `player_token` — grep CI logs
  once with a two-client test as a sanity check.

**Test infrastructure B3 owes you**

- `tests/network_protocol.rs` (14 tests) — green now, no deps needed.
- The `tokio-tungstenite` dev-dependency PR + `tests/network_ws.rs`
  (5 tests, doc 03 §3) land together with the router; the harness helpers
  (`spawn_server` on an ephemeral port, `connect`, `send_json`, `next_json`)
  are shared — build the 50-room benchmark on them rather than forking
  (`tests/README.md` §3–§4). After Phase 2, `build_router(registry)` is the
  one-call app for harnesses.
- Benchmark notes: per connection B3 costs one task, one 64-slot mpsc, one
  30 s interval timer, and one subscription receiver; at 100 connections
  that is negligible — the measured numbers are yours to record (Master
  Plan §13.3: measured, with hardware noted, never claimed).

## 5. Before you call it done — the open register

The doc-02 §9 register must show **D1 (dedup) and D2 (rejection channel)
closed with B2, D3/D5 closed with F3/B2 as marked — D4 is already resolved
in the base** before the module
is "done" in the Table-19 sense. Each closure is: decision, PR (if types
change), doc update, and a test that pins the agreed behavior. Until then
you have built a correct pipe with two unspecified joints — say so in the
PR description, don't let them pass silently.
