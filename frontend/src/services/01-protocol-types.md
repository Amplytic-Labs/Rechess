# 01 — Owning `src/types/protocol.ts`

> The types are **already written and complete**. This doc tells you what
> exists, the rule you are bound by when touching it, and the verification
> habit that keeps the frontend and backend from drifting apart.

## 1. What already exists — do not rewrite it

`src/types/protocol.ts` is the complete TypeScript mirror of
`docs/protocol-v1.md` and the Rust types in `backend/src/protocol/`.
**Read the file itself**; this is the inventory you own:

| Export | Kind | Notes |
|---|---|---|
| `PROTOCOL_VERSION` | `const` (`1`) | Every `ClientEnvelope` carries `v: PROTOCOL_VERSION` |
| `PlayerColor` | union | `"white" \| "black"` |
| `RoomStatus` | union | `"waiting" \| "active" \| "finished" \| "expired"` |
| `GameStatus` | union | `"in_progress" \| "check" \| "checkmate" \| "stalemate" \| "draw" \| "resigned"` |
| `GameResultReason` | union | `checkmate \| stalemate \| resignation \| draw_agreed \| insufficient_material \| threefold_repetition \| fifty_move_rule` |
| `Promotion` | union | `"q" \| "r" \| "b" \| "n"` |
| `ErrorCode` | union | 12 codes: `ROOM_NOT_FOUND` `ROOM_FULL` `ROOM_EXPIRED` `INVALID_TOKEN` `NOT_YOUR_TURN` `ILLEGAL_MOVE` `STALE_REVISION` `GAME_FINISHED` `INVALID_PAYLOAD` `UNSUPPORTED_VERSION` `RATE_LIMITED` `INTERNAL` |
| `MoveRecord` | interface | `{ from, to, san }` |
| `PlayerPublic` | interface | `{ id, nickname, color, connected }` — **never** contains `player_token` |
| `RoomPublicState` | interface | `{ room_code, status, players }` |
| `GameResult` | interface | `{ winner: PlayerColor \| null, reason }` |
| `GameSnapshot` | interface | `{ room_code, status, game_status, result, fen, turn, last_move, move_history, legal_moves, players, draw_offer }` — the authoritative state |
| `EmptyPayload` | type | `Record<string, never>` (payload of `room.leave`) |
| `ClientCommand` | union | 9 variants — see doc 03 §5 |
| `ClientEnvelope` | type | `{ v: number; id: string } & ClientCommand` |
| `RoomCreatedPayload` | interface | the ONLY payload with `player_token` |
| `RoomJoinedPayload` | interface | also carries `player_token` |
| `SessionReadyPayload` | interface | `{ player_id, color, snapshot }` |
| `PlayerPresencePayload` | interface | `{ player_id, connected }` |
| `MoveRejectedPayload` | interface | `{ error, message, snapshot: GameSnapshot \| null }` |
| `GameFinishedPayload` | interface | `{ result, snapshot }` |
| `ServerErrorPayload` | interface | `{ error, message }` |
| `ServerEvent` | union | 8 variants: `room.created` `room.joined` `session.ready` `game.state` `player.presence` `game.move_rejected` `game.finished` `server.error` |
| `ServerEnvelope` | type | `{ v: number; revision?: number } & ServerEvent` |

Two invariants baked into the shapes (both load-bearing for your service):

- `revision` lives at **envelope level**, only on stateful events
  (`game.state`, `game.move_rejected`, `game.finished`) — never inside
  a payload.
- `type` + `payload` are flattened into both envelope shapes, exactly
  like serde flatten on the Rust side.

## 2. The binding change rule — the same-PR trio

Any protocol change, no matter how small, must update **all three in the
same reviewed PR**:

