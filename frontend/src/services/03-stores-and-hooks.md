# 03 — Stores (`src/store/`) and hooks (`src/hooks/`)

> Zustand (already installed, `~5.1.0`). One store per domain; screens
> subscribe with **narrow selectors** (`useRoomStore((s) => s.status)`),
> never whole-store reads. State is server-authoritative — your stores
> are a typed cache of what the server said, plus pending/error flags.

## 1. The three stores

| File | Store | Holds | Notable fields |
|---|---|---|---|
| `src/store/connection-store.ts` | `connectionStore` | transport status only | `status` (enum from doc 02 §5), `attempts` (backoff attempt count), `lastError` (last protocol/user-facing error) |
| `src/store/room-store.ts` | `roomStore` | room + identity | `room_code`, `players` (`PlayerPublic[]`), `status` (`RoomStatus`), `yourColor` (`PlayerColor \| null`), `playerId`, `roomState` — plus action state: `loading`, `error` |
| `src/store/game-store.ts` | `gameStore` | the authoritative game | `revision`, `snapshot` (the `GameSnapshot` fields, or the snapshot itself — pick one shape and document it), `pending`, `lastError` |

> Consolidation note: the previous `README.md` (§6) proposed a fourth,
> `session-store.ts` (`yourColor`, `playerId`). This doc folds those into
> `roomStore` ("my credentials wrapper") — one fewer store, same data.
> If you prefer four stores, update this doc in the same PR; do not ship
> both. The `player_token` is **not** in any store — SecureStore only
> (doc 04).

Rules:

- **Connection status is separate from game status** — disconnected ≠
  finished (master plan §8). A finished game can be viewed offline.
- Action state (loading/error) lives with the action, not in a global
  toast queue — screens bind it to the button that caused it.

## 2. The snapshot you store — `GameSnapshot` (from `protocol.ts`)

```ts
export interface GameSnapshot {
  room_code: string;
  status: RoomStatus;
  game_status: GameStatus;
  result: GameResult | null;
  fen: string;
  turn: PlayerColor;
  last_move: MoveRecord | null;
  move_history: string[];
  legal_moves: string[];
  players: PlayerPublic[];
  draw_offer: PlayerColor | null;
}
```

`legal_moves` (UCI-style, e.g. `"e2e4"`) comes from the server ONLY —
never compute it client-side (RULES.md §4).

## 3. The atomic update rule (the most important rule in this doc)

`revision` is **envelope-level** on stateful events — this is the exact
shape from `protocol.ts`:

```ts
export type ServerEnvelope = { v: number; revision?: number } & ServerEvent;
```

So a `game.state` arrives as
`{ v: 1, type: "game.state", revision: 13, payload: GameSnapshot }` —
the `revision` is a sibling of `payload`, never inside it.

The rule: **apply an incoming snapshot only if its envelope `revision`
is `>=` the currently stored revision** — out-of-order and duplicate
deliveries are silently dropped. Store `revision` and the whole snapshot
in ONE zustand `set()` call (FEN + turn + players + revision together,
never separate async steps — `docs/protocol-v1.md` §8.3). If revisions
skip (12 → 15), the newer full snapshot still wins — never assume every
intermediate event arrives. The same guard governs `game.finished` and
the optional snapshot inside `game.move_rejected`. A `session.ready`
snapshot (no envelope revision) is the one exception: it **replaces**
the whole local game state unconditionally — the server just told you
exactly where you are (followed by a `game.state` carrying the revision,
doc 02 §7).

## 4. The pending-move pattern (RULES.md §4: never optimistic)

1. `submitMove(from, to, promotion)` → set `pending = { from, to,
   promotion }` in `gameStore`, send the `game.move` command (doc 02 §3).
   **Do not touch `snapshot.fen`.**
2. On `game.state` (guard passed) → apply the new snapshot atomically
   **and** clear `pending` in the same `set()`.
3. On `game.move_rejected` → clear `pending`, set the safe error
   (`payload.error` + `payload.message`), and if `payload.snapshot` is
   present apply it under the revision guard.
4. On resume (`session.ready`) → clear `pending` (the snapshot is
   the arbiter: the move landed or it never happened).
5. Duplicate submissions: ignore move/action button presses while
   `pending` is set (screens disable, service double-guards).

Do not blindly re-send a move after a timeout — the server dedups by
command id (B3 doc 02 §5.3, D1) but the safe client reconciles via
resume + full snapshot first.

## 5. The hooks (`src/hooks/`) — frozen names

