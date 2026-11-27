# `src/services/` — Owner: **F3** (WebSocket Client and Frontend State)

> **Read `frontend/RULES.md` first** — it is binding. Then read this file
> completely. You also co-own `docs/protocol-v1.md` (at the repo root) with
> B3 — review it before anything else; it is the wire contract.

---

## 1. Your mission

Deliver the **single typed WebSocket connection** to the backend, the
application stores (zustand), the protocol TypeScript types, and secure
token storage. Every other frontend module consumes your typed API — you
are the bridge between the backend protocol and all UI contributors.

You are **not** building screens (F1/F4) or board components (F2).

## 2. What you own / what you don't touch

| You own | You do NOT touch |
|---|---|
| `src/services/**` (WebSocket service) | `src/screens/**` (F1/F4) |
| `src/store/**` (zustand stores) | `src/components/chess/` (F2) |
| `src/types/protocol.ts` (TS mirror of the protocol; B3 owns the Rust side) | `src/app/` routes (F1/F4 own their route files) |
| `src/security/**` (token storage) | The server side of the protocol (B3) |
| `src/hooks/**` (the public typed hooks F1/F2/F4 consume) | |

**Change rule for the protocol:** any change updates, in the same PR: your
`src/types/protocol.ts`, B3's `backend/src/protocol/`, and
`docs/protocol-v1.md`. Never rename a field silently.

## 3. The base that already exists (don't rebuild it)

- `zustand ~5.1.0` (state), `expo-secure-store ~58.0.3` (token storage),
  `expo-crypto ~58.0.9` (UUIDs) — installed and SDK-matched.
- No socket library is needed and none is installed: React Native ships a
  global `WebSocket` class — use it directly (see §5).
- The React Native `WebSocket` API is callback-based (`onopen`, `onmessage`,
  `onclose`, `onerror`, `close()`, `send(string)`).

## 4. Protocol types — `src/types/protocol.ts`

