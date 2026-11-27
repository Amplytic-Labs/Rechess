# 01 — Board architecture (HOW to build the board)

You build: the grid, the FEN adapter, orientation, square colors, sizing, the
keyboard/focus model. Style with Tailwind `className` only (RULES.md §1.1);
colors from `global.css` tokens only (RULES.md §1.2). TypeScript is `strict`
— no `any` in the module's public APIs. The board is a DOM **client component**: put
`"use client"` at the top of every component file in this module (the pure
`.ts` modules need no directive).

## 1. Component tree — a CSS grid of 64 square elements

One container with `display:grid` and 64 square children — **no row wrappers
needed**; the grid makes equal cells:

```
ChessBoard ("use client")        // <div role="group" aria-label="Chess board">
                                 //   className="grid aspect-square w-full grid-cols-8 grid-rows-8"
├── <Square /> × 64              // a <button> — focusable (roving tabindex, §6),
│   │                            //   keyed by square name "a1".."h8"
│   ├── square bg                //   bg-board-light / bg-board-dark
│   ├── highlight overlays       //   last-move / move-target / check / selected
│   └── <Piece />                // 02 §1 — your asset strategy, one file
└── <PromotionChooser />         // small dialog overlay — only while a promotion is pending
```

Why a CSS grid (and not 8 flex-row wrappers, and not one wrapping flex list):

- **Guaranteed equal squares with zero measurement.** `grid-cols-8
  grid-rows-8` inside an `aspect-square w-full` container yields 64
  exactly-equal cells — no math, no rounding drift, no layout code.
- Eight flex-row wrappers would also work, but buy nothing in the DOM: the
  grid handles equal sizing natively, and fewer wrappers mean simpler
  keyboard traversal (§6).
- A flex-wrap list depends on wrap math and can produce off-by-one rows once
  borders/overlays enter. It buys nothing at 64 cells (§7).

`Square` is the only interactive element (a `<button>`); `Piece` is inert
rendering (until the optional drag enhancement, 02 §6).

Suggested file layout (yours to adjust — keep names stable once frozen):

```
src/components/chess/
├── README.md               // this doc set — keep §4 = the contract
├── 01-board-architecture.md
├── 02-pieces-and-interaction.md
├── 03-manual-testing.md
├── 04-acceptance.md
├── chess-board.tsx          // "use client" — grid + interaction state machine (02 §2)
├── square.tsx               // "use client" — one memoized square button + overlays
├── piece.tsx                // "use client" — one piece; the ONLY file that knows the asset
├── promotion-chooser.tsx    // "use client" — Queen/Rook/Bishop/Knight dialog overlay
├── fen.ts                   // PURE: fenToPieces + checkedKingSquare + types
├── fen.test.ts              // vitest sibling (node env — pure logic, no DOM)
├── orientation.ts           // PURE: display order for white/black-at-bottom
├── orientation.test.ts      // vitest sibling
├── move-targets.ts          // PURE: legal_moves → target set / promotion flag (02 §2)
└── move-targets.test.ts     // vitest sibling
```

The `.test.ts` siblings are part of the module's quality gates (RULES.md
§5): vitest runs in a **node environment**, so those suites call functions
only and never render anything (03 §2).

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
   FEN lists **rank 8 first**: `ranks[0]` is rank 8 (top from White's
   perspective), `ranks[7]` is rank 1.
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
// Unit tests: fen.test.ts (03 §2) — node environment, no DOM.

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
 * Algorithm: this doc §2 (steps 1–6). TODO: implement.
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

/**
 * Square of the given side's king, or null when absent.
 * Pure position reading — used ONLY to place the `bg-check` highlight
 * when the server already said game_status === "check" (02 §3).
 * It never decides whether it IS check. TODO: implement.
 */
export function checkedKingSquare(
  pieces: Map<Square, Piece>,
  color: PieceColor,
): Square | null {
  // TODO: scan the map for { type: "k", color } and return its square
  return null; // placeholder — replace
}
```

Memoize at the call site so a re-render with the same position is free:

```tsx
const pieces = useMemo(() => fenToPieces(snapshot.fen), [snapshot.fen]);
```

## 3. Orientation (who sits at the bottom)

The **local player's color** arrives via the `orientation` prop — F4 gets it
from F3's session state (the `color` from `session.ready`, surfaced as
`useRoomState().yourColor` per F3's `src/services/03-stores-and-hooks.md`
§5). The board does NOT derive it: `snapshot.players` says who plays which
color, but only F3's session knows which of them is holding this device.

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
// Unit tests: orientation.test.ts (03 §2).
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

Key invariants (your tests and manual checks in `03-manual-testing.md` pin
them):

- Every `Square` is keyed by its **square name**, never by row/column index —
  when the board flips, the same 64 keys land in different positions and
  clicks keep reporting the correct square.
- Square **colors** are computed from the square name (§4), so flipping the
  board never flips light/dark squares.
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
automatically with the system theme (`prefers-color-scheme` — zero
per-component work). You never write a color literal or `bg-[#…]` (ESLint
bans it).

