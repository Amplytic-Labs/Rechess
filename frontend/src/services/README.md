# `src/services/` — Owner: **F3** (WebSocket Client and Frontend State)

> **Read `frontend/RULES.md` first** — it is binding. Then read this file
> completely before writing any code. You also co-own
> `docs/protocol-v1.md` (repo root) with B3 — it is the wire contract and
> the source of truth for every type you will touch.

This README is the **index of your doc set**. Each doc answers one part of
your brief; read them in order the first time:

| Doc | What it covers |
|---|---|
| `01-protocol-types.md` | Owning `src/types/protocol.ts`: what already exists, the binding change rule, how to extend types safely, how the fixtures file is used |
| `02-websocket-service.md` | The single WebSocket client (`src/services/websocket.ts`): URL derivation, send/receive paths, lifecycle state machine, reconnect + resume, heartbeat |
| `03-stores-and-hooks.md` | The zustand stores (`src/store/`) and the public hooks (`src/hooks/`) screens consume, including the frozen hook/action names |
| `04-security.md` | `player_token` handling with `expo-secure-store` |
| `05-manual-testing.md` | How to verify your work with NO test runner: typecheck, real-backend scenarios, and a dev-only mock strategy |

---

## 1. Your mission

Deliver four things, in this order of dependency:

1. **The single typed WebSocket connection** to the backend — exactly ONE
   connection per app, owned by one module-level service
   (`src/services/websocket.ts`, doc 02). Screens never open sockets.
2. **The zustand stores** (`src/store/`): connection status, room state,
   and the authoritative game snapshot with revision guarding
   (doc 03).
3. **The public typed hooks** (`src/hooks/`) that F1 (lobby) and F4 (game
   screen) call to read state and send commands (doc 03).
4. **Token security** (`src/security/`): the `player_token` lives ONLY in
   `expo-secure-store` (doc 04).

Plus **ownership of the protocol types**: `src/types/protocol.ts` already
exists and is complete (doc 01). You own keeping it in sync with the
backend and `docs/protocol-v1.md`.

## 2. Ownership map

| Path | Yours | What goes there |
|---|---|---|
| `src/services/**` | ✔ | `websocket.ts` — the WS client (module-level singleton, not a hook) |
| `src/store/**` | ✔ | `connection-store.ts`, `room-store.ts`, `game-store.ts` |
| `src/types/protocol.ts` (+ `protocol.fixtures.ts`) | ✔ | The TS mirror of the wire contract |
| `src/security/**` | ✔ | SecureStore wrapper for `player_token` |
| `src/hooks/**` | ✔ | `useConnection`, `useRoomState`, `useGameState`, `useRoomActions` — the public API |

> Note: `RULES.md` §0 lists `src/services/`, `src/store/`, `src/types/`,
> `src/security/` under F3; the `frontend/README.md` module map also lists
> `src/hooks/` under F3. If you want this formalized in `RULES.md` §0,
> propose a tiny doc PR — do not edit `RULES.md` yourself.

Everything else is someone else's: `src/screens/lobby/` (F1),
`src/components/chess/` (F2), `src/screens/game/` (F4), routes, theme,
configs. Cross-folder changes go through a short PR the owner approves.

## 3. Boundaries — you are not the UI

- **You never render.** No JSX, no components, no screens in your folders.
  Your code produces typed state and typed actions; F1/F4 produce pixels.
- **Hooks expose data + actions, nothing else.** A hook returns state
  slices (from your stores) and command senders (through your service).
  Screens subscribe with narrow selectors and call actions — they never
  see the socket, the envelope, or the token.
- **F2 is pure props.** F2's `ChessBoard` receives the whole `snapshot`
  (`GameSnapshot`), `orientation`, and an `onMoveSubmit` callback from F4
  (the frozen contract: `src/components/chess/README.md` §4); it does not
  consume your hooks directly. Do not add imports of `@/hooks` or `@/store`
  into `src/components/chess/`.
- **The server is authoritative.** You never compute FEN, legal moves,
  or results client-side, and you never optimistically apply a move
  (RULES.md §4, doc 03).

## 4. Definition of done

- [ ] `protocol.ts` stays in sync: every change updates B3's
      `backend/src/protocol/` and `docs/protocol-v1.md` in the **same** PR.
- [ ] One WS service: connect / send / subscribe / disconnect; unique
      command ids (`expo-crypto` `randomUUID()`); generation guard against
      stale sockets.
- [ ] Reconnect: exponential backoff with jitter (capped), automatic
      `session.resume` from SecureStore, `session.ready` snapshot applied
      atomically, `INVALID_TOKEN` clears credentials.
- [ ] Three stores with the atomic-revision rule and the pending-move
      pattern (doc 03).
- [ ] Hooks `useConnection` / `useRoomState` / `useGameState` /
      `useRoomActions` — the names F1/F4's READMEs already reference.
- [ ] `player_token` only ever in SecureStore (doc 04); never in logs,
      route params, stores, or fixtures shipped to production.
- [ ] `npm run typecheck` and `npm run lint` pass, and every scenario in
      `05-manual-testing.md` has been walked through on a device/simulator
      with the real backend at least once.

## 5. Quickstart

```bash
cd frontend
npm install            # once — deps are committed in package-lock.json
npm run typecheck      # tsc --noEmit — must be zero errors, always
npm run lint           # eslint — must be zero errors/warnings
```

Then:

1. Read `01-protocol-types.md` (short — the types already exist).
2. Read `02-websocket-service.md` and write `src/services/websocket.ts`.
3. Read `03-stores-and-hooks.md` and write the stores, then the hooks.
4. Read `04-security.md` and wire the token flow.
5. Read `05-manual-testing.md` and run the scenarios against
   `cargo run` in `backend/` (server listens on `0.0.0.0:3000` by
   default; health check: `curl http://localhost:3000/health`).

Point the app at the backend: set `EXPO_PUBLIC_API_URL` in `.env`
(doc 02 §2) — e.g. `http://192.168.1.20:3000` (your machine's LAN IP).

## 6. Key coordination

- **B3** (backend transport): your mirror counterpart. Every protocol
  change is a joint PR. Read B3's `backend/src/network/01-…` and
  `02-…` docs — your client must match their lifecycle (heartbeat,
  auth state machine, limits).
- **B2** (rooms): owns token verification, reconnect grace, room
  retention — the server-side half of doc 04.
- **F1** (lobby): consumes `useRoomActions().createRoom/joinRoom` and
  `useRoomState()` (their `04-acceptance.md` §2 pins these names).
- **F4** (game screen): consumes `useGameState()`, `useRoomState()`,
  `useRoomActions()`, `useConnection()` (their `01-screen-composition.md` §6
  pins these).
- **B4** (CI/deployment): runs the same typecheck/lint gates; keep your
  connection states observable for their end-to-end tests.
