# 01 — Board architecture (HOW to build the board)

You build: the grid, the FEN adapter, orientation, square colors, sizing.
Style with uniwind `className` only (RULES.md §1.1); colors from `global.css`
tokens only (RULES.md §1.3). TypeScript is `strict` — no `any` in exported
APIs.

## 1. Component tree — recommended: Board → 8 rows → 8 Squares

Two viable shapes:

- **(A) 8 row `View`s, each with 8 `Square`s (RECOMMENDED)**
- (B) one flat 64-`Square` list in a wrapping container

Choose **(A)**. Reasons:

- **Guaranteed equal squares with zero measurement.** The board fixes its
  outer size (`aspect-square w-full`, §5); each row is `flex-1` of the board
  height; each square is `flex-1` of the row width. 8 rows × 8 equal columns
  follow from flex alone — no `onLayout`, no math, no rounding drift.
- Flat `flex-wrap` (B) depends on wrap math and minimum touch-size
  interactions; it can produce off-by-one-subpixel rows and unequal children
  once borders/overlays enter. It buys nothing here.
- (B)'s only advantage is virtualization for huge lists — irrelevant at 64
  children (§6).

```
ChessBoard                       // aspect-square w-full — owns interaction state
├── <View> × 8                   // one per rank, className="flex-1 flex-row"
│   └── <Square /> × 8           // flex-1, keyed by square name "a1".."h8"
│        ├── square bg            // bg-board-light / bg-board-dark
│        ├── highlight overlays   // last-move / move-target / check / selected
│        └── <Piece />           // 02 §1 — your asset strategy, one file
└── <PromotionChooser />         // modal — only when a promotion is pending
```

`Square` is the only touchable thing (`Pressable`); `Piece` is inert
rendering (until the optional drag stretch goal, 02 §6).

Suggested file layout (yours to adjust — keep names stable once frozen):

```
src/components/chess/
├── README.md               // this doc set — keep §4 = the contract
├── 01-board-architecture.md
├── 02-pieces-and-interaction.md
├── 03-manual-testing.md
├── 04-acceptance.md
├── chess-board.tsx          // the grid + interaction state machine (02 §2)
├── square.tsx              // one memoized Pressable square + overlays
├── piece.tsx               // one piece — the ONLY file that knows the asset
├── fen.ts                  // PURE: fenToPieces(fen) + Square/Piece types
├── orientation.ts          // PURE: display order for white/black-at-bottom
└── promotion-chooser.tsx   // Queen/Rook/Bishop/Knight modal
```

## 2. The FEN adapter — pure `fenToPieces(fen: string)`

