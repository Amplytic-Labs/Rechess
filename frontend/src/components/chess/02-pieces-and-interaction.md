# 02 — Pieces and interaction

Covers: piece rendering strategy, the tap-to-move state machine, all
highlights, the promotion picker, disabled states, and the optional drag
stretch goal. Styling is uniwind `className` with `global.css` tokens only
(RULES.md §1); type names come from `@/types/protocol` (F3's file — import,
never edit).

## 1. Piece rendering — pick a strategy

| | (a) Unicode glyphs in `<Text>` | (b) `react-native-svg` shapes | (c) `expo-image` raster assets |
|---|---|---|---|
| Effort | hours — a glyph map | 1–2 days — SVG paths per piece | medium — source + bundle assets |
| Quality | fine at MVP sizes; depends on platform fonts | best: crisp at any size, tintable | good, but fixed resolutions |
| Risk | font fallback differs iOS/Android (must test both) | none — `react-native-svg` 15.16.0 already in `package.json` | licensing + asset management |
| Dependency | none | **none — already installed, no team PR** | none — `expo-image` ~58.0.19 installed |

**Recommendation: ship the MVP with (a); upgrade to (b) as the first stretch
goal once tap-to-move works; treat (c) as the fallback if SVG authoring drags.**

- (a): map your `Piece` (`{ type, color }`) to one of ♔♕♖♗♘♙ / ♚♛♜♝♞♟ in a
  single constant; render with `<Text>` sized relative to the square
  (`flex-1` square, glyph `className` with a large text size — tune on a
  device). The pieces' color is carried by the glyph itself — do NOT tint
  them with board tokens. You MUST verify glyphs actually render on both
  iOS and Android (your brief §6: test fallback fonts); if Android shows
  tofu boxes, jump straight to (b) rather than bundling a font.
- (b): no new dependency (already installed), so no package.json PR — it is
  purely your code. Keep it inside `piece.tsx`.
- (c): only if you find a cleanly licensed set; record the source/license in
  a comment in `piece.tsx`.
- Whatever you choose, the strategy lives in **exactly one file**
  (`piece.tsx`) behind one memoized component — swapping strategies must
  never touch the board grid or the contract (README §4).

```tsx
// src/components/chess/piece.tsx — skeleton, strategy (a). TODOs are yours.
import { memo } from "react";
import { Text } from "react-native";
import type { Piece } from "./fen"; // types from 01 §2

// White ♔♕♖♗♘♙, black ♚♛♜♝♞♟ — indexed by PieceType.
const GLYPHS = { /* TODO: p n b r q k → glyph pairs per color */ } as const;

function PieceGlyph({ piece }: { piece: Piece }) {
  // TODO: render the glyph for piece.type/piece.color, sized to the square.
  // Glyph color comes from the glyph itself; never tint with board tokens.
  return <Text className="text-foreground">{""}</Text>; // placeholder — replace
}

export const Piece = memo(PieceGlyph);
```

## 2. Tap-to-move — a three-state machine

Selection is LOCAL UI state (the only state your module keeps — a
"tentative selection" is explicitly allowed, RULES.md §4). The position
itself is never mutated locally.

```tsx
// chess-board.tsx — local interaction state. TODOs are yours.
type SelectionState =
  | { phase: "idle" }                                        // nothing selected
  | { phase: "selected"; from: Square }                      // piece picked
  | { phase: "promotion"; from: Square; to: Square };         // picker open
```

**Guards before ANY selection** (all derived from props — no server calls,
no rule math):

