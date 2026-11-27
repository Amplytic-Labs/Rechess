# 02 — Controls and result states

All controls go through F3's actions (`useRoomActions()`). F3 owns
`expected_revision` and duplicate-submission guards (`pending`) — you own the
button states, confirmations and result presentation.

Every control you render is a shared UI primitive from F1's
`src/components/common/` (RULES.md §1.3) — the app has **no component
library** by design, so never hand-roll a button/dialog/toast copy here.
Exact import names, paths and props are defined by F1's doc set; if a
variant you need is missing, that's a short PR to F1, not a private
duplicate.

## 1. Offer / accept draw

Wire state comes from the snapshot: `draw_offer: PlayerColor | null` — the
color of the player with an active offer. It identifies **who** offered
(your brief §6).

| Situation (`draw_offer`, game in progress) | What you render | On tap |
|---|---|---|
| `null` | "Offer draw" button (`bg-surface`, `border-border`) | `offerDraw()` |
| `yourColor` | "Draw offered — waiting for opponent…" (info state, no button) + a way to see it's yours | nothing (offer stands; server has no withdraw command in v1) |
| opponent's color | "Opponent offers a draw" banner + **"Accept draw"** (`bg-primary`/`text-primary-foreground`) + decline = just keep playing | `acceptDraw()` |

Protocol subtlety (`docs/protocol-v1.md` §3): sending `game.offer_draw` while
the **opponent already offered** **accepts** it — the server treats the offer
as mutual. So the "Offer draw" button and "Accept draw" are both correct
entries into the same state; still render the explicit Accept button when
you know an offer is on the table (clearer UX), and call `acceptDraw()` for
it. When either path succeeds, both clients receive a `game.state` snapshot
with `result: { winner: null, reason: "draw_agreed" }` — render the result
view (§4).

