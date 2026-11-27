# 03 — Manual testing (there is NO test runner — by design)

RULES.md §5 is binding: the frontend ships **no jest** (no test files in
this folder, ever). Your verification is exactly:

```bash
cd frontend
npm run typecheck   # tsc --noEmit — zero errors
npm run lint        # eslint — zero errors/warnings
```

…plus the manual checklists below, run on iOS AND Android before declaring
done (note what you checked in the PR description, RULES.md §5).

## 1. The dev harness (temporary, dev-only)

**Recommended (simplest): a scratch screen in YOUR folder + one temporary
line in `src/app/index.tsx`.**

1. Create `src/components/chess/board-dev-harness.tsx` (F2-owned) — a screen
   rendering `ChessBoard` with fixture props, state to flip
   orientation/snapshot, and a payload logger:

```tsx
// src/components/chess/board-dev-harness.tsx — DEV ONLY, never imported by
// any screen. Extra local FENs live HERE, not in protocol.fixtures.ts (F3 owns
// that file). TODOs are yours.
import { useState } from "react";
import { Text, View } from "react-native";
import { sampleFinishedSnapshot, sampleSnapshot } from "@/types/protocol.fixtures";
import { ChessBoard } from "./chess-board"; // + local fixtures below

// Local test FENs (do NOT edit src/types/protocol.fixtures.ts):
const INITIAL_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const PROMO_FEN = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";            // white pawn a7
const CHECK_FEN = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR"; // Qh4 gives check

export function BoardDevHarness() {
  // TODO: picker/buttons to switch snapshot (initial / sample / promo /
  //       check / finished) and orientation ("white" | "black")
  // TODO: render <ChessBoard snapshot={…} orientation={…}
  //        onMoveSubmit={(m) => console.log("[BoardMove]", m)} />
  return <View className="flex-1 items-center justify-center" />;
}
```

2. Temporarily point `src/app/index.tsx` at it (replace its default export's
   body with `return <BoardDevHarness />;` + import). **That file is F1's**
   (currently a placeholder) — keep the edit to one import + one render, and
   **REVERT IT BEFORE ANY PR/merge** (`git checkout -- src/app/index.tsx`);
   never commit it. Say so in the PR description.

3. Run: `npm start` → Expo Go on your phone, `npm run ios` / `npm run
   android`, or `npm run web` for a quick look (web is a dev convenience
   only — glyph/font and touch checks must still be done on real
   iOS/Android).

