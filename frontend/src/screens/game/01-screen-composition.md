# 01 — Screen composition and data wiring

> Everything in this doc lives under your ownership:
> `src/screens/game/**` + the thin route file
> `src/app/game/[roomCode]/page.tsx`.

## 1. The route file — `src/app/game/[roomCode]/page.tsx` (thin, RULES.md §3)

A placeholder route file already exists — you replace its body. Route files
render screen components; real UI lives in screens. The route receives
**only non-secret** references: the room code is part of the URL —
`player_token` NEVER appears in a URL (it stays in F3's storage module, and
F3's service resumes the session after navigation).

Keep the route a plain server component that renders your client screen; the
screen reads the room code from the URL itself with `useParams`
(`next/navigation`, RULES.md §3):

```tsx
// src/app/game/[roomCode]/page.tsx — THIN. No logic, no data, no styles.
import { GameScreen } from "@/screens/game/game-screen";

export default function GameRoute() {
  return <GameScreen />;
}
```

```tsx
// inside game-screen.tsx ("use client")
import { useParams } from "next/navigation";

const { roomCode } = useParams<{ roomCode: string }>();
```

**One route, three screens' worth of state.** The waiting state, the live
game and the result view are ALL rendered by this one route — never add
another route under `/game`. Which one shows is driven entirely by F3's
store state:

| F3 state | What you render |
|---|---|
| `status === "waiting"` (from `useRoomState()`) | F1's `WaitingRoom` (from `@/screens/lobby/waiting-room`) — room code, seated players, "waiting for opponent"; F1 owns the view, you mount it |
| `status === "active"` (or a snapshot with `result === null`) | The full game composition below |
| `snapshot.result !== null` (or `status === "finished"`) | The result view (doc 02 §4) |
| `status === "expired"` | Friendly `ROOM_EXPIRED` message + way home (doc 02 §5) |
| `snapshot === null` (first load / resuming) | Themed loading state — never a crash (doc 03 §4) |

Coordinate with F1: their lobby (create/join flow) navigates here — into the
game route with a replace-style navigation, so browser Back does not return
to a stale lobby screen. The waiting-room division is FIXED: F1 owns the
`WaitingRoom` view (`src/screens/lobby/waiting-room.tsx`), you mount it
while `status === "waiting"` (their doc 01 §3 documents the view itself).

Leaving: the explicit leave control calls `leaveRoom()` (F3's action), then
navigates home (`router.replace("/")` from `next/navigation` — the lobby
route is F1's; confirm the exact target with them). A page reload or an
accidental Back is NOT a leave: F3's service resumes the session from its
storage, and the screen just renders whatever the store says on mount.
Intercepting the browser Back button mid-game is an open coordination item
with F1 (doc 05 §3).

## 2. Component tree

```
game-route (src/app/game/[roomCode]/page.tsx)   // thin, server component
└── GameScreen                      // game-screen.tsx ("use client") — composition root
    ├── WaitingRoom  (F1's)         // status === "waiting": lobby module's view, mounted here
    ├── GameHeader                  // room code + leave action
    ├── PlayerInfo  (opponent)      // nickname, color, presence dot, active-turn ring
    ├── TurnIndicator               // "Your move" / "Waiting for opponent…" / check
    ├── ChessBoard   (F2's)         // snapshot props + onMoveSubmit → F3 action
    ├── PlayerInfo  (you)           // same component, your seat
    ├── MoveHistory                  // move_history SAN strings, newest visible
    ├── GameActions                  // offer/accept draw · resign · rematch
    ├── ConnectionBanner            // your own connection status
    ├── DrawOfferBanner             // active draw_offer state
    └── GameResultModal / result view // when result !== null
```

Suggested files (yours to adjust inside this folder — keep the names
grep-able so the team can navigate):

```
src/screens/game/
├── README.md                  # doc set index
├── 01…05*.md                  # these docs
├── game-screen.tsx            # composition root ("use client")
├── game-header.tsx
├── player-info.tsx
├── turn-indicator.tsx
├── move-history.tsx
├── game-actions.tsx
├── draw-offer-banner.tsx
├── connection-banner.tsx
├── game-result-modal.tsx
├── result-reasons.ts          # PURE: GameResult → friendly text (doc 02 §4)
├── error-messages.ts           # PURE: ErrorCode → friendly text (doc 02 §5)
└── game-screen.dev.tsx         # dev-only fixture variant (doc 04 §1) — must NOT ship
```

## 3. The layout — responsive, mobile portrait first (RULES.md §2)

One markup serves every viewport. The DOM order IS the mobile order; on
wide screens a grid promotes the panel content into a side column. No fixed
page widths anywhere — `max-w-*`, percentages and `minmax()` only.

Mobile portrait (default — the board sits high, panel content below it):

```
┌──────────────────────────────┐
│ GameHeader: room code, exit  │  bg-surface, border-b border-border
├──────────────────────────────┤
│ PlayerInfo — OPPONENT        │  nickname · color chip · presence dot
├──────────────────────────────┤
│ TurnIndicator                │  "Your move" / "Waiting…" / "Check!"
├──────────────────────────────┤
│                              │
│   ChessBoard (F2)            │  w-full inside the padded column;
│                              │  F2 guarantees square squares
├──────────────────────────────┤
│ PlayerInfo — YOU             │  + active-turn highlight
├──────────────────────────────┤
│ MoveHistory (scroll)         │  SAN pairs, newest visible, bounded max-h
├──────────────────────────────┤
│ GameActions                  │  Draw · Resign (· Rematch when finished)
└──────────────────────────────┘
   Banners sit directly under the header (ConnectionBanner / DrawOfferBanner).
```

Wide screens (`lg:` and up) — board column + side panel:

```
┌──────────────────────────────────────────────────┐
│ GameHeader: room code · leave    (full width)    │
├──────────────────────────────┬───────────────────┤
│ PlayerInfo — OPPONENT        │ MoveHistory       │
│ TurnIndicator                │ (scrolls,         │
│ ChessBoard (F2)              │  bounded)         │
│ PlayerInfo — YOU             │ PlayerInfo — YOU  │
│                              │ GameActions       │
└──────────────────────────────┴───────────────────┘
```

Layout rules:

- Outer container: a plain `<main>`/`<div>` element with
  `className="flex flex-1 flex-col bg-background"` — the root layout
  (`src/app/layout.tsx`) already provides the `bg-background` /
  `text-foreground` body and the column flex shell; you add page padding
  with `p-*` utilities. There is nothing else to mount — no providers.
- One responsive grid, mobile order preserved:

  ```tsx
  <div className="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-4 p-4
                  lg:grid lg:grid-cols-[minmax(0,1fr)_minmax(18rem,22rem)] lg:items-start">
    <section className="flex min-w-0 flex-col gap-3">
      {/* opponent bar · turn indicator · board · your bar (mobile order) */}
    </section>
    <aside className="flex min-w-0 flex-col gap-3">
      {/* history · actions — below the board on narrow, beside it on wide */}
    </aside>
  </div>
  ```

- Board: give it a width — `className="w-full"` inside the padded column;
  F2 makes the grid square (`aspect-square`). On wide screens also bound it
  by height so it never forces page scroll (e.g.
  `max-w-[min(100%,calc(100dvh-12rem))]` — tune the offset). Test on small
  viewports: the history list gets a bounded height (`max-h-*` +
  `overflow-y-auto`) and scrolls; the board never shrinks to unreadable.
- Panels/bars: `bg-surface`, `rounded-lg`, `border` + `border-border` (the
  token utility is `border-border`), text `text-foreground` /
  `text-muted-foreground`. Actions: `bg-primary` /
  `text-primary-foreground`.
- **Never hand-roll controls.** Buttons, dialogs and toasts come from F1's
  `src/components/common/` primitives (RULES.md §1.3 — the app deliberately
  has no component library; F1 builds each primitive once). Import from
  there so the app has one visual language — check F1's doc set for the
  exact names/variants (e.g. a Button with primary/secondary/destructive
  and disabled + loading states). If a variant you need is missing, that's
  a short PR to F1's folder, not a private copy in yours.
- Accessibility is not optional (RULES.md §2): every control is a real,
  keyboard-operable, focus-visible `<button>`-based primitive; the turn
  indicator carries `aria-live="polite"` so turn changes are announced;
  focus management inside dialogs is F1's Dialog primitive's job — do not
  rebuild it.

## 4. Embedding F2's `ChessBoard` — THE prop contract

F2's contract (`src/components/chess/README.md` §4) — wire to these names
exactly; changes go through a PR with F2:

```tsx
interface BoardMove { from: string; to: string; promotion: Promotion | null; }
interface ChessBoardProps {
  snapshot: GameSnapshot;          // whole snapshot from F3's useGameState
  orientation: PlayerColor;        // local player's color (yourColor)
  disabled?: boolean;              // F4's kill-switch (pending/disconnected/finished)
  onMoveSubmit: (move: BoardMove) => void;
}
```

(`GameSnapshot`, `PlayerColor`, `Promotion` come from `@/types/protocol`. The
component is `ChessBoard`; `BoardMove.promotion` is ALWAYS present —
`Promotion | null`, never optional.)

> ⚠️ **Name mismatch to be aware of (flagged):** your original brief (TABLE 5)
> says "`<Board>`" with flat FEN/legal-move props. **F2's frozen contract is
> the component `ChessBoard` taking the whole `snapshot` plus `orientation`,
> reporting moves through the callback `onMoveSubmit`** (the team's planning
> notes' "snapshot + local color" shape) — that is what you import and wire.
> If F2 renames anything, it's a PR, not a silent change.

### 4.1 Snapshot → props mapping (do this in ONE place, not scattered)

`useGameState()` gives you the whole `GameSnapshot` plus `pending` (F3's doc
03 §5). Wire them once:

| F2 prop | Source | Notes |
|---|---|---|
| `snapshot` | `snapshot` from `useGameState()` | verbatim, authoritative — pass it straight through; the board reads `fen`, `legal_moves`, `last_move`, `game_status` from it itself; legality stays server-only |
| `orientation` | `yourColor` from `useRoomState()` | your color at the bottom |
| `disabled` | `pending \|\| turn !== yourColor \|\| connectionLost \|\| gameIsOver` | see doc 03 §5 |
| `onMoveSubmit` | `(move: BoardMove) => submitMove(move.from, move.to, move.promotion)` | straight into F3's action; F3 owns `expected_revision` |

### 4.2 Check highlighting — internal to F2's board

The snapshot has no king-square field, and F2's contract takes the whole
`snapshot`: the board derives the checked king's square itself from
`snapshot.fen` when `game_status === "check"` (F2's doc 02 §3 — board math,
not rules) and renders the check highlight. You do NOT pass (or derive) a
`checkSquare` prop on F4's side — no `check-square.ts` helper is needed.
`game_status: "check"` itself comes from the server; it is all you need for
the "Check!" turn-indicator state (§5).

## 5. Player info bars, turn indicator, history

- **`PlayerInfo`** (reused for both seats): `nickname` (`text-foreground`),
  color label "White"/"Black" (small chip or glyph, e.g. `bg-surface-muted`
  chip with `text-muted-foreground`), and a **presence dot**: `bg-success`
  when `players[seat].connected === true`, muted/destructive when `false`
  (see doc 03 §3 — read `connected` from F3's store, presence events update
  it). **Active-turn indicator:** highlight the bar of the player whose
  `PlayerColor` equals the snapshot's `turn` (e.g. `border-primary` +
  `text-foreground` weight change). Your own bar must be visually
  distinguishable ("You") per your brief §6.
- **`TurnIndicator`**: "Your move" when `turn === yourColor`, "Waiting for
  opponent…" otherwise; "Check!" state when `game_status === "check"`
  (accent with `text-destructive`); disabled/pending variant while
  `pending` is true. Only render the turn indicator while the game is not
  finished — the result view takes over (doc 02 §4).
- **`MoveHistory`**: render `snapshot.move_history` (SAN strings, oldest
  first) in **move-pair rows** ("1. e4 e5") as a semantic `<ol>` — the pairs
  are `<li>`s, accessible by default, no list-numbering tricks. Keep it
  simple and readable: the container is the scroll box
  (`max-h-*` + `overflow-y-auto` on narrow screens; bounded in the side
  panel on wide ones). Newest visible: keep the list in move order and
  scroll to the bottom when it grows (set `scrollTop = scrollHeight` on
  the container in an effect keyed on the snapshot revision) — or render
  newest-first if you prefer, but never mirror the text.
  **Performance note:** a chess game is at most ~150–300 SAN strings —
  simple rows are fine; do not add windowing. The one thing that matters:
  derive the pair rows with `useMemo` keyed on the snapshot revision so
  re-renders don't re-map the array. History must update ONLY from
  accepted server snapshots (your brief §6, RULES.md §4) — never insert
  the pending move locally.

## 6. The screen's data wiring — F3's hooks (exact names)

F3's public API (`src/services/03-stores-and-hooks.md` §5 — names frozen
with F1/F4; **these are the names you wire to**):

```ts
const { submitMove, offerDraw, acceptDraw, resign, rematch, leaveRoom } = useRoomActions();
// also: createRoom, joinRoom — lobby-side (F1's); you use the game ones.
// resume is NOT here — it is service-internal (F3 sends session.resume on
// connect/reconnect); you never call it.
const { players, status, yourColor } = useRoomState();
// (useRoomState() also returns roomCode and a connection slice —
//  roomCode duplicates the URL param; see the note below)
const { snapshot, pending } = useGameState();      // snapshot = the whole GameSnapshot
const { status: connectionStatus } = useConnection();
// connectionStatus: "connecting" | "connected" | "reconnecting" | "disconnected"
```

> ⚠️ **Name mismatches (flagged, use the list above):**
> - Your original brief (TABLE 6) proposes `makeMove(…)` and
>   `requestRematch()` — F3's frozen API is **`submitMove(…)`** and
>   **`rematch()`**.
> - Planning notes also mention `useGameSnapshot`, `useRoom`,
>   `useConnectionStatus`, `useSendCommand` — F3's doc 03 §5 actually
>   freezes **`useGameState`, `useRoomState`, `useRoomActions`,
>   `useConnection`**. Do not invent alternatives; if F3 ships different
>   final names, update this doc + your wiring in the same PR.
> - `useConnection()` returns a slice (`{ status, attempts, lastError }`)
>   per F3's doc 03 §5 — destructure `status` (aliased to
>   `connectionStatus` above) rather than comparing the hook result to a
>   string. `useRoomState().connection` carries the same status too — pick
>   one read for the screen and be consistent.
> - `src/hooks/` does not exist yet (only F3's docs) — build against the
>   fixtures + the dev-only fixture variant (doc 04 §1) until it does.

Rules for using them (RULES.md §4, F3's doc 03):

- Screens **never** call the WebSocket service / stores directly — hooks
  only. And never an API route, server action or `fetch`: the Rust WS
  server is the only data source (RULES.md §2).
- F3's actions own `expected_revision` and command IDs. `submitMove(from, to,
  promotion)` — pass the user's choice through, including `null` promotions;
  `BoardMove.promotion` is always present (`Promotion | null`), never
  omitted.
- Subscribe with **narrow selectors** (e.g. `useGameState((s) => s.pending)`),
  not whole-store reads, so the board re-renders only when its inputs change.
- `players[].connected` in the STORE is fresher than the last full snapshot
  when a `player.presence` event just arrived — always read through F3's
  hooks, never cache players in screen state (doc 03 §3).

## 7. Skeleton with TODOs — start here, fill it in

```tsx
// src/screens/game/game-screen.tsx — SKELETON (F4: implement)
"use client";

import { useParams, useRouter } from "next/navigation";

import { ChessBoard } from "@/components/chess/chess-board"; // F2 — verify the component name & file path against their README §4

import { useRoomActions } from "@/hooks/room-actions"; // TODO: F3's real paths — see doc 01 §6 before finalizing
import { useRoomState } from "@/hooks/room-state";
import { useGameState } from "@/hooks/game-state";
import { useConnection } from "@/hooks/connection";

import { ConnectionBanner } from "./connection-banner";
import { DrawOfferBanner } from "./draw-offer-banner";
import { GameActions } from "./game-actions";
import { GameHeader } from "./game-header";
import { GameResultModal } from "./game-result-modal";
import { MoveHistory } from "./move-history";
import { PlayerInfo } from "./player-info";
import { TurnIndicator } from "./turn-indicator";
import { WaitingRoom } from "@/screens/lobby/waiting-room";

export function GameScreen() {
  const { roomCode: urlRoomCode } = useParams<{ roomCode: string }>();
  const router = useRouter();
  const { submitMove, offerDraw, acceptDraw, resign, rematch, leaveRoom } = useRoomActions();
  const { players, status, yourColor } = useRoomState();
  const { snapshot, pending } = useGameState();
  const { status: connectionStatus } = useConnection();

  // TODO: null-safety while the first snapshot loads (snapshot may be null
  //       on entry/resume — render a themed loading state, not a crash).
  // TODO: identify seats: opponent = players.find(p => p.color !== yourColor)
  // TODO: gameIsOver = snapshot.result !== null
  // TODO: boardDisabled = pending || snapshot.turn !== yourColor
  //                            || connectionStatus !== "connected" || gameIsOver
  // TODO: presence/turn wiring into PlayerInfo (doc 01 §5)
  // TODO: banners: DrawOfferBanner when snapshot.draw_offer !== null and
  //       it's the opponent's; ConnectionBanner from connectionStatus (doc 03)
  // TODO: result view when gameIsOver (doc 02 §4)
  // TODO: leave = leaveRoom() then router.replace("/") — target route agreed with F1

  return (
    <main className="flex flex-1 flex-col bg-background">
      {/* TODO: responsive shell (doc 01 §3): WaitingRoom while status is
          "waiting"; otherwise GameHeader, banners, board column (opponent,
          TurnIndicator, ChessBoard, you) + panel column (history, actions) */}
    </main>
  );
}
```

Every `TODO` above is expanded into a full spec in docs 01–03; nothing here
is final — it exists so typecheck has a shape to verify early.