Visibility: draw controls only while `status === "active"` and
`result === null`. Disable (don't hide) while `pending` is true — no
duplicate submissions.

## 2. Resign — confirmation dialog, destructive styling

Resign is irreversible; it must require confirmation (your brief §6). Use
the shared **Dialog** and **Button** primitives from F1's
`src/components/common/` — a hand-rolled copy here would fork focus
management, keyboard operability and styling across the app (RULES.md §1.3,
§2 accessibility):

```tsx
// F1's shared primitives — exact names/paths/props come from F1's doc set
// (src/components/common/); verify them there before wiring, and PR to F1
// if a variant is missing.
import { Button } from "@/components/common/button";
import { Dialog } from "@/components/common/dialog";
```

```tsx
{/* SKELETON — F4: implement (exact subcomponents/props per F1's docs) */}
<Button variant="outline" className="border-destructive text-destructive">
  Resign
</Button>
<Dialog>
  {/* Confirm: bg-destructive + text-primary-foreground
      Cancel: bg-surface + border-border */}
</Dialog>
```

Styling rules:

- The confirm action uses the `destructive` token: `bg-destructive` with
  `text-destructive-foreground`, or an outline variant
  (`border-destructive text-destructive`) for the trigger.
- Confirm calls `resign()` once; dialog open/closed is local UI state. While
  `pending`, the trigger is disabled.
- On success both clients get `game.finished` /
  `game.state` with `result: { winner: <opponent color>, reason:
  "resignation" }`.

## 3. Rematch — vote flow

Protocol: `game.rematch` is a **vote**; the new game starts when both voted
(`docs/protocol-v1.md` §3), and the server then broadcasts a fresh
`game.state` snapshot (reset board, `status: "active"`, new revision context).

UX you build:

1. Game finished (`result !== null`) → show "Rematch" button.
2. On tap → `rematch()` (F3's name; your brief's TABLE 6 says
   `requestRematch()` — **flagged mismatch**, use F3's).
3. Immediately after your vote → "Waiting for opponent…" state (spinner +
   text, `text-muted-foreground`). Key it to **your own sent vote** — the
   snapshot has NO rematch-vote field (see flag below), so until the fresh
   snapshot arrives the only vote you can show is yours.
4. Fresh `game.state` (`status: "active"`, `result: null`, empty
   `move_history`) = rematch started: clear result view, banners, local
   vote state; the board resets **from the snapshot** — never reset it
   locally.

> ⚠️ **Flagged protocol gap (raise with F3/B3):** `GameSnapshot` carries no
> rematch-vote field, so the opponent's vote is invisible until both voted
> and the fresh snapshot lands. If the team wants "opponent wants a
> rematch" shown to the non-voting player, F3 needs to surface it in room
> state (or the protocol needs a `rematch_offers` field). Until then:
> "Waiting for opponent…" keyed to your own vote is the best possible UX.

## 4. Result view — winner + reason mapping

When `snapshot.result !== null` (and/or a `game.finished` event landed in
F3's store), replace the turn indicator with the result view (an overlay
dialog via F1's Dialog primitive, or an inline panel —
`game-result-modal.tsx`):

- Headline from `result.winner`: `null` → "Draw"; your color → "You win";
  opponent's color → "You lose" (or "<nickname> wins" — pick one, keep it
  consistent).
- Reason line from `result.reason` (the **full** table — every
  `GameResultReason` in `@/types/protocol`):

| `reason` | `winner` | Headline | Reason text (suggested — refine the wording with the team) |
|---|---|---|---|
| `"checkmate"` | your color | "You win" | "Checkmate — your opponent has no legal moves." |
| `"checkmate"` | opponent's color | "You lose" | "Checkmate — you have no legal moves." |
| `"stalemate"` | `null` | "Draw" | "Stalemate — no legal moves and no check." |
| `"resignation"` | your color | "You win" | "Your opponent resigned." |
| `"resignation"` | opponent's color | "You lose" | "You resigned." |
| `"draw_agreed"` | `null` | "Draw" | "Draw agreed by both players." |
| `"insufficient_material"` | `null` | "Draw" | "Neither side has enough material to force checkmate." |
| `"threefold_repetition"` | `null` | "Draw" | "The same position occurred three times." |
| `"fifty_move_rule"` | `null` | "Draw" | "Fifty moves passed with no capture or pawn move." |

- Style win/lose/draw with tokens (e.g. `text-success` for a win, muted for
  a draw, `text-destructive`-free choice for a loss — don't invent colors).
- Below the result: rematch button (§3) + leave ("Back to menu" → `leaveRoom()`
  + navigation agreed with F1). The board stays visible but **inert**
  (doc 03 §6).
- Build the mapping as a PURE table/function in `result-reasons.ts` so it is
  trivially reviewable; keep every `GameResultReason` covered — TypeScript
  exhaustiveness (`Record<GameResultReason, …>`) proves it at typecheck time.

## 5. Error codes → friendly themed messages (RULES.md §4: never raw JSON)

F3 surfaces errors (e.g. from `game.move_rejected` / `server.error`) as an
`ErrorCode`-typed value + the server's safe `message`. You map the code —
show your themed text, never raw JSON. Full table — every `ErrorCode` in
`@/types/protocol`:

| `ErrorCode` | Friendly message (suggested) | Where it usually surfaces |
|---|---|---|
| `ROOM_NOT_FOUND` | "That room no longer exists." | navigation edge cases; F1 owns lobby flows |
| `ROOM_FULL` | "That room already has two players." | lobby (F1) — listed for completeness |
| `ROOM_EXPIRED` | "This room has expired." | result/exit flows |
| `INVALID_TOKEN` | "Your session could not be restored." | resume failure — back to home (F3/F1) |
| `NOT_YOUR_TURN` | "It's not your turn yet." | move rejected toast |
| `ILLEGAL_MOVE` | "That move isn't legal in this position." | move rejected toast |
| `STALE_REVISION` | "Position changed — showing the latest move." | move rejected; board reconciles from fresh snapshot |
| `GAME_FINISHED` | "The game is already over." | tapping through a stale control |
| `INVALID_PAYLOAD` | "Something went wrong — try again." | should not happen if UI validates |
| `UNSUPPORTED_VERSION` | "The app needs an update to connect." | protocol version mismatch |
| `RATE_LIMITED` | "Too many actions — wait a moment." | rapid taps / reconnect storms |
| `INTERNAL` | "Something went wrong on our side." | server bug — safe fallback |

Implementation notes:

- Put this in a PURE `error-messages.ts` (`Record<ErrorCode, string>` —
  exhaustive at typecheck; never a fallback that leaks `message` or JSON).
- Display via a short-lived themed toast/banner — F1's toast primitive from
  `src/components/common/` (auto-dismiss, token-styled). If F1 has not
  landed one yet, coordinate: a temporary inline banner in this folder is
  acceptable while developing, but it moves to `src/components/common/` via
  F1 as soon as the app needs it anywhere else — never keep a private
  copy. The wire `message` is safe for display, but prefer your mapped text
  for tone consistency.
- For move rejections specifically, the board must **not** be patched by
  you — the fresh snapshot F3 applied is the truth (doc 03 §2).
