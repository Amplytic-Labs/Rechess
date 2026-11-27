# WebSocket Protocol — v1 (DRAFT)

> **Status: DRAFT — B3 (Rust) and F3 (TypeScript) review this together on
> Day 1 and freeze it.** After the freeze, every change is a PR that
> updates this file, `backend/src/protocol/` (B3) and
> `frontend/src/types/protocol.ts` (F3) **in the same PR**.
>
> This document specifies the wire JSON. It is the single source of truth.
> The Rust structs and the TS types must match it exactly.

## 1. Transport

- Single WebSocket endpoint per client: `ws://<host>/ws` (dev) /
  `wss://<host>/ws` (deployed, TLS terminated at the reverse proxy).
- All messages are single-frame JSON **text** messages, one message per
  command/event, UTF-8.
- The server is authoritative for everything: FEN, turn, legal moves, game
  status, result. Client-claimed values are ignored.

## 2. Envelopes

### 2.1 Client → server (command)

```json
{
  "v": 1,
  "id": "cmd-unique-001",
  "type": "game.move",
  "payload": { "from": "e2", "to": "e4", "promotion": null, "expected_revision": 12 }
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `v` | number | ✔ | Protocol version. Unsupported ⇒ `UNSUPPORTED_VERSION` error. |
| `id` | string | ✔ | Unique per client (suggest UUID). Used for dedup; echoed nowhere except errors if agreed. |
| `type` | string | ✔ | Command name (namespaced `area.action`). |
| `payload` | object | ✔ | Per-command fields (may be `{}` for e.g. `room.leave`). |

### 2.2 Server → client (event)

```json
{
  "v": 1,
  "type": "game.state",
  "revision": 13,
  "payload": { "fen": "…", "turn": "black", "…": "…" }
}
```

| Field | Type | Required | Notes |
|---|---|---|---|
| `v` | number | ✔ | Always `1` in v1. |
| `type` | string | ✔ | Event name. |
| `revision` | number | stateful events only | Room revision after this event (§5). |
| `payload` | object | ✔ | Per-event fields. |

## 3. Client commands (v1)

| `type` | `payload` | Effect |
|---|---|---|
| `room.create` | `{ "nickname": string }` | Create a room; host takes White. Private reply `room.created`. |
| `room.join` | `{ "room_code": string, "nickname": string }` | Join a waiting room; guest takes Black. Private reply `room.joined`. |
| `session.resume` | `{ "room_code": string, "player_token": string }` | Resume after reconnect. Reply `session.ready` with full snapshot. |
| `game.move` | `{ "from": square, "to": square, "promotion": "q"\|"r"\|"b"\|"n"\|null, "expected_revision": number }` | Submit a move. `square` = `"a1".."h8"`. Broadcast `game.state` on success, `game.move_rejected` on failure. |
| `game.offer_draw` | `{ "expected_revision": number }` | Offer a draw. If the opponent already offered, this **accepts** it (no separate accept needed from the offerer). |
| `game.accept_draw` | `{ "expected_revision": number }` | Accept the opponent's active offer. |
| `game.resign` | `{ "expected_revision": number }` | Resign the current game. |
| `game.rematch` | `{ "expected_revision": number }` | Vote for a rematch; new game starts when both voted. |
| `room.leave` | `{}` | Leave the room explicitly. |

Payload validation (enforced by B3 at the gate, before B2):
nickname 1–20 chars, no control characters; room code 8 chars from
`ABCDEFGHJKMNPQRSTUVWXYZ23456789`, case-insensitive (server uppercases);
unknown/missing fields in `payload` ⇒ `INVALID_PAYLOAD`.

## 4. Server events (v1)

### 4.1 Private (targeted to one connection — these are the only events that may contain `player_token`)

| `type` | Payload | Sent when |
|---|---|---|
| `room.created` | `{ "room_code": string, "player_id": string, "player_token": string, "color": "white", "room": RoomPublicState }` | Reply to `room.create`. |
| `room.joined` | `{ "player_id": string, "player_token": string, "color": "black", "room": RoomPublicState }` | Reply to `room.join`. |
| `session.ready` | `{ "player_id": string, "color": "white"\|"black", "snapshot": GameSnapshot }` | Reply to `session.resume` (also used after both seats fill). |

### 4.2 Public (broadcast to the room / both players)

| `type` | Payload | Sent when |
|---|---|---|
| `game.state` | `GameSnapshot` | After every accepted state change (moves, draw accept, resign, rematch start) |
| `player.presence` | `{ "player_id": string, "connected": boolean }` | Opponent connect/disconnect |
| `game.move_rejected` | `{ "error": ErrorCode, "message": string, "snapshot": GameSnapshot \| null }` (envelope carries `revision`) | A submitted move failed validation (to the mover only) |
| `game.finished` | `{ "result": GameResult, "snapshot": GameSnapshot }` (envelope carries `revision`) | Game over (checkmate / stalemate / draw / resignation) |
| `server.error` | `{ "error": ErrorCode, "message": string }` | Any rejected command or protocol violation |

### 4.3 Shared payload objects

```jsonc
// RoomPublicState — never contains tokens
{ "room_code": "ABC23XY9", "status": "waiting", "players": [ /* PlayerPublic */ ] }

