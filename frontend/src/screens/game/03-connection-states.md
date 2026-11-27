# 03 — Non-happy-path UX: pending, rejected, presence, reconnect

The happy path is doc 01. This doc is everything that can go wrong — and it
is where the screen proves the golden rule: **the displayed position is only
ever the server's snapshot.**

## 1. Pending move — after `onMove`, before `game.state`

**Rule (RULES.md §4, protocol §5): NEVER show your own move applied.** The
board component reports the attempt; the server is the only authority. There
is no optimistic board commit in this app.

What you build:

- `onMove` → immediately hand off to `sendMove(...)` and render a **pending
  indicator** keyed to `pending` from `useGameState()` (F3 sets it on send and
  clears it on the next `game.state` / rejection): a small spinner next to
  your player bar and/or "Sending move…" in the turn indicator (HeroUI
  `heroui-native/spinner` is a verified subpath — check its named exports in
  the installed package, granular import only).
- While `pending`: `disabled` for the board is true (a second tap must not
  submit twice — F3 also guards, but the UI must not invite it).
- The board's FEN does not change until the next snapshot arrives — that IS
  the correct behavior; do not "help" it.

Sequence you will see (two devices, revision N):

1. You tap → F3 sends `game.move` with `expected_revision: N`.
2. Screen: `pending === true`, board disabled.
3. Server accepts → broadcasts `game.state` (revision **N+1**) → F3 applies
   the snapshot atomically → board updates, `pending === false`, history row
   appears. Both devices converge on the same revision.

## 2. Move rejected — `game.move_rejected`

The event is targeted at the mover and carries
`{ error: ErrorCode, message: string, snapshot: GameSnapshot | null }`, with
the new revision on the envelope (protocol §4.2). **F3 handles it**: clears
`pending`, applies the included snapshot if present. **You render the
result**:

- Show the mapped friendly message (doc 02 §5) for `error` — most commonly
  `NOT_YOUR_TURN`, `ILLEGAL_MOVE`, `STALE_REVISION`, `GAME_FINISHED`.
- **Reconcile from the fresh snapshot — never patch the board yourself.**
  Because F3 applied the newer snapshot, your board props (`fen`,
  `legalMoves`, `turn`, `lastMove`, `checkSquare`) already re-derive from it;
  make sure they are store-driven so this is automatic.
- UX: a short-lived toast/banner, auto-dismisses (doc 02 §5). The player's
  selection is F2's local state; their contract clears it when `fen`/revision
  changes — nothing for you to reset.

## 3. Opponent presence — `player.presence`

`player.presence` (`{ player_id, connected }`, protocol §4.2) arrives when the
opponent connects/disconnects; F3 merges it into the players array in room
state. The snapshot's `players[].connected` alone can be stale between
presence events — **always read presence through F3's hooks** (`players` from
`useRoomState()`), never cache players in screen state.

What you render:

- **Opponent disconnected:** an "Opponent disconnected" banner (distinct from
  your own reconnecting banner — different states, different copy; your brief
  §2: "Distinguish a temporary disconnect from a finished game"):
  e.g. `bg-surface-muted` banner with `text-muted-foreground` and a
  destructive dot; text like "Opponent disconnected — waiting for them to
  reconnect…". Never present this as a loss or a result.
- **Grace window expectation:** the server keeps the seat for a bounded
  reconnect grace period (default ~60s per the master plan — a B2 config
  constant; don't hardcode a countdown in the UI). Copy should set the
  expectation of auto-resume ("they can reconnect and the game continues").
  What actually happens at grace expiry is B2's lifecycle call (forfeit or
  expiry) — when a decision lands you will receive it as a snapshot /
  `game.finished`, and the result view takes over. Until then: waiting
  state, not defeat.
- **Opponent reconnected:** banner disappears (players array updates);
  the game resumes from the current snapshot with no special action from you.
- The presence **dot** in the opponent's `PlayerInfo` bar (`bg-success` /
  muted) is the compact, always-visible version of the same state (doc 01 §5).

## 4. Your own reconnect — connection banner

Your own connection is F3's domain: backoff reconnect with
`session.resume`, and the resume flow replaces the whole local state from
`session.ready` (protocol §7). **The screen just renders state** — it never
triggers reconnects:

- `useConnection()` (F3's README §6 store states): `"connecting"` |
  `"connected"` | `"reconnecting"` | `"disconnected"`.
- `"connecting"`/`"reconnecting"` → a "Reconnecting…" banner (e.g.
  `bg-surface-muted`, `border-border`, spinner + text). Keep it calm and
  persistent — it leaves when F3 reports `"connected"`.
- While NOT `"connected"`: the board is **disabled** (board input must not
  queue moves while disconnected — F3 buffers/rejects sends; the UI must not
  imply moves will land).
- `"disconnected"` after retries gave up → the persistent banner plus a
  retry affordance only if F3 exposes one — otherwise the banner plus your
  leave action. **Do not** call `wsService` from the screen; if resume
  behavior needs surfacing, ask F3 to expose it in the hook (flagged in
  doc 05).
- **Distinct from game-over:** connection state and game state are separate
  stores (F3's README §6: "disconnected ≠ finished"). The result view renders
  from `result`; the banner renders from `connection` — they must never be
  derived from each other.

## 5. Not-your-turn / disabled states

`turn` comes from the snapshot; `yourColor` from room state:

- While `turn !== yourColor` and the game is live: board `disabled`, turn
  indicator shows "Waiting for opponent…". The server still enforces turn —
  this is UX, not authorization (your brief §6).
- Disabled must be **visible but not jarring**: F2's contract takes a
  boolean `disabled`; combine all sources in ONE expression in the screen:
  `pending || turn !== yourColor || connection !== "connected" || result !== null`.
- Buttons follow the same discipline: draw/resign disabled while `pending`;
  rematch only when finished (doc 02 §3); leave always available (but confirm
  if the game is live — leaving an active game is destructive-adjacent).

## 6. Game finished — inert board + result + rematch

When `result !== null` (from a snapshot or `game.finished`):

- Board stays visible (the final position — e.g. the checkmated king) but is
  **inert**: `disabled` true, no selection/legality UI (F2 handles the
  disabled look).
- Turn indicator is replaced by the result view (doc 02 §4): headline,
  reason, rematch + leave.
- Draw offer banner hides; draw controls hide (`status`/`result` gates);
  rematch controls appear (§ doc 02 §3).
- The finished room is retained server-side (default ~10 minutes, master
  plan TABLE 7) so both players can see the result and vote a rematch; after
  expiry you may receive `ROOM_EXPIRED` → map it to the friendly message and
  route home with F1's navigation.
