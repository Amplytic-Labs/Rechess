# `src/components/chess/` — Owner: **F2** (Chessboard and Interaction Components)

> **Read `frontend/RULES.md` first** — it is binding (no StyleSheet, no
> expo-status-bar, everything themed, ownership map). Then read this file
> completely before writing code.

---

## 1. Your mission

Deliver the reusable 8×8 **chessboard** component family: squares, pieces,
selection, move highlighting and the pawn-promotion chooser. F4 embeds your
board in the game screen; F3 feeds it the authoritative FEN and legal moves.

You are **not** implementing chess rules ("is this move legal?") — the server
is the only authority. You render what the server-provided FEN says and
highlight what the server-provided legal-move list allows.

## 2. What you own / what you don't touch

| You own | You do NOT touch |
|---|---|
| `src/components/chess/**` | `src/screens/**` (F1 lobby, F4 game) |
| The FEN → squares adapter for rendering | `src/services/`, `src/store/` (F3) — you consume their props |
| Board orientation/flipping helpers | `src/types/protocol.ts` (F3) |
| Piece asset strategy + rendering | Rules logic — never a client-side legality engine |
| Promotion chooser UI | |

## 3. The base that already exists (don't rebuild it)

- NativeWind tokens for your board: **`board-light`, `board-dark`,
  `move-target`, `last-move`, `check`** (defined in `tailwind.config.js`,
  light+dark values in `src/theme/ThemeProvider.tsx`). Use ONLY these for
  board colors — the board must work in dark mode with zero extra work.
- `react-native-gesture-handler`, `react-native-reanimated` (installed;
  `GestureHandlerRootView` already wraps the app), `react-native-safe-area-context`.
- `expo-image` (installed) if you go with raster piece assets.

## 4. Component contract (agree with F3 + F4 on Day 1, then freeze)

```tsx
// Proposed props — finalize with F3/F4, then keep stable.
type ChessBoardProps = {
  fen: string;                       // authoritative position (from F3 store)
  orientation: "white" | "black";    // local player at the bottom
  legalMoves: string[];               // UCI-style from server, e.g. ["e2e4", "e7e8q"]
  lastMove: { from: string; to: string } | null;
  checkSquare: string | null;         // e.g. "e1" when in check (derive from FEN/status)
  disabled: boolean;                  // not your turn / disconnected / pending
  onMove: (move: { from: string; to: string; promotion?: "q" | "r" | "b" | "n" }) => void;
};
```

Suggested file layout (yours to adjust):

```
src/components/chess/
├── chess-board.tsx        // the 8×8 grid; composes everything below
├── square.tsx             // one Pressable square with highlight states
├── piece.tsx              // one of 12 piece types (asset strategy lives here)
├── fen-board.ts           // PURE: fen → 64-square array (rendering only!)
├── orientation.ts         // PURE: square mapping for white/black-at-bottom
├── promotion-chooser.tsx  // modal: Q/R/B/N with clear labels
└── chess-board.test.tsx   // component tests with FEN fixtures
```

## 5. Behaviour requirements (your brief)

- Board renders from `fen` — parse it with your **pure** `fen-board.ts`
  adapter (parsing for *rendering* only; it must never validate legality).
- All 12 piece types render reliably on iOS + Android. Pick ONE asset
  strategy (vector/SVG via react-native-svg — needs team PR per RULES.md §2 —
  or glyph fonts, or raster assets via `expo-image`) and document your choice.
- Orientation: when `orientation === "black"`, rank/file mapping flips; a1
  must be top-right. Write `orientation.ts` as a pure function and test it.
- Interaction flow: tap own piece → select + show its legal destinations
  (filter `legalMoves` by `from`); tap highlighted square → `onMove`;
  tap promotion square → show chooser → `onMove` with the chosen piece;
  tap elsewhere → clear selection. **Never mutate the displayed position
  locally** — the board updates only when `fen` changes from the server.
- Highlights are visually distinct and never obscure pieces: selected
  square, legal destinations (`move-target`), last move (`last-move`),
  check (`check`), disabled/pending state.
- Squares stay perfectly square and the board scales to available width
  (`aspect-square`, one dimension from parent, `flex-1` rows/cols) — test on
  small phones; no clipping, no stretching.
- When `fen`/revision changes (server update, reconnect, resume): clear
  selection, pending promotion and any local interaction state.

## 6. How to test your module independently

```bash
cd frontend
npm run typecheck && npm run lint
npx jest chess
```

- Fixtures as FEN constants (initial position, castling position, en-passant
  position, promotion-ready position, midgame) — assert `fen-board.ts`
  produces the right 64 squares and `orientation.ts` maps squares correctly
  both ways.
- Component tests: render `ChessBoard` with each fixture; assert 64 squares,
  correct piece counts, selection + highlight behavior, promotion chooser
  flow, `onMove` payloads, disabled state.
- No sockets, no stores needed — the board is pure props in/props out
  (that's exactly why the contract in §4 is frozen before you start).

## 7. Deliverables (definition of done)

- [ ] `ChessBoard` + subcomponents rendering all pieces from FEN, both orientations
- [ ] Selection, legal-destination highlights, last-move, check, disabled states
- [ ] Promotion chooser (accessible, labeled)
- [ ] Pure, unit-tested FEN + orientation adapters
- [ ] Component tests on the fixture positions
- [ ] Documented props + usage example in this README (update §4 with final names)

## 8. Key coordination

- **F3**: where FEN/legal moves come from and their exact encoding (see
  `src/types/protocol.ts` and `docs/protocol-v1.md` — `legal_moves` is
  UCI-style `"e2e4"` / `"e7e8q"`).
- **F4**: they embed your board — prop names are a contract; changes via PR.
- Asset choice (SVG vs font vs raster) touches `package.json` → team PR.