1. `backend/src/protocol/` (B3's Rust types)
2. `frontend/src/types/protocol.ts` (yours)
3. `docs/protocol-v1.md` (the spec)

Never rename a field silently. TypeScript and Rust both compile happily
after a one-sided rename — and then the app breaks at runtime, usually in
a way nobody reproduces locally. This rule is in the header comment of
`protocol.ts` itself; keep that header comment true.

## 3. The cautionary tale: `room.leave` payload shape

This actually happened and is recorded in
`backend/src/network/02-protocol-implementation.md` §9 (D4):

- The Rust `ClientCommand::LeaveRoom` was a **unit variant**. With
  `#[serde(tag = "type", content = "payload")]` it serialized as
  `{"v":1,"id":"…","type":"room.leave"}` — **no `payload` key** — and
  deserialization of `"payload": {}` failed with
  `invalid type: map, expected unit variant`.
- The TS types (correctly, per the spec) modeled it as
  `{ type: "room.leave"; payload: EmptyPayload }`, so a TypeScript
  client sent `"payload": {}` — and the Rust server answered
  `server.error INVALID_PAYLOAD`. Both sides "compiled"; the wire broke.
- **Fix:** the variant became a zero-field struct variant `LeaveRoom {}`,
  which serializes `"payload": {}` exactly like the TS types and the doc.

That is the entire argument for the same-PR trio in one incident. When
you are tempted to "just add a field" on one side, remember the leave
bug.

## 4. How to extend a type safely

Prefer this order, each step a joint PR with B3:

1. **Additive optional field first**: `field?: NewType` on the TS side,
   `#[serde(default)] Option<NewType>` on the Rust side, documented in
   `docs/protocol-v1.md`. Old and new clients coexist during rollout.
2. Only when every client is on the new version may the field become
   required.
3. Never rename. If you must change a field's name or meaning, add the
   new field alongside, migrate consumers, and remove the old one in a
   later PR — in that order, across the trio, each step.
4. Never change a wire *value* (e.g. a union member like `"in_progress"`)
   without a new value; removing/renaming values breaks old clients
   mid-flight.
5. Bump nothing silently: v1 has one `PROTOCOL_VERSION`; a v2 is a team
   decision, not a side effect.

## 5. `protocol.fixtures.ts` — how the rest of the team uses it

`src/types/protocol.fixtures.ts` contains typed, realistic wire data
(mirroring the backend fixtures in `backend/src/protocol/mod.rs` and
`backend/src/rooms/testkit.rs`):

- `sampleSnapshot: GameSnapshot` — full in-progress game, one move played
- `sampleRoom: RoomPublicState` — waiting room, host seated
- `sampleFinishedSnapshot: GameSnapshot` — checkmate result
- `sampleRoomCreatedEvent: ServerEvent` — the private create reply
  (token value is the dummy `"secret-token"` — a fixture, never real)
- `sampleMoveCommand: ClientEnvelope` — a `game.move` exactly as the
  client sends it

**While your service is in development, F1/F2/F4 build their UI against
these fixtures** (e.g. F1's waiting room renders `sampleRoom`, F4's
screen renders `sampleSnapshot`, the board adapter eats
`sampleSnapshot.fen`). That is the whole point: UI work does not block on
your socket. Two consequences for you:

1. **Do not change fixture shapes casually** — screens depend on them.
   If a fixture must change (because the protocol changed), announce it
   in the PR and check F1/F4's screens still render.
2. When your real service lands, the fixtures stay: they are the mock
   data for the dev-only mock strategy (doc 05 §4) and the typecheck
   proof of the contract (below).

## 6. The verification habit — after ANY edit to `protocol.ts`

No exceptions, every time:

```bash
cd frontend
npm run typecheck   # proves the fixtures still satisfy the types
npm run lint
```

and in the same PR:

- cross-check `docs/protocol-v1.md` — field names, values, and
  "Required" column must agree with your types;
- cross-check `backend/src/protocol/` — every Rust field name and serde
  rename must agree (B3 reviews, but you diff it yourself first);
- if you added/changed a fixture, re-read it against the new types (that
  is what makes `npm run typecheck` meaningful as a contract check).

Why typecheck is the logic gate: `protocol.fixtures.ts` imports the types
and constructs full realistic messages. If your edit breaks the contract,
`tsc --noEmit` fails on the fixtures. The backend half of the same proof
is `cargo test` (`backend/tests/protocol_contract.rs` round-trips the
identical JSON through the Rust wire types). One wire, two compilers.

## 7. Known nit to fix when you touch the file

The header comment in `protocol.ts` says the fixtures live in
`protocol.test.ts` — that file was renamed to `protocol.fixtures.ts` when
jest was dropped (RULES.md §5: no frontend test runner by design). Fix
the comment in your next PR touching the file so nobody goes hunting for
a test file that does not exist.
