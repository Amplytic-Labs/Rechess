# `src/services/` — Owner: **F3** (WebSocket Client and Frontend State)

> **Read `frontend/RULES.md` first** — it is binding. Then read this file
> completely before writing any code. You also co-own
> `docs/protocol-v1.md` (repo root) with B3 — it is the wire contract and
> the source of truth for every type you will touch.

This README is the **index of your doc set**. Each doc answers one part of
your brief; read them in order the first time:

| Doc | What it covers |
|---|---|
| `01-protocol-types.md` | Owning `src/types/protocol.ts`: what already exists, the binding change rule, how to extend types safely, how the fixtures and the wire-contract tests are used |
| `02-websocket-service.md` | The single WebSocket client (`src/services/websocket.ts`): env/URL config, send/receive paths, lifecycle state machine, reconnect + resume (incl. resume-on-navigation), heartbeat |
| `03-stores-and-hooks.md` | The zustand stores (`src/store/`) and the public hooks (`src/hooks/`) client components consume, including the frozen hook/action names |
| `04-security.md` | `player_token` handling in `localStorage` — the storage wrapper, the honest XSS tradeoff, deletion rules |
| `05-manual-testing.md` | The vitest unit-test suite AND the manual browser checklist (desktop + mobile viewport) |

---

## 1. Your mission

Deliver four things, in this order of dependency:

1. **The single typed WebSocket connection** to the backend — exactly ONE
   connection per app, owned by one module-level service
   (`src/services/websocket.ts`, doc 02) built on the browser's native
   `WebSocket` API. Screens never open sockets.
2. **The zustand stores** (`src/store/`): connection status, room state,
   and the authoritative game snapshot with revision guarding
   (doc 03).
3. **The public typed hooks** (`src/hooks/`) that F1 (lobby) and F4 (game
   screen) call to read state and send commands (doc 03). The names are
   frozen: `useConnection`, `useRoomState`, `useGameState`,
   `useRoomActions` — and the color field is `yourColor`.
4. **Token storage** (`src/security/`): the `player_token` lives ONLY in
   `localStorage`, one key per room, wrapped by a single module (doc 04).

Plus **ownership of the protocol types**: `src/types/protocol.ts` already
exists and is complete (doc 01). You own keeping it in sync with the
backend and `docs/protocol-v1.md`. The wire protocol itself is
**unchanged by the web migration** — the three-way match
(Rust ↔ TypeScript ↔ spec) was verified clean and stays described
as-is.

## 2. Ownership map

| Path | Yours | What goes there |
|---|---|---|
| `src/services/**` | ✔ | `websocket.ts` — the WS client (module-level singleton, not a hook); `config.ts` — the env/URL config module |
| `src/store/**` | ✔ | `connection-store.ts`, `room-store.ts`, `game-store.ts` |
| `src/types/protocol.ts` (+ `protocol.fixtures.ts` + `protocol.test.ts`) | ✔ | The TS mirror of the wire contract + its fixtures and vitest suite |
| `src/security/**` | ✔ | `player-token.ts` — the ONLY module touching `localStorage` for the token |
| `src/hooks/**` | ✔ | `useConnection`, `useRoomState`, `useGameState`, `useRoomActions` — the public API |

Everything else is someone else's: `src/screens/lobby/` (F1),
`src/components/chess/` (F2), `src/screens/game/` + `src/app/game/**`
(F4), other route files, `src/theme/` + `global.css`, configs
(RULES.md §0). Cross-folder changes go through a short PR the owner
approves.

## 3. Boundaries — you are not the UI

- **You never render.** No JSX, no components, no screens in your
  folders. Your code produces typed state and typed actions; F1/F4
  produce pixels.
- **Hooks give data + actions, nothing else.** A hook returns state
  slices (from your stores) and command senders (through your service).
  Screens subscribe with narrow selectors and call actions — they never
  see the socket, the envelope, or the token.
- **Client-side modules, client components only.** The service and the
  stores run in the browser, imported (directly or through the hooks)
  from client components (`"use client"`). Never add Next API routes or
  server actions that proxy game data — the Rust WebSocket server is the
  only data source (RULES.md §2).