## 5. Sizing — square squares that scale

```tsx
// chess-board.tsx
<div className="grid aspect-square w-full grid-cols-8 grid-rows-8">
  {/* 64 × <Square /> — each cell is exactly 1/8 of the board on BOTH axes */}
</div>
```

- The board takes **full width** from F4's game screen (F4's README: "board
  scales to width — give it a width") and `aspect-square` derives a square
  height. Never a fixed pixel size, never a stretched rectangle.
- The grid does the rest: 8 equal columns × 8 equal rows inside that square.
  No per-square size math anywhere.
- Page-level layout (page padding, max-widths, page scrolling) is F4's
  concern — the board itself adds none. It must not clip at phone widths
  (~375 px — checklist 03 §I).
- Keep overlays inside their square (absolute `inset-0` layers, 02 §3) so
  nothing spills onto neighboring squares.

## 6. Keyboard + focus model (REQUIRED — RULES.md §2)

The board must be fully playable from the keyboard. This module's decision:
**64 square `<button>`s under a roving tabindex** (not 64 tab stops — one):

- **Roving tabindex.** Exactly ONE square has `tabIndex={0}`; the other 63
  have `tabIndex={-1}`. Tab into the board lands on that square; Tab again
  leaves the board. Keep the roving stop in state (where focus last was);
  initialize it to the top-left displayed square.
- **Arrow keys move focus by displayed position** (left arrow = one cell
  visually left in the CURRENT orientation). Clamp at edges — no wrapping.
  Call `e.preventDefault()` on the handled arrows so the page does not
  scroll while the board has focus.
- **Enter / Space** activate the focused square button — the browser does
  this natively for `<button>`; route it through the SAME handler as a
  click (one code path, 02 §2).
- **Escape** cancels the current selection (02 §2) — a keydown handler on
  the board while a selection is active.
- **Visible focus in both themes** (RULES.md §2): e.g.
  `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary`
  on each square. Check it in dark mode too (03 §J).
- Each square button carries an `aria-label`: square name + occupant, e.g.
  `"e4, white pawn"`, `"e5"`. (Appending `, legal move target` while a
  selection is active is welcome polish, not required.) The board container
  is the `<div role="group" aria-label="Chess board">` from §1.

Do NOT use the HTML `disabled` attribute to make squares inert — it removes
them from keyboard focus. Keep them focusable, ignore activation, set
`aria-disabled="true"` (02 §5).

## 7. Rendering performance — memo, stable keys, nothing exotic

- 64 small DOM nodes is nothing for a browser: no virtualization, no
  windowing, no measurement code. Plain elements.
- **`React.memo` on `Square`.** Squares re-render only when their own
  primitive props change. For that, pass **primitives** down (square name,
  piece type + color or `null`, booleans like `isTarget` / `isLastMove` /
  `isCheck` / `isSelected` / `isFocusStop`) — never a freshly created
  object/array per render, or memo never helps.
- **Stable keys:** `key={squareName}` (`"a1"…"h8"`), never the array index —
  an index key would survive an orientation flip while its square changed
  meaning, which React resolves by mislabeling state.
- Memoize the FEN parse on `snapshot.fen` (§2) and derive per-square props in
  one `useMemo` per concern (e.g. a `Set` of target squares — 02 §2).
- Snapshots arrive one move at a time (seconds apart) — this budget is
  generous. Do not add measurement hacks or manual DOM diffing.

## 8. What good looks like at the end of this stage

- `fen.ts`, `orientation.ts` (+ `move-targets.ts` from 02) are pure modules
  with no React imports, each with a passing vitest sibling.
- Feeding `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`
  renders 32 pieces in the right places (harness: `03-manual-testing.md`).
- Flipping `orientation` rotates the board; clicks and keyboard activation
  still report correct square names (log them).
- Keyboard: Tab reaches exactly one square, arrows move focus, Enter/Space
  activates, Escape cancels (03 §J).
- `npm run typecheck`, `npm run lint` and `npm run test` are clean.

Next: `02-pieces-and-interaction.md` — pieces and the click-to-move machine.
