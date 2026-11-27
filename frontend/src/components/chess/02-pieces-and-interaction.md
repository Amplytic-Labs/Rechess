# 02 — Pieces and interaction

Covers: piece rendering strategy, the click-to-move state machine (pointer
AND keyboard), all highlights, the promotion dialog, disabled states, and the
optional drag-and-drop enhancement. Styling is Tailwind `className` with
`global.css` tokens only (RULES.md §1); type names come from
`@/types/protocol` (F3's file — import, never edit).

## 1. Piece rendering — pick a strategy

| | (a) Unicode glyphs in a `<span>` | (b) inline SVG you author | (c) raster image assets |
|---|---|---|---|
| Effort | hours — a glyph map | 1–2 days — SVG paths per piece | medium — sourcing + bundling |
| Quality | fine at MVP sizes; depends on system fonts | best: crisp at any size, theme-safe | fixed resolutions; blurs when scaled |
| Risk | tofu/fallback differences across platforms and browsers | none — zero dependencies | licensing + asset management |
| Dependency | none | none — you draw them yourself | none if assets live in your folder |

**Recommendation: (b) inline SVG is the recommended strategy — crisp
rendering at any size with zero dependencies. (a) Unicode glyphs are an
acceptable starting point if you must ship the board faster; upgrade to (b)
once click-to-move works. Treat (c) as the fallback if SVG authoring drags.**

- (a): map your `Piece` (`{ type, color }`) to one of ♔♕♖♗♘♙ / ♚♛♜♝♞♟ in a
  single constant; render with a `<span>` sized relative to the square. The
  piece's color is carried by the glyph pair itself (outline glyphs =
  White, solid glyphs = Black) — do NOT tint them with board tokens;
  pieces are not surfaces. You MUST check that the glyphs actually render
  in every browser you support (system fonts differ by OS); if a target
  browser shows tofu boxes, jump to (b) rather than bundling a font.
- (b): hand-authored inline SVG React components inside `piece.tsx` — no
  SVG rendering library, no chess UI or rules package (`chessground`,
  `react-chessboard`, a chess-rules lib, a piece pack, …). If you ever
  want one, it is a **team PR** (RULES.md §2) — propose it, don't install
  it silently. Draw each of the 12 pieces as a small component (or one
  component parameterized by type + color). Colors are tokens only — see
  §1.1.
- (c): only if you find a cleanly licensed set; record the source/license
  in a comment in `piece.tsx`, and keep the files inside
  `src/components/chess/` (imported from there) — `/public` is outside
  your ownership map (RULES.md §0).
- Whatever you choose, the strategy lives in **exactly one file**
  (`piece.tsx`) behind one memoized component — swapping strategies must
  never touch the board grid or the contract (README §4).

### 1.1 Piece colors obey the token rule

RULES.md §1.2 bans hardcoded colors — including `fill="#fff"` inside your
SVG paths. Two compliant paths:

- **Glyph MVP:** zero color code — the outline/solid glyph pair carries the
  side. Nothing to decide.
- **SVG upgrade:** pieces need two stable colors that read against
  `board-light`/`board-dark` in BOTH themes — the `piece-white` /
  `piece-black` tokens already exist in BOTH blocks of `global.css` (F1
  curates). Author paths and fill them with the token utilities
  `fill-piece-white` / `fill-piece-black` (and `stroke-*` variants where
  the art needs outlines).

```tsx
// src/components/chess/piece.tsx — skeleton, strategy (a). TODOs are yours.
import { memo } from "react";
import type { Piece } from "./fen"; // types from 01 §2

// White ♔♕♖♗♘♙, black ♚♛♜♝♞♟ — indexed by PieceType.
const GLYPHS = { /* TODO: p n b r q k → glyph pairs per color */ } as const;

function PieceGlyph({ piece }: { piece: Piece }) {
  // TODO: render the glyph for piece.type/piece.color, sized relative to
  // the square (tune with the harness — 03 §A / §I).
  // Glyph color comes from the glyph itself; never tint with board tokens.
  return <span className="leading-none">{""}</span>; // placeholder — replace
}

export const Piece = memo(PieceGlyph);
```

