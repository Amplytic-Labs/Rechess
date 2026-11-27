# 01 — Screen composition and data wiring

> Everything in this doc lives under your ownership:
> `src/screens/game/**` + the thin route file `src/app/game.tsx`.

## 1. The route file — `src/app/game.tsx` (thin, RULES.md §3)

Route files render screen components; real UI lives in screens. The screen
receives **only non-secret** references: the room code at most —
`player_token` NEVER (it stays in F3's secure storage).

```tsx
// src/app/game.tsx — THIN. No logic, no data, no styles beyond layout.
import { useLocalSearchParams } from "expo-router";

import { GameScreen } from "@/screens/game/game-screen";

export default function GameRoute() {
  // Room code only — never a token (RULES.md §3).
  const { roomCode } = useLocalSearchParams<{ roomCode: string }>();
  return <GameScreen roomCode={roomCode} />;
}
```

Coordinate with F1: their waiting room navigates here once both players are
present (F1's README §4), driven by F3's room state — never a local timer.

## 2. Component tree

```
game-route (src/app/game.tsx)
└── GameScreen                      // game-screen.tsx — composition root
    ├── GameHeader                  // room code + leave action
    ├── PlayerInfo  (opponent)      // nickname, color, presence dot, active-turn ring
    ├── TurnIndicator               // "Your move" / "Waiting for opponent…" / check
    ├── ChessBoard   (F2's)         // snapshot props + onMove → F3 action
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
├── game-screen.tsx            # composition root
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
├── check-square.ts             # PURE: fen → king square (rendering only, §4.2)
└── game-screen.dev.tsx         # dev-only fixture variant (doc 04 §1) — must NOT ship
```

## 3. The layout (top to bottom, common phones)

```
┌────────────────────────────┐
│ GameHeader: room code, exit │  bg-surface, border-b border-border
├────────────────────────────┤
│ PlayerInfo — OPPONENT      │  nickname · color chip · presence dot
├────────────────────────────┤
│ TurnIndicator              │  "Your move" / "Waiting…" / "Check!"
├────────────────────────────┤
│                            │
│   ChessBoard (F2)          │  width = screen width − padding;
│                            │  F2 guarantees square squares
├────────────────────────────┤
│ PlayerInfo — YOU           │  + active-turn highlight
├────────────────────────────┤
│ MoveHistory (scroll)       │  SAN pairs, newest visible, max height
├────────────────────────────┤
│ GameActions                │  Draw · Resign (· Rematch when finished)
└────────────────────────────┘
   Banners overlay the top (ConnectionBanner / DrawOfferBanner).
```

Layout rules:

- Outer container: `StyledSafeAreaView` from `@/components/common/styled.ts`
  (or `useSafeAreaInsets`), `className="flex-1 bg-background"` — never padding
  hacks (RULES.md §2).
- Board: give it a width — `className="w-full"` inside a padded container;
  F2 makes the grid square (`aspect-square`). Test on small phones: history
  list gets a bounded height (`max-h-*`) and scrolls; the board never shrinks
  to unreadable.
- Panels/bars: `bg-surface`, `rounded-lg`, `border` + `border-border` (the
  token utility is `border-border`), text `text-foreground` /
  `text-muted-foreground`. Actions: `bg-primary` / `text-primary-foreground`.
- Prefer HeroUI components (`heroui-native/button`, `heroui-native/card`)
  over hand-rolled controls (RULES.md §2); if F1's
  `src/components/common/app-button` exists with the variants you need
  (primary/secondary/destructive, disabled + loading), prefer it so the app
  has one visual language — coordinate in PRs, it's F1's folder.

## 4. Embedding F2's `ChessBoard` — THE prop contract

F2's contract (`src/components/chess/README.md` §4) — wire to these names
exactly; changes go through a PR with F2:

```tsx
type ChessBoardProps = {
  fen: string;                       // authoritative position (from F3 store)
  orientation: "white" | "black";    // local player at the bottom
  legalMoves: string[];              // UCI-style from server, e.g. ["e2e4", "e7e8q"]
  lastMove: { from: string; to: string } | null;
  checkSquare: string | null;        // e.g. "e1" when in check (derive from FEN/status)
  disabled: boolean;                 // not your turn / disconnected / pending
  onMove: (move: { from: string; to: string; promotion?: "q" | "r" | "b" | "n" }) => void;
};
```

> ⚠️ **Name mismatch to be aware of (flagged):** your original brief (TABLE 5)
> says "`<Board>`" and the team's planning notes say `onMoveSubmit` /
> "snapshot + local color". **F2's frozen contract is the component
> `ChessBoard` with the callback prop `onMove`** — that is what you import
> and wire. If F2 renames anything, it's a PR, not a silent change.

### 4.1 Snapshot → props mapping (do this in ONE helper, not scattered)

`useGameState()` gives you the current `GameSnapshot` fields plus `pending`
(F3's README §6/§7). Map them once:

| F2 prop | Source | Notes |
|---|---|---|
| `fen` | `snapshot.fen` | verbatim, authoritative |
| `orientation` | `yourColor` from `useRoomState()` | your color at the bottom |
| `legalMoves` | `snapshot.legal_moves` | UCI-style, server only — never filter/compute legality yourself |
| `lastMove` | `snapshot.last_move ? { from: snapshot.last_move.from, to: snapshot.last_move.to } : null` | F2 wants `{from,to}`; drop the `san` field |
| `checkSquare` | derived: when `snapshot.game_status === "check"`, the side-to-move's king square parsed from `fen` | see §4.2 — rendering support only |
| `disabled` | `pending \|\| turn !== yourColor \|\| connectionLost \|\| gameIsOver` | see doc 03 §5 |
| `onMove` | `{ from, to, promotion? } => sendMove(from, to, promotion ?? null)` | straight into F3's action; F3 owns `expected_revision` |

### 4.2 `checkSquare` — pure rendering derivation (NOT rule validation)

The snapshot has no king-square field; F2's contract asks you to derive it.
Write a tiny PURE helper `check-square.ts`: when
`game_status === "check"`, parse `fen`'s board part and return the square of
the king whose turn it is (e.g. `"e1"`), else `null`. This is parsing for
**rendering only** (same category as F2's own `fen-board.ts`) — it must never
decide legality, results, or turn. `game_status: "check"` itself comes from
the server.

## 5. Player info bars, turn indicator, history

- **`PlayerInfo`** (reused for both seats): `nickname` (`text-foreground`),
  color label "White"/"Black" (small chip or glyph, e.g. `bg-surface-muted`
  chip with `text-muted-foreground`), and a **presence dot**: `bg-success`
  when `players[seat].connected === true`, muted/destructive when `false`
  (see doc 03 §4 — read `connected` from F3's store, presence events update
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
  first) in **move-pair rows** ("1. e4 e5"). Keep it simple and readable:
  a `FlatList` (accepts `className` and `contentContainerClassName`,
  RULES.md §2) or a `ScrollView`. Newest visible: either `inverted` on a
  FlatList or `data = [...move_history].reverse()` — with `inverted`, rows
  themselves must be re-inverted so the text isn't mirrored.
  **Performance note:** a chess game is at most ~150–300 SAN strings —
  simple rows are fine; do NOT pre-measure or virtualize aggressively. The
  one thing that matters: derive the list with `useMemo` keyed on the
  snapshot revision so every keystroke-free re-render doesn't re-map the
  array. History must update ONLY from accepted server snapshots
  (your brief §6, RULES.md §4) — never insert the pending move locally.

## 6. The screen's data wiring — F3's hooks (exact names)

F3's public API (`src/services/README.md` §7 — names proposed to be frozen
with F1/F4; **these are the names you wire to**):

```ts
const { sendMove, offerDraw, acceptDraw, resign, rematch, leaveRoom } = useRoomActions();
// also: createRoom, joinRoom, resumeRoom — lobby-side (F1's); you use the game ones
const { roomCode, players, status, drawOffer, yourColor } = useRoomState();
const { snapshot, pending } = useGameState();      // snapshot = current GameSnapshot fields
const connection = useConnection();                // "connecting" | "connected" | "reconnecting" | "disconnected"
```

> ⚠️ **Name mismatches (flagged, use the list above):**
> - Your original brief (TABLE 6) proposes `makeMove(…)` and
>   `requestRematch()` — F3's proposed API is **`sendMove(…)`** and
>   **`rematch()`**.
> - Planning notes also mention `useGameSnapshot`, `useRoom`,
>   `useConnectionStatus`, `useSendCommand` — F3's README §7 actually
>   proposes **`useGameState`, `useRoomState`, `useRoomActions`,
>   `useConnection`**. Do not invent alternatives; if F3 ships different
>   final names, update this doc + your wiring in the same PR.
> - `src/hooks/` does not exist yet (only F3's README) — build against the
>   fixtures + a typed local mock (doc 04 §1) until it does.

Rules for using them (RULES.md §4, F3's README §5–6):

- Screens **never** call `wsService` / stores directly — hooks only.
- F3's actions own `expected_revision` and command IDs. `sendMove(from, to,
  promotion ?? null)` — pass the user's choice through, including `null`
  promotions; F2 may omit `promotion` when not promoting.
- Subscribe with **narrow selectors** (e.g. `useGameState((s) => s.pending)`),
  not whole-store reads, so the board re-renders only when its inputs change.
- `players[].connected` in the STORE is fresher than the last full snapshot
  when a `player.presence` event just arrived — always read through F3's
  hooks, never cache players in screen state (doc 03 §4).

## 7. Skeleton with TODOs — start here, fill it in

```tsx
// src/screens/game/game-screen.tsx — SKELETON (F4: implement)
import { ChessBoard } from "@/components/chess/chess-board"; // F2 — verify export name & path against their README §4
import { StyledSafeAreaView } from "@/components/common/styled";

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

export function GameScreen({ roomCode }: { roomCode: string }) {
  const { sendMove, offerDraw, acceptDraw, resign, rematch, leaveRoom } = useRoomActions();
  const { players, yourColor } = useRoomState();
  const { snapshot, pending } = useGameState();
  const connection = useConnection();

  // TODO: null-safety while the first snapshot loads (snapshot may be null
  //       on entry/resume — render a themed loading state, not a crash).
  // TODO: identify seats: opponent = players.find(p => p.color !== yourColor)
  // TODO: gameIsOver = snapshot.result !== null
  // TODO: boardDisabled = pending || snapshot.turn !== yourColor
  //                            || connection === "disconnected" || gameIsOver
  // TODO: checkSquare derivation (doc 01 §4.2)
  // TODO: presence/turn wiring into PlayerInfo (doc 01 §5)
  // TODO: banners: DrawOfferBanner when snapshot.draw_offer !== null and
  //       it's the opponent's; ConnectionBanner from connection (doc 03)
  // TODO: result view when gameIsOver (doc 02 §4)

  return (
    <StyledSafeAreaView className="flex-1 bg-background">
      {/* TODO: GameHeader, PlayerInfo(opponent), TurnIndicator, ChessBoard,
          PlayerInfo(you), MoveHistory, GameActions, banners, result modal */}
    </StyledSafeAreaView>
  );
}
```

Every `TODO` above is expanded into a full spec in docs 01–03; nothing here
is final — it exists so typecheck has a shape to verify early.