- **F2 is pure props.** F2's `ChessBoard` receives the whole `snapshot`
  (`GameSnapshot`), `orientation`, and an `onMoveSubmit` callback from F4
  (the frozen contract: `src/components/chess/README.md` §4); it does not
  consume your hooks directly. Do not add imports of `@/hooks` or
  `@/store` into `src/components/chess/`.
- **The server is authoritative.** You never compute FEN, legal moves,
  or results client-side, and you never optimistically apply a move
  (RULES.md §4, doc 03).

**The single route and resume-on-navigation:** the waiting room, the
game and the result all live at the single route `/game/[roomCode]`
(F4). Your service owns the navigation-time session flow (doc 02 §7):
on landing at `/game/[roomCode]` — fresh load, reload, deep link — it
reads the room code from the URL, loads the stored token for that room
(doc 04), and resumes with `session.resume`; on leave/expiry the token
is deleted. Screens never trigger resume; they render the state it
produces.

## 4. Definition of done

- [ ] `protocol.ts` stays in sync: every change updates B3's
      `backend/src/protocol/` and `docs/protocol-v1.md` in the **same** PR.
- [ ] One WS service: connect / send / subscribe / disconnect; unique
      command ids (`crypto.randomUUID()`); generation guard against
      stale sockets.
- [ ] Reconnect: capped exponential backoff with jitter (0.5 s → 8 s),
      triggered by socket close, the `online` window event and
      `document.visibilitychange`; automatic `session.resume` from
      storage; `session.ready` snapshot applied atomically;
      `INVALID_TOKEN` clears credentials.
- [ ] Three stores with the atomic-revision rule and the pending-move
      pattern (doc 03).
- [ ] Hooks `useConnection` / `useRoomState` / `useGameState` /
      `useRoomActions` — the names F1's and F4's READMEs already
      reference.
- [ ] `player_token` only ever in `localStorage` via the security module
      (doc 04); never in logs, URLs, cookies, stores, or fixtures shipped
      to production.
- [ ] `npm run typecheck`, `npm run lint` and `npm run test` pass; the
      F3 unit suite (doc 05 §1) covers codec, stores and reconnect
      transitions; every scenario in `05-manual-testing.md` §3 has been
      walked through in a browser (desktop + mobile viewport) against
      the real backend at least once.

## 5. Quickstart

```bash
cd frontend
npm install            # once — deps are committed in package-lock.json
npm run typecheck      # tsc --noEmit — must be zero errors, always
npm run lint           # eslint — must be zero errors/warnings
npm run test           # vitest — the unit suite must be green
npm run dev            # http://localhost:3001 — the Rust backend owns :3000
npm run build          # production build — CI (B4) runs the same gates
```

Then:

1. Read `01-protocol-types.md` (short — the types already exist).
2. Read `02-websocket-service.md` and write `src/services/config.ts` +
   `src/services/websocket.ts`.
3. Read `03-stores-and-hooks.md` and write the stores, then the hooks.
4. Read `04-security.md` and wire the token flow.
5. Read `05-manual-testing.md` and run the unit suite + the manual
   scenarios against `cargo run` in `backend/` (server listens on
   `0.0.0.0:3000` by default; health check:
   `curl http://localhost:3000/health`).

Point the app at a non-default backend: set `NEXT_PUBLIC_API_URL` in
`frontend/.env.local` (doc 02 §2). The default is
`ws://localhost:3000/ws`, so same-machine local dev needs no
`.env.local` at all. `.env.local` is gitignored — never commit it.

## 6. Key coordination

- **B3** (backend transport): your mirror counterpart. Every protocol
  change is a joint PR. Read B3's `backend/src/network/01-…` and
  `02-…` docs — your client must match their lifecycle (heartbeat,
  auth state machine, limits).
- **B2** (rooms): owns token verification, reconnect grace, room
  retention — the server-side half of doc 04.
- **F1** (lobby): consumes `useRoomActions().createRoom/joinRoom` and
  `useRoomState()` (their `04-acceptance.md` §2 pins these names), then
  navigates to `/game/[roomCode]`.
- **F4** (game screen): consumes `useGameState()`, `useRoomState()`,
  `useRoomActions()`, `useConnection()` (their
  `01-screen-composition.md` §6 pins these) at the single route
  `/game/[roomCode]`.
- **B4** (CI/deployment): runs typecheck + lint + test + build; keep
  your connection states observable for their end-to-end tests.