Alternative (once F4's screen exists): render inside the game screen with
mocked F3 props — better layout-in-context, worse Day-1 isolation. Either
way you need no server: `sampleSnapshot` (position after 1.e4, black to
move, `legal_moves: ["e7e5", "e7e6"]`) and `sampleFinishedSnapshot`
(checkmate, `legal_moves: []`) come from `@/types/protocol.fixtures`.

Why the logger matters: the harness's `onMoveSubmit` handler is your
assertion mechanism — you READ the exact payload in the console and compare
it character-by-character with the `game.move` payload in
`src/types/protocol.ts`.

## 2. The checklists

Run every item; expected result must match exactly. Fixture references:
`sampleSnapshot` / `sampleFinishedSnapshot` from `@/types/protocol.fixtures`
(fields: `fen`, `legal_moves`, `last_move`, `turn`, `game_status`,
`result`, …).

### A. Initial position (local `INITIAL_FEN`, orientation `"white"`)

- [ ] Exactly **32 pieces** render, no blanks/dupes.
- [ ] Spot-check squares: white pawns on rank 2, black pawns on rank 7,
      white king `e1`, black king `e8`, queens on `d1`/`d8`.
- [ ] `a1` (bottom-left) is a **dark** square; `h1` light; `a8` light.
- [ ] All squares equal size; board is square (measure by eye across
      several screen widths, incl. a small phone / 375 dp viewport).

### B. `sampleSnapshot` (after 1.e4) — last move + turn

- [ ] White pawn stands on **e4**, e2 is empty (board renders the snapshot
      fen, not local assumptions).
- [ ] `e2` and `e4` (from `last_move: { from: "e2", to: "e4", san: "e4" }`)
      show the `bg-last-move` highlight — and NOTHING else is highlighted.
- [ ] `turn` is `"black"`: with `orientation: "white"` the board is inert
      (white player watching black's turn) — taps do nothing.

### C. Legal-move targets (`sampleSnapshot`, `orientation: "black"`)

- [ ] Tap the e7 pawn → it becomes selected; **only `e6` and `e5** show
      `bg-move-target` markers (exactly `snapshot.legal_moves` =
      `["e7e5", "e7e6"]`, nothing more, nothing less).
- [ ] Tap another own piece (e.g. `g8`) → selection switches; it has no
      legal moves in the fixture, so NO target markers appear.
- [ ] Tap the selected square again / any unhighlighted square → selection
      clears, position never changes.

### D. Submit fires the exact payload (harness logger)

- [ ] `orientation: "black"`, tap `e7` then `e6` → console shows EXACTLY
      `{ from: "e7", to: "e6", promotion: null }` — field names, order
      irrelevant, `promotion` explicitly `null` (matches the `game.move`
      payload shape in `src/types/protocol.ts`).
- [ ] The board does NOT move the pawn itself — it stays until the harness
      swaps the snapshot prop (no optimistic update).
- [ ] A piece WITHOUT a listed legal move (e.g. `g8` knight) cannot submit.

### E. Promotion flow (local `PROMO_FEN` + `legal_moves: ["a7a8q","a7a8r","a7a8b","a7a8n"]`, `turn: "white"`, `orientation: "white"`)

- [ ] Tap `a7`, then `a8`: the **picker appears BEFORE any callback** (log
      stays silent).
- [ ] Picker offers Queen/Rook/Bishop/Knight with readable labels.
- [ ] Choose Rook → log shows `{ from: "a7", to: "a8", promotion: "r" }`.
- [ ] Cancel/dismiss the picker → NO callback, selection back (no submit).

### F. Orientation (`PROMO_FEN`, `orientation: "black"`)

- [ ] Board flips 180°: `a1` now **top-right**, `h8` bottom-left; colors of
      squares keep their names' parity (B).
- [ ] Tap-tap still submits the SAME payload: pieces on `a7`→`a8` produce
      `{ from: "a7", to: "a8", … }` (display position ≠ square name).

### G. Inert board

- [ ] `sampleSnapshot` + `orientation: "white"` (not your turn) → no
      selection possible.
- [ ] `sampleFinishedSnapshot` (checkmate, `result` set) → board renders
      the final position with last-move highlight but no interaction.
      (Its `fen` is the synthetic 1.e4 position — that's fine; this tests
      state-driven inertness, not mate visuals.)
- [ ] `disabled: true` passed by the harness → same inertness.
- [ ] Local `CHECK_FEN` snapshot with `game_status: "check"`, `turn:
      "white"` (white king `e1` in check from `Qh4`) → `e1` shows the
      `bg-check` highlight.

### H. Themes — both board token pairs

- [ ] Toggle the simulator between light and dark appearance: board uses
      the dark palette automatically (tokens from `global.css`), highlights
      stay readable and distinguishable from each other in BOTH themes.
- [ ] No hardcoded color anywhere survives a theme switch (if a highlight
      stays the same color in both themes, you hardcoded something).

### I. Layout + performance sanity

- [ ] Small phone (~375 dp): board fits the width, stays square, no
      clipping, nothing stretched.
- [ ] Rapid tapping / selecting many squares in a row: no visible lag or
      jank; press feedback is instant.
- [ ] Rotate/resize (or web-resize) — squares stay square at any width.

### J. Gates + cleanup

- [ ] `npm run typecheck` → zero errors. `npm run lint` → zero
      errors/warnings.
- [ ] `src/app/index.tsx` REVERTED (`git status` clean for that file);
      decide with the team whether the harness file itself is committed
      (dev-only, never imported by screens) or dropped.
- [ ] `04-acceptance.md` checked off and the PR description lists which
      checklist sections you ran.