// PlayerPublic
{ "id": "p-8…", "nickname": "Guest", "color": "white", "connected": true }

// GameSnapshot — the full authoritative state
{
  "room_code": "ABC23XY9",
  "status": "active",                  // room status: waiting|active|finished|expired
  "game_status": "in_progress",        // in_progress|check|checkmate|stalemate|draw|resigned
  "result": null,                      // GameResult | null — present when finished
  "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
  "turn": "black",
  "last_move": { "from": "e2", "to": "e4", "san": "e4" },
  "move_history": ["e4"],
  "legal_moves": ["e7e5", "e7e6", "…"],   // UCI-style, from the server ONLY
  "players": [ /* PlayerPublic, PlayerPublic */ ],
  "draw_offer": null                   // "white" | "black" | null
}

// GameResult — reason values line up with B1's engine outcomes
{ "winner": "black", "reason": "checkmate" }   // winner: null for draws
```

`GameResult.reason`: `checkmate | stalemate | resignation | draw_agreed |
insufficient_material | threefold_repetition | fifty_move_rule`.

## 5. Revision & idempotency semantics

- `revision` is a per-room, monotonically increasing counter starting at 0,
  bumped by 1 for every **accepted** state-changing command.
- State-changing commands carry `expected_revision`. If it does not equal the
  room's current revision, the command is rejected with `STALE_REVISION` and
  the client should reconcile from the newest snapshot.
- Every client command `id` must be unique (UUID). The room actor remembers
  recent IDs per player in a bounded window: a retried `game.move` with an
  already-processed ID is not applied twice (either idempotent-accepted with
  the original outcome or `STALE_REVISION`, agreed by B2+B3).
- Clients must not optimistically apply their own move to the board — set a
  "pending" indicator and wait for `game.state`.

## 6. Error codes

`ROOM_NOT_FOUND` · `ROOM_FULL` · `ROOM_EXPIRED` · `INVALID_TOKEN` ·
`NOT_YOUR_TURN` · `ILLEGAL_MOVE` · `STALE_REVISION` · `GAME_FINISHED` ·
`INVALID_PAYLOAD` · `UNSUPPORTED_VERSION` · `RATE_LIMITED` · `INTERNAL`

Error `message` strings are safe for display; diagnostic detail stays in
server logs. Never include the other player's token or internal errors.

## 7. Reconnect flow

1. Socket closes / app backgrounds → client keeps `player_token` (secure
   storage only) and room code in the session store.
2. Client reconnects with capped exponential backoff + jitter (e.g. 0.5s →
   30s), sends `session.resume`.
3. Valid token ⇒ `session.ready` with the **full** snapshot at the current
   revision; the client replaces its whole local state with it.
4. Invalid/expired token ⇒ `INVALID_TOKEN`; the client returns to the home
   screen with a friendly message.
5. On the server, disconnect starts a grace deadline (B2); a valid resume
   cancels it. Expiry of the grace window forfeits/resolves the room
   according to B2's lifecycle rules.

## 8. Invariants (all must hold in every message)

1. `player_token` appears **only** in `room.created` / `room.joined`
   payloads, sent only to the initiating connection. Never in broadcasts,
   logs, or any other event.
2. Server-generated fields (FEN, turn, legal moves, result) are never taken
   from the client.
3. A snapshot is applied atomically by the client (FEN + turn + revision
   together); older-revision snapshots are ignored by the client.
4. Unknown protocol versions get a structured `UNSUPPORTED_VERSION` error,
   not a dropped connection.