- `props.disabled !== true` (F4's kill-switch: pending move, disconnected),
- it is the local player's turn: `snapshot.turn === orientation`,
- the game is live: `snapshot.result === null` and
  `snapshot.game_status` is `"in_progress"` or `"check"`.

**Transitions:**

| Current state + tap | Next |
|---|---|
| `idle` + own piece (`piece.color === snapshot.turn`) | `selected(from)` — show its targets |
| `idle` + anything else | `idle` (inert; optional polish: subtle feedback) |
| `selected` + another own piece | `selected(newFrom)` — switch selection |
| `selected` + a legal destination of `from` (non-promotion) | fire `onMoveSubmit({ from, to, promotion: null })` → `idle` |
| `selected` + a legal destination (promotion — see §4) | `promotion(from, to)` — picker first |
| `selected` + same square / non-highlighted square | `idle` — deselect, NEVER alter the board |
| `promotion` + choose a piece | fire `onMoveSubmit({ from, to, promotion: choice })` → `idle` |
| `promotion` + dismissed | `selected(from)` — no submit |

**Deriving destinations** (string filtering of server data — allowed AND
required; computing what *should* be legal is forbidden):

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

- Overlays sit BEHIND the piece glyph or are translucent (Tailwind opacity
  modifier, e.g. `bg-last-move/40`), so highlights never obscure pieces.
  Verify the modifier renders in both themes; if not, a plain token fill
  with the glyph on top still satisfies "not obscured".
- **Check indicator — finding the king is board math, NOT rules logic.** When
  `snapshot.game_status === "check"`, scan your `fenToPieces` map for the
  piece `{ type: "k", color: snapshot.turn }` (the side to move is in check)
  and highlight that square with `bg-check`. You are reading the position —
  you are NOT deciding whether it is check (the server already said so), so
  this is allowed. No attack detection, ever.
- Four highlight kinds must stay visually distinguishable (your brief §6) —
  check them in both themes (`03-manual-testing.md` §H).
- Touch targets: the whole square is the `Pressable` — no pixel-perfect taps.
- If you want a dedicated `selected-square` token instead of `border-primary`,
  that is a small PR against `global.css` (F1 curates) — never a local color.

## 4. Promotion picker — ask BEFORE submitting

Trigger: from `selected(from)`, the tapped `to` has a matching legal move
WITH a 5th character (`"e7e8q"`) — i.e. `isPromotion` from §2. Then show the
chooser BEFORE any callback. The wire values are exactly the `Promotion` type
from `@/types/protocol`: `"q" | "r" | "b" | "n"`.

- Options: **Queen `"q"`, Rook `"r"`, Bishop `"b"`, Knight `"n"`** — each
  button shows the glyph AND a clear text label (accessibility, brief §6).
- On choose → `onMoveSubmit({ from, to, promotion: choice })` (promotion is
  the choice, never `null` here) → `idle`.
- On dismiss → back to `selected(from)`, nothing submitted.
- Implementation: HeroUI Native `Dialog` with a granular import
  (`import { Dialog } from "heroui-native/dialog";` — root `heroui-native`
  import is banned by ESLint), or a simple custom modal (a `View` overlay +
  four themed buttons) if the dialog fights your layout. Either is fine;
  themed with tokens, obviously.

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
  // TODO: modal with the four labeled choices (glyph + text), themed.
  // onChoose(value) → parent submits { from, to, promotion: value }
  // onCancel()      → parent returns to "selected", submits nothing
  return null; // placeholder — replace
}
```

## 5. Disabled states (inert board)

The board stops RESPONDING but keeps rendering (final position, last move):

- **Not your turn** (`snapshot.turn !== orientation`): taps select nothing.
  Recommended for MVP: fully inert (the server enforces turn anyway — this is
  UX, per F4's README). Optional polish later: preview-selection.
- **Game finished**: `snapshot.result !== null`, or `game_status` is
  `"checkmate" | "stalemate" | "draw" | "resigned"`, or room
  `status === "finished"` → inert.
- **`disabled` prop** (F4: pending submitted move, lost connection): inert.

Turn/connection banners, pending spinners and error toasts are F4's screen
concerns (see `04-acceptance.md` §2) — you only make the board unresponsive
in the three cases above.

## 6. OPTIONAL stretch goal — drag-and-drop (explicitly optional)

Do NOT start before tap-to-move passes its checklist (03 §A–§G). Both
libraries are already installed: `react-native-reanimated` 4.7.1 +
`react-native-gesture-handler` ~3.3 (the app is already wrapped in
`GestureHandlerRootView` — see `src/app/_layout.tsx`). Sketch:

- `Gesture.Pan()` on the piece; `useSharedValue` for the translate offsets;
  `useAnimatedStyle` for the transform; on gesture end, hit-test the drop
  square and reuse the SAME `selected → target` submit path from §2 (the
  gesture only replaces the second tap).
- The animated translate offset is a **computed dynamic value** — this is the
  one legitimate `style` prop in your module; keep it to that single use and
  comment it (RULES.md §1.1). Everything else stays `className`.
- Tap must keep working when the gesture is a no-op (a tap is a pan of zero
  movement — make sure the tap path survives).
- Never use the legacy `Animated` API from react-native (RULES.md §2) —
  always Reanimated 4.7.

## 7. Where this stage is tested

Every behavior in this doc has a matching checklist section in
`03-manual-testing.md` (§C targets, §D submit payload, §E promotion, §G
inert, §H themes). Run them before every PR; there is no jest on this
frontend (RULES.md §5) and never a test file in this folder.