## 2. Click-to-move — a three-state machine (pointer AND keyboard)

**Click-click is the REQUIRED base**: click your own piece → its legal
targets highlight → click a target to submit. (Drag-and-drop is an optional
later enhancement, §6.)

Selection is LOCAL UI state (the only state your module keeps — a
"tentative selection" is explicitly allowed, RULES.md §4). The position
itself is never mutated locally.

```tsx
// chess-board.tsx — local interaction state. TODOs are yours.
type SelectionState =
  | { phase: "idle" }                                        // nothing selected
  | { phase: "selected"; from: Square }                      // piece picked
  | { phase: "promotion"; from: Square; to: Square };         // dialog open
```

**Guards before ANY selection** (all derived from props — no server calls,
no rule math):

- `props.disabled !== true` (F4's kill-switch: pending move, disconnected),
- it is the local player's turn: `snapshot.turn === orientation`,
- the game is live: `snapshot.result === null` and
  `snapshot.game_status` is `"in_progress"` or `"check"`.

**Transitions** (an "activation" = a mouse click, a tap, or Enter/Space on
the focused square — all one code path, 01 §6):

| Current state + activation | Next |
|---|---|
| `idle` + own piece (`piece.color === snapshot.turn`) | `selected(from)` — show its targets |
| `idle` + anything else | `idle` (inert; optional polish: subtle feedback) |
| `selected` + another own piece | `selected(newFrom)` — switch selection |
| `selected` + a legal destination of `from` (non-promotion) | fire `onMoveSubmit({ from, to, promotion: null })` → `idle` |
| `selected` + a legal destination (promotion — §4) | `promotion(from, to)` — dialog first |
| `selected` + same square / non-highlighted square | `idle` — deselect, NEVER alter the board |
| `promotion` + choose a piece | fire `onMoveSubmit({ from, to, promotion: choice })` → `idle` |
| `promotion` + dismissed | `selected(from)` — no submit |

**Keyboard additions** (focus model in 01 §6): arrow keys move focus only —
they never select; Enter/Space activate the focused square exactly like a
click; Escape while `selected` returns to `idle` (focus stays on the square
it was on).

**Deriving destinations** — pure helpers in `move-targets.ts` (string
filtering of server data — allowed AND required; computing what *should* be
legal is forbidden):

```ts
// src/components/chess/move-targets.ts — PURE.
// Unit tests: move-targets.test.ts (03 §2) — node environment, no DOM.

/** Target squares of `from` from the server's legal_moves: "e7e5" → "e5". */
export function targetsFor(legalMoves: string[], from: Square): Set<string>;
/** True when from→to has a promotion variant ("e7e8q" — a 5th character). */
export function isPromotion(legalMoves: string[], from: Square, to: Square): boolean;
```

The implementation is two filters (keep the logic in the pure module, the
board just calls it):

```tsx
const fromMoves = snapshot.legal_moves.filter((m) => m.slice(0, 2) === from);
const targets = new Set(fromMoves.map((m) => m.slice(2, 4)));       // "e7e5" → "e5"
const isPromotion = fromMoves.some((m) => m.length > 4 && m.slice(0, 4) === from + to);
```

- A move is `"e2e4"`-style UCI; a **5th character** (`"e7e8q"`) marks a
  promotion and is one of `"q" | "r" | "b" | "n"`.
- Never invent a target that is not in `snapshot.legal_moves` — not even
  "obvious" ones like captures. Highlights are server data, full stop.

**After a submit — you do nothing else.** Clear selection, stay on the
current `fen`, wait for F4 to pass the next snapshot. There is no optimistic
move (RULES.md §4): a rejected move (`game.move_rejected`, F3/F4's side)
never desyncs you because the board never moved. That satisfies your brief's
"invalid/rejected move feedback without leaving the board visually out of
sync" by construction — F4 shows the error toast; you keep rendering the
previous `fen`.

**Reset on new snapshot:** when `snapshot.fen` changes (server update,
reconnect, resume), clear `selected` AND `promotion` — a `useEffect` on
`snapshot.fen` (or reset during render when the fen changed since last
render). Also reset when the guards above turn false mid-selection (e.g. the
opponent just moved). A pending-move lock is F4's `disabled` prop (brief §5:
"prefer disabling for MVP") — do not build your own in-flight queue.

## 3. Highlights — tokens, z-order, never obscure pieces

All colors are board tokens from `frontend/global.css`. Layer inside each
`Square`: background → `last-move`/`check` overlay → `move-target` marker →
piece → (optional drag ghost).

| Highlight | Source (field names exact) | Token / rendering |
|---|---|---|
| Selected square | local `selected.from` | distinct ring, e.g. `rounded-sm border-2 border-primary` inset — must differ clearly from targets |
| Legal targets | `snapshot.legal_moves` filtered by `from` (§2) | `bg-move-target` — small centered dot (`size-1/4 rounded-full`) on empty squares; translucent full-square overlay or ring when an enemy piece sits there (capture). Uniform overlay is acceptable for MVP; dot-vs-ring is polish |
| Last move | `snapshot.last_move?.from` and `.to` (`{ from, to, san }`) | translucent `bg-last-move` overlay on both squares |
| Check | `game_status === "check"` → king square, §below | translucent `bg-check` overlay |
| Board squares | square-name parity (doc 01 §4) | `bg-board-light` / `bg-board-dark` |

- Overlays sit BEHIND the piece or are translucent (Tailwind opacity
  modifier, e.g. `bg-last-move/40` — it works with `@theme` tokens in
  Tailwind v4), so highlights never obscure pieces. Verify the modifier
  renders in both themes; if not, a plain token fill with the piece on top
  still satisfies "not obscured".
- **Check indicator — finding the king is board math, NOT rules logic.** When
  `snapshot.game_status === "check"`, call `checkedKingSquare(pieces,
  snapshot.turn)` from `fen.ts` (doc 01 §2 — it scans your `fenToPieces` map
  for the side to move's king) and highlight that square with `bg-check`.
  You are reading the position — you are NOT deciding whether it is check
  (the server already said so), so this is allowed. No attack detection,
  ever.
- Four highlight kinds must stay visually distinguishable (your brief §6) —
  check them in both themes (`03-manual-testing.md` §H).
- The whole square is the `<button>` (doc 01 §1) — comfortable click targets,
  no pixel-perfect clicking.
- If you want a dedicated `selected-square` token instead of `border-primary`,
  that is a small PR against `global.css` (F1 curates) — never a local color.

## 4. Promotion dialog — ask BEFORE submitting

Trigger: from `selected(from)`, the clicked `to` has a matching legal move
WITH a 5th character (`"e7e8q"`) — i.e. `isPromotion` from §2. Then show the
chooser BEFORE any callback. The wire values are exactly the `Promotion` type
from `@/types/protocol`: `"q" | "r" | "b" | "n"`.

- UI: a **small dialog or dialog-like overlay centered over the board —
  NOT a bottom sheet** (that is a mobile-app pattern; on the web, desktop
  and phone alike, a compact centered dialog is the right shape).
- Options: **Queen `"q"`, Rook `"r"`, Bishop `"b"`, Knight `"n"`** — each
  button shows the piece visual AND a clear word label ("Queen", "Rook",
  "Bishop", "Knight") for accessibility (your brief §6).
- **Defaults to Queen**: the Queen button receives focus when the dialog
  opens, so a straight Enter submits Queen — the common case is one
  keystroke.
- On choose → `onMoveSubmit({ from, to, promotion: choice })` (promotion is
  the choice, never `null` here) → `idle`.
- On dismiss (Escape, scrim click, an explicit Cancel) → back to
  `selected(from)`, nothing submitted.
- Implementation — a small custom overlay (primary spec): a scrim
  (`fixed inset-0 bg-foreground/25`) + a card (`bg-surface border
  border-border rounded-lg p-4 shadow-lg`), carrying `role="dialog"`,
  `aria-modal="true"` and `aria-label="Choose a promotion piece"`; keep Tab
  within the four buttons while it is open, and the board beneath does not
  respond while the dialog is up. A native `<dialog>` element opened with
  `showModal()` (focus containment and Escape come free) is an acceptable
  alternative — either way, tokens only.
- RULES.md §1.3 puts shared dialog primitives in `src/components/common/`
  (F1's), but that folder does not exist yet — and the promotion chooser is
  chess-specific UI that belongs to F2. When F1 ships a shared Dialog
  primitive, switching to it is a small refactor behind this file's
  interface.

```tsx
// src/components/chess/promotion-chooser.tsx — skeleton. TODOs are yours.
import type { Promotion } from "@/types/protocol"; // "q" | "r" | "b" | "n"

const CHOICES: { value: Promotion; label: string }[] = [
  { value: "q", label: "Queen" }, { value: "r", label: "Rook" },
  { value: "b", label: "Bishop" }, { value: "n", label: "Knight" },
];

export function PromotionChooser({
  from, to, onChoose, onCancel,
}: {
  from: string; to: string;
  onChoose: (p: Promotion) => void;
  onCancel: () => void;
}) {
  // TODO: dialog overlay with the four labeled choices (piece visual + word
  //       label), themed with tokens. The Queen button is focused first
  //       (the default choice). Escape/scrim/Cancel dismiss without choosing.
  // onChoose(value) → parent submits { from, to, promotion: value }
  // onCancel()      → parent returns to "selected", submits nothing
  return null; // placeholder — replace
}
```

## 5. Disabled states (inert board)

The board stops RESPONDING but keeps rendering (final position, last move):

- **Not your turn** (`snapshot.turn !== orientation`): activations select
  nothing. Recommended for MVP: fully inert (the server enforces turn
  anyway — this is UX, per F4's README). Optional polish later: allow
  selecting own pieces to browse their targets without submitting.
- **Game finished**: `snapshot.result !== null`, or `game_status` is
  `"checkmate" | "stalemate" | "draw" | "resigned"`, or room
  `status === "finished"` → inert.
- **`disabled` prop** (F4: pending submitted move, lost connection): inert.

When inert, ignore activation but keep squares focusable for keyboard
browsing and set `aria-disabled="true"` — never the HTML `disabled`
attribute, which would kill the focus model (doc 01 §6).

Turn/connection banners, pending spinners and error toasts are F4's screen
concerns (see `04-acceptance.md` §2) — you only make the board unresponsive
in the three cases above.

## 6. OPTIONAL enhancement — drag-and-drop (after click-click passes)

Do NOT start before click-to-move passes its checklist (03 §A–§G, §J). No
drag/gesture library — **pointer events only** (RULES.md §2: no new
dependency without a team PR):

- `onPointerDown` on the piece → `setPointerCapture(e.pointerId)`; track a
  translate offset while the pointer moves; on `pointerup`, compute the
  drop square from the board's bounding rect (pure math with
  `orientation.ts` — no hit-testing hacks) and reuse the SAME
  `selected → target` submit path from §2 (the drag only replaces the
  second click). A drop on an illegal square = deselect, no board change.
- The animated translate offset is a **computed dynamic value** — this is
  the one legitimate `style` prop in your module; keep it to that single
  use and comment it (RULES.md §1.1). Everything else stays `className`.
- Set `touch-none` (`touch-action: none`) on the draggable piece so
  pointermove is not hijacked by page scrolling on touch screens.
- Apply a small movement threshold before a drag engages, so a plain click
  still follows the click-click path (a pointerdown→pointerup on the same
  spot without movement is a click, not a drag).
- Escape cancels an active drag (the piece snaps back, no submit).
- Promotion via drag (dropping on the last rank where `isPromotion` is
  true) opens the same §4 dialog before submitting.

## 7. Where this stage is tested

- **Unit (vitest, node env)**: `move-targets.test.ts` pins `targetsFor` /
  `isPromotion` against `"e7e5"` / `"e7e8q"`-style lists — see
  `03-manual-testing.md` §2.
- **Manual (browser)**: every behavior in this doc has a matching checklist
  section in `03-manual-testing.md` (§C targets, §D submit payload,
  §E promotion, §G inert, §H themes, §J keyboard, §L drag if implemented).
  Run them before every PR; UI verification is manual by design
  (RULES.md §5).