Implement the TS mirror of `docs/protocol-v1.md` (it's the source of truth;
B3's Rust structs mirror it too). Shape:

```ts
export const PROTOCOL_VERSION = 1;

// Client → server: discriminated union on `type`, flattened into the envelope
export type ClientEnvelope = { v: number; id: string } & ClientCommand;

export type ClientCommand =
  | { type: "room.create"; payload: { nickname: string } }
  | { type: "room.join"; payload: { room_code: string; nickname: string } }
  // …every command from docs/protocol-v1.md §3
  ;

export type ServerEnvelope = { v: number; revision?: number } & ServerEvent;

export type ServerEvent =
  | { type: "room.created"; payload: { room_code: string; player_id: string;
      player_token: string; color: PlayerColor; room: RoomPublicState } }
  // …every event from docs/protocol-v1.md §4
  ;

export type ErrorCode = "ROOM_NOT_FOUND" | "ROOM_FULL" | "ROOM_EXPIRED" |
  "INVALID_TOKEN" | "NOT_YOUR_TURN" | "ILLEGAL_MOVE" | "STALE_REVISION" |
  "GAME_FINISHED" | "INVALID_PAYLOAD" | "UNSUPPORTED_VERSION" |
  "RATE_LIMITED" | "INTERNAL";

export type GameSnapshot = { /* fen, turn, game_status, result, legal_moves,
  move_history, players, draw_offer, room_code, status — exact §4.3 */ };
```

Add a small **decoder** (`parseServerEnvelope(raw: string): ServerEnvelope | null`)
so malformed server data is dropped in ONE place instead of crashing screens.

## 5. WebSocket service — `src/services/ws-service.ts`

One connection per app, owned by a module-level service (not by screens):

```ts
// Sketch — implement it your way, keep the responsibilities:
export const wsService = {
  connect(url: string): void;            // ws:// dev, wss:// prod (no trailing logic in screens)
  send(command: ClientCommand): void;    // adds v, unique id (expo-crypto randomUUID), enqueue-or-fail
  onEvent(handler: (e: ServerEvent) => void): () => void;  // unsubscribe fn
  disconnect(): void;
};

// Reconnect with capped exponential backoff + jitter (e.g. 0.5s → 30s, ±30%);
// stop retrying when the room is finished/expired or the user leaves.
```

Rules:
- Unique command IDs via `expo-crypto`'s `randomUUID()`.
- You own `expected_revision`: read the store's current revision when
  sending a state-changing command.
- Buffer/reject sends while disconnected — surface connection state in the
  store, never leave a "silent dead" UI.
- On `onclose` (not user-initiated) or app foregrounding (AppState →
  "active"), attempt resume: `session.resume` with the stored token, then
  **replace** the whole local state with the `session.ready` snapshot.
- Stale socket callbacks must never overwrite newer state — guard by
  connection/generation counter.
- `player_token` goes ONLY to `expo-secure-store` (via `src/security/`),
  never in logs, route params, or normal persisted state. Clear it when the
  player leaves or the room expires.

## 6. Stores — `src/store/`

zustand, one store per domain; screens subscribe with **narrow selectors**
(`useRoomStore((s) => s.status)`), not whole-store reads:

```ts
// src/store/connection-store.ts — connecting|connected|reconnecting|disconnected
// src/store/room-store.ts       — roomCode, players, status, drawOffer, revision
// src/store/game-store.ts       — the CURRENT snapshot applied atomically:
//                                  { revision, fen, turn, gameStatus, result,
//                                    lastMove, moveHistory, legalMoves, pending }
// src/store/session-store.ts    — yourColor, playerId (NOT the token — that's secure storage)
```

Store rules (from the master plan):

- Apply a snapshot **atomically** (one `set()` — FEN + turn + revision
  together, never separate async steps). Ignore snapshots with
  `revision <=` the applied one. If revisions skip, the next full snapshot
  still wins — don't assume every intermediate event arrives.
- Never optimistically apply your own move: set `pending: true`, wait for
  `game.state`. On `game.move_rejected`: clear pending, show the safe error,
  apply the newer snapshot if the server included one.
- Connection status stays separate from game status: disconnected ≠ finished.
- One in-flight command per action: ignore duplicate taps of move/resign
  buttons while `pending`.

## 7. Public hooks — `src/hooks/` (your API to F1/F2/F4)

Freeze these names with F1/F4 on Day 1 (proposed):

```ts
useRoomActions()  // createRoom, joinRoom, resumeRoom, leaveRoom, sendMove,
                 // offerDraw, acceptDraw, resign, rematch — each sets loading/error
useRoomState()    // roomCode, players, status, drawOffer, yourColor
useGameState()    // the snapshot + pending
useConnection()   // connection state for banners/disabled states
```

## 8. How to test your module independently

```bash
cd frontend
npm run typecheck && npm run lint
npx jest --testPathPattern="(ws|store|types)"
```

- Unit tests: envelope encode/decode round-trips against the JSON examples
  in `docs/protocol-v1.md` (they are fixtures — paste them verbatim);
  malformed payload handling; revision guard (older snapshot ignored);
  pending clear on rejection; token never in any store snapshot
  (assert serialize(store) contains no token).
- Mock `global.WebSocket` in jest (a tiny fake with `send` capture and
  event triggering) to test connect/backoff/resume logic without a server.
- On Day 3–4 wire it to B3's real server for the two-device vertical slice.

## 9. Deliverables (definition of done)

- [ ] `src/types/protocol.ts` matching `docs/protocol-v1.md` + decoder
- [ ] Single-connection WebSocket service with backoff+jitter reconnect + resume
- [ ] zustand stores with atomic snapshot application + revision guards
- [ ] `expo-secure-store` token handling (set/get/clear, never logged)
- [ ] The four hooks, typed, with loading/error states
- [ ] Unit tests (§8) — protocol fixtures, store guards, WS fake
- [ ] Error-code → user-message map table in this README (update §9)

## 10. Key coordination

- **B3** from Day 1: freeze `docs/protocol-v1.md` together.
- **F1** (create/join flows), **F2** (snapshot fields), **F4** (game screen):
  publish the hook names early; changes via PR.
- **B4**: end-to-end tests use your connection states — keep them observable.