F4 hands you `snapshot.fen` (full FEN, e.g. from `sampleSnapshot`:

```
"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
```

A FEN has 6 space-separated fields; **you only ever parse field 0** (piece
placement). Turn it into a lookup the grid can render: a `Map<Square, Piece>`
where `Square` is the algebraic name `"a1".."h8"`.

### The exact algorithm (this is board math — reading a position — NOT chess
rules; that's why it is allowed. YOU write the code.)

1. `const placement = fen.split(" ")[0];` — the piece-placement field, e.g.
   `"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR"`.
2. `const ranks = placement.split("/");` — must yield **exactly 8 strings**.
   FEN lists **rank 8 first**: `ranks[0]` is rank 8 (top from White's view),
   `ranks[7]` is rank 1.
3. For rank index `i` (0-based into `ranks`), the rank **number** is
   `8 - i` (so `i = 0` → rank `8`, `i = 7` → rank `1`).
4. Walk each rank string left→right while tracking a 0-based **file index**
   `f` (0 = file `"a"`):
   - **Digit** `"1"`..`"8"`: that many consecutive empty squares — advance
     `f` by the digit's value; emit nothing.
   - **Letter** (`PNBRQKpnbrqk`): a piece on the square named
     `"abcdefgh"[f] + rankNumber` (e.g. `f = 4`, rank `1` → `"e1"`). Uppercase
     = White, lowercase = Black. The letter lowercased is the piece type.
     Then `f += 1`.
5. Defensive guard (the server is trusted, but a parser bug must never crash
   the UI): if `ranks.length !== 8`, or any rank string consumes a file
   count ≠ 8, or an unexpected character appears, log once and return an
   empty `Map` (or the last good parse) — never throw from render.
6. Return the `Map<Square, Piece>` containing **only occupied squares**
   (32 entries in the starting position). `snapshot.fen` is your single
   input; turn is NOT read from the FEN (use `snapshot.turn`).

Ignored on purpose: fields 1–5 (side to move, castling rights, en-passant
square, halfmove clock, fullmove number). `snapshot.turn` already carries the
side to move; castling/en-passant legality is the server's job and is
already encoded inside `snapshot.legal_moves` (e.g. `"e1g1"`, `"e5d6"`) —
that is exactly why this adapter stays pure rendering math and never becomes
a rules engine.

### Typed skeleton (TODOs are yours)

```ts
// src/components/chess/fen.ts
// PURE module: no React, no state, no side effects.

export type PieceType = "p" | "n" | "b" | "r" | "q" | "k";
export type PieceColor = "white" | "black";
export interface Piece {
  type: PieceType;
  color: PieceColor;
}
/** Algebraic square name, "a1".."h8" — file letter + rank digit. */
export type Square = string;

const FILES = "abcdefgh" as const;
const PIECE_TYPES = "pnbrqk" as const;

/**
 * Parse the piece-placement field of a FEN into occupied squares only.
 * Pure rendering math — NEVER validates move legality (RULES.md §4).
 * Algorithm: README doc 01 §2 (steps 1–6). TODO: implement.
 */
export function fenToPieces(fen: string): Map<Square, Piece> {
  // TODO 1: placement = fen.split(" ")[0]
  // TODO 2: ranks = placement.split("/") — guard length 8
  // TODO 3: for each rank index i, rankNumber = 8 - i
  // TODO 4: walk chars — digit => skip files; letter => map square
  //         squareName = FILES[f] + rankNumber
  //         color = uppercase ? "white" : "black"
  //         type   = char.toLowerCase() validated against PIECE_TYPES
  // TODO 5: guard each rank consumes exactly 8 files; never throw
  // TODO 6: return the Map
  return new Map(); // placeholder — replace
}
```

Memoize at the call site so a re-render with the same position is free:

```tsx
const pieces = useMemo(() => fenToPieces(snapshot.fen), [snapshot.fen]);
```

## 3. Orientation (who sits at the bottom)

The **local player's color** arrives via the `orientation` prop — F4 gets it
from F3's session state (the `color` from `session.ready`, surfaced as
`useRoomState().yourColor` per F3's `src/services/README.md` §7). The board
does NOT derive it: `snapshot.players` says who plays which color, but only
F3's session knows which of them is holding this device.

Display order (pure helpers in `orientation.ts`):

| Orientation | Rows (top→bottom) | Files (left→right) | Bottom-left square | Top-right square |
|---|---|---|---|---|
| `"white"` | rank 8 → rank 1 | `a` → `h` | `a1` | `h8` |
| `"black"` | rank 1 → rank 8 | `h` → `a` | `h8` | `a1` |

Both orientations = the same board rotated 180°: for `"black"`, `a1` sits at
the **top-right** (each square keeps its name; only its display position
moves).

```ts
// src/components/chess/orientation.ts — PURE.
export type Orientation = "white" | "black"; // or import PlayerColor from "@/types/protocol"

/**
 * Square name at display row r (0 = top) and column c (0 = left).
 * white: rank = 8 - r, file = a + c.  black: rank = r + 1, file = h - c.
 * TODO: implement + keep it a total function for r, c in 0..7.
 */
export function squareAt(r: number, c: number, orientation: Orientation): Square {
  // TODO
  throw new Error("TODO");
}
```

Key invariants (your manual tests in `03-manual-testing.md` check them):

- Every `Square` is keyed by its **square name**, never by row/column index —
  when the board flips, the same 64 keys land in different positions and
  taps keep reporting the correct square.
- Square **colors** are computed from the square name (§4), so flipping the
  view never flips light/dark squares.
- Highlights are looked up by square name too (`last_move.from`,
  legal-move destinations), so orientation code and highlight code never
  interact.

## 4. Square coloring — parity + tokens

With 0-based indices of the square **name** — file `f` (`a=0 … h=7`) and
rank `r` (`1=0 … 8=7`):

- `f + r` **even** → `className="bg-board-dark"` (a1, c1, e1, … b2, …)
- `f + r` **odd** → `className="bg-board-light"` (h1, a8, …)

Check against a real board: `a1` is a dark square, `a8` and `h1` are light.
Compute parity from the square NAME (not the display position) — that is
what makes orientation-independent colors automatic.

Tokens only, from `frontend/global.css`: `board-light`, `board-dark`,
`move-target`, `last-move`, `check` (utilities `bg-board-light`,
`bg-board-dark`, `bg-move-target`, `bg-last-move`, `bg-check`). Dark-mode
values for every token are defined in `global.css` too and switch
automatically — you do zero extra work, and you never write a color literal
or `bg-[#…]` (ESLint bans it).

## 5. Sizing — square squares that scale

```tsx
// chess-board.tsx
<View className="aspect-square w-full">     {/* width from parent, height = width */}
  {/* 8 ×  <View className="flex-1 flex-row">  containing 8 × <Square className="flex-1" /> */}
</View>
```

- The board takes **full width** from F4's game screen (F4's README: "board
  scales to width — give it a width") and `aspect-square` derives a square
  height. Never a fixed pixel size, never a stretched rectangle.
- Rows and squares share the space with `flex-1` (§1) — no per-square size
  math anywhere.
- Safe areas are NOT the board's concern: the game screen wraps content in
  `StyledSafeAreaView` / insets. The board itself must not add padding hacks.
- Small-phone sanity is part of your checklist (`03-manual-testing.md` §I):
  at ~375 dp width the full board fits with no clipping.

## 6. Rendering performance — plain Views, memo, stable keys

- **No FlatList.** FlatList is for long/unknown-length scrolling lists. A
  fixed 64-cell grid is ~64 small views — virtualization adds overhead,
  breaks the flex grid from §1, and buys nothing. Use plain `View`/`Pressable`.
- **`React.memo` on `Square`.** Squares re-render only when their own
  primitive props change. For that, pass **primitives** down (square name,
  piece type + color or `null`, booleans like `isTarget` / `isLastMove` /
  `isCheck` / `isSelected`) — never a freshly created object/array per
  render, or memo never helps.
- **Stable keys:** `key={squareName}` (`"a1"…"h8"`), never the array index —
  an index key would survive an orientation flip while its square changed
  meaning, which React resolves by mislabeling state.
- Memoize the FEN parse on `snapshot.fen` (§2) and derive per-square props in
  one `useMemo` per concern (e.g. a `Set` of target squares — see 02 §2).
- Keep `Piece` dumb and memoized too; the expensive thing in this module is
  nothing at 60fps — snapshots arrive one move at a time (seconds apart), so
  this budget is generous. Do not add `windowing`, `VirtualizedList`,
  `shouldComponentUpdate` class hacks, or manual measure code.

## 7. What good looks like at the end of this stage

- `fen.ts` and `orientation.ts` are pure modules with no React imports.
- Feeding `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`
  renders 32 pieces in the right places (harness: `03-manual-testing.md`).
- Flipping `orientation` rotates the board; taps still report correct
  square names (log them).
- `npm run typecheck` and `npm run lint` are clean.

Next: `02-pieces-and-interaction.md` — pieces and the tap-to-move machine.
