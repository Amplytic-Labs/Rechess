# 05 — Manual testing (there is NO test runner — by design)

> **Accepted deviation:** the original brief's "unit tests" deliverable is
> deliberately replaced by this manual + typecheck strategy (team decision
> — jest-expo is broken upstream, see RULES.md §5).

> RULES.md §5: jest-expo is broken upstream (SDK 58 + Jest 30); the team
> decided the frontend is verified by `npm run typecheck` + `npm run lint`
> + manual checklists. **Never write jest tests.** The wire contract
> itself is proven by typecheck + the backend's `cargo test`
> (`backend/tests/protocol_contract.rs`).

## 1. The logic gate: typecheck

TypeScript strict + the typed fixtures ARE the contract test (doc 01 §6):

```bash
cd frontend
npm run typecheck   # fixtures must still satisfy protocol.ts
npm run lint        # zero errors/warnings
```

Run after every change, before every push. What it proves: every
envelope/command/payload you build satisfies the frozen types; what it
does not prove: runtime behavior — that is §2–§4.

## 2. Testing against the real backend (B4's runbook)

```bash
cd backend
cargo run                       # serves /health on 0.0.0.0:3000 by default
curl http://localhost:3000/health   # {"service":"rechess-backend","status":"ok"}
```

Frontend: `frontend/.env` with `EXPO_PUBLIC_API_URL=http://<LAN-IP>:3000`
(doc 02 §2), `npm start`, load the app on a device/simulator (phone on
the same LAN). For quick wire checks without the app, `websocat` works:

```bash
websocat ws://<LAN-IP>:3000/ws
# then paste: {"v":1,"id":"cmd-1","type":"room.create","payload":{"nickname":"Tester"}}
# expected reply: room.created with room_code, player_id, player_token
```

## 3. The scripted scenarios (run all before declaring done)

For each: steps, and the store state you expect after each step. Watch
state via a temporary dev-only log or React DevTools; verify the UI (F1's
waiting room, F4's game screen) reflects it.

**S1 — create → join happy path.** Device A: create room with nickname →
expect `connectionStore.status === "connected"`, `roomStore.roomCode`
set (8 chars), `yourColor === "white"`, `status === "waiting"`, one player
in `players`; SecureStore holds token + room code. Device B: join with
the code → `yourColor === "black"`, two players, `status === "active"` (or
the room-state event your flow expects), both devices show both players.
Promises: A's `createRoom` resolved `{ roomCode, yourColor }`.

**S2 — move + snapshot update.** A (white) plays e2e4 → on A:
`gameStore.pending` set while waiting; on `game.state`
(revision 1): `pending === null`, `snapshot.fen` updated, `fen` turn
`"black"`, `move_history` contains `"e4"`, `revision === 1`. Device B
sees the same snapshot. Then B plays a legal reply; snapshots sync again.

**S3 — move rejected flow.** B (black) tries to move on white's turn (or
an illegal square): `game.move_rejected` → `pending === null`,
`lastError` shows the safe message, board unchanged; then B moves
legally → works (server state not corrupted).

**S4 — heartbeat / idle.** Leave both apps idle > 90 s. Expected: server
pings every 30 s, auto-pong keeps both connections alive (status stays
`"connected"`). If a device gets dropped, the client goes
`reconnecting` → resumes → `session.ready` snapshot applied. Record
whether RN auto-pongs on each platform you can test (doc 02 §9).

**S5 — disconnect/reconnect with resume.** With a game in progress:
airplane-mode device A ~10 s, back online → status
`reconnecting` → after backoff, `session.ready` with the current
snapshot → `connected`, same room, board identical to B's, pending
cleared. Within B2's grace (~60 s) the seat survives. Bonus: background
the app > grace → resume should fail with `INVALID_TOKEN`/`ROOM_NOT_FOUND`.

**S6 — stale-token path.** Stop the backend (rooms are in-memory, they
die), restart it, foreground the app → resume fails → credentials
cleared, friendly message, back at home; no reconnect loop.

**S7 — server-down backoff behavior.** Stop the backend while the app is
connected. Watch `connectionStore.attempts` climb 1,2,3… with delays
1s→2s→4s→…→capped 30s (add jitter; no machine-gun). Restart the backend
→ client recovers to `connected` and resumes. While down: UI shows the
banner, board interactions disabled, no crash, no rate-limit errors.

**S8 — leave semantics.** Player leaves → server closes the socket
(Close 1000 "left room", B3 doc 02 D3) → client treats it as success:
no reconnect loop, credentials cleared. A new room needs a NEW
connection (verify create-after-leave works). Also verify the opponent
gets `player.presence` with `connected: false`.

## 4. Dev-only mock strategy (before the backend is up / for F1/F4)

F1/F4 need to develop before your real service works. Build a small
**fake service that implements the same hooks interface**, driven by the
fixtures (`src/types/protocol.fixtures.ts`), and keep it quarantined:

- Lives as one dev-only module inside your ownership (e.g.
  `src/services/mock-websocket.ts`) or as a hook-level fake
  (`useMockRoomActions` etc. implementing the frozen names from doc 03).
- **Behind a flag**, e.g. `EXPO_PUBLIC_USE_MOCK_WS=1` read via
  `process.env` — the real service stays the default. No fixture data in
  the production bundle: gate imports so dead-code elimination can drop
  the mock when the flag is absent, and delete the module before release
  if it isn't needed.
- Clearly marked at the top of the file: dev-only, never ships, no real
  tokens (fixtures use `"secret-token"`).
- Behavior: emit `sampleRoomCreatedEvent` on `createRoom`,
  `sampleSnapshot` on the first `game.state`, advance a mock revision on
  each `submitMove`, emit `sampleFinishedSnapshot` on resign/rematch,
  simulate disconnect after a timeout to let screens test banners.
- Screens import ONLY the frozen hooks, so switching mock ↔ real is one
  flag, zero screen edits. Master plan §16: replace mocks with the real
  server by Day 4 — the mock does not linger.

## 5. Recording results

Note in each PR description which scenarios you ran (S1–S8) and on which
platform(s): device/simulator, OS, app version. If a scenario can't be
run (e.g. no second device), say so — an untested claim is worse than a
listed gap.