**The names below are already referenced by F1's and F4's docs —
they are frozen, not a proposal.** F4's
`src/screens/game/01-screen-composition.md` §6 wires exactly these hooks
(`useGameState()`, `useRoomState()`, `useRoomActions()`, `useConnection()`).
F1's `src/screens/lobby/04-acceptance.md` §2 pins the same names
(`useRoomActions`, `useRoomState`).

| Frozen hook | Returns | Consumed by |
|---|---|---|
| `useConnection()` | `connectionStore` slice: `status`, `attempts`, `lastError` — for banners/disabled states | F1 (waiting room), F4 (connection banner) |
| `useRoomState()` | `{ roomCode, players, status, yourColor, connection }` (F1's `04-acceptance.md` §2 pins `roomCode, players, status, connection`; camelCase display names mapping `room_code` etc.) | F1 (waiting room), F4 (panels) |
| `useGameState()` | `{ snapshot, pending }` — the whole `GameSnapshot` (+ `revision` if F4 needs it) | F4 (board/turn/history/result) |
| `useRoomActions()` | `createRoom, joinRoom, submitMove, offerDraw, acceptDraw, resign, rematch, leaveRoom` — each sets loading/error and returns a promise where a reply is expected | F1 (create/join), F4 (controls), F2 via F4 |

Reconciliation flag (agreed direction, confirm on Day 1): an earlier
draft proposed `useConnectionStatus / useRoom / useGameSnapshot /
usePlayers / useSendCommand`. Those do **not** match F1/F4's READMEs and
are NOT adopted. Mapping: `useConnectionStatus` → `useConnection`;
`useRoom` → `useRoomState`; `useGameSnapshot` → `useGameState`;
`usePlayers` → the `players` field of `useRoomState`; `useSendCommand` →
`useRoomActions`. If F3/F1/F4 later agree to rename anything, the PR
must update both screen READMEs in the same change.

Signatures F1's README pins (§6): `createRoom(nickname)` resolves
`{ roomCode, yourColor }`; `joinRoom(roomCode, nickname)` resolves
`{ roomCode, yourColor }`; both expose loading + error states. Resume is
service-internal (automatic on reconnect/foreground, doc 02 §7) —
screens never call it. F4's README pins `leaveRoom` ("navigate out
cleanly … call F3's `leaveRoom`").

## 6. Actions — 1:1 with `ClientCommand`

The `ClientCommand` union (verbatim from `protocol.ts`) and its action:

```ts
export type ClientCommand =
  | { type: "room.create"; payload: { nickname: string } }              // createRoom(nickname)
  | { type: "room.join"; payload: { room_code: string; nickname: string } } // joinRoom(roomCode, nickname)
  | { type: "session.resume"; payload: { room_code: string; player_token: string } } // internal
  | {
      type: "game.move";                                                 // submitMove(from, to, promotion)
      payload: {
        from: string;
        to: string;
        promotion: Promotion | null;   // always present; null when not a promotion
        expected_revision: number;     // read from gameStore at send time
      };
    }
  | { type: "game.offer_draw"; payload: { expected_revision: number } }  // offerDraw()
  | { type: "game.accept_draw"; payload: { expected_revision: number } } // acceptDraw()
  | { type: "game.resign"; payload: { expected_revision: number } }     // resign()
  | { type: "game.rematch"; payload: { expected_revision: number } }    // rematch()
  | { type: "room.leave"; payload: EmptyPayload };                      // leaveRoom()
```

One action = one command variant = one promise/error state, nothing
more. Each action: guard on connection status → set loading → build the
command → `websocketService.send(command)` → resolve on the matching
reply event (`room.created`/`room.joined`), reject on `server.error`.
`submitMove` uses the pending pattern (§4), not a promise.

## 7. Wire-up sketch

```ts
// src/hooks/use-room-actions.ts — SKELETON
import { useRoomStore } from "@/store/room-store";

export function useRoomActions() {
  // TODO: return { createRoom, joinRoom, submitMove, offerDraw,
  //         acceptDraw, resign, rematch, leaveRoom } — each: loading +
  //         error in roomStore, promise resolved by the service when the
  //         reply event arrives (doc 02 §3).
  // TODO: subscribe to websocketService.onEvent once (module level), route
  //         events into the stores (doc 02 §4) — hooks only read stores.
}
```

```ts
// src/store/game-store.ts — SKELETON
interface PendingMove { from: string; to: string; promotion: Promotion | null; }

interface GameStore {
  revision: number;
  snapshot: GameSnapshot | null;        // null until the first game.state
  pending: PendingMove | null;
  // TODO: applySnapshot(envelopeRevision, snapshot) — atomic set + guard (§3);
  //       clearPending(), rejectPending(error, snapshot | null) (§4).
}
```

Hooks stay thin (selectors + stable action callbacks); the service routes
events into stores; stores hold state; screens render. Nothing more.
