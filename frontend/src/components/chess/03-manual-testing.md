# 03 — Testing: vitest units + manual UI in the browser

RULES.md §5 is binding: vitest covers **pure logic only** (node environment —
the suites call functions, never render anything); the UI is verified
**manually in the browser** — desktop AND a phone-width window. No simulator,
no emulator: the browser is the platform. Your verification is exactly:

```bash
cd frontend
npm run typecheck   # tsc --noEmit — zero errors
npm run lint        # eslint — zero errors/warnings
npm run test        # vitest — pure-logic units (fen / orientation / move-targets)
npm run build       # production build (CI runs it too)
```

…plus the manual checklists below, run before declaring done — note what you
checked in the PR description (RULES.md §5).

## 1. The dev harness (temporary, dev-only)

**Recommended (simplest): a scratch component in YOUR folder + one temporary
line in a route file.**

1. Create `src/components/chess/board-dev-harness.tsx` (F2-owned) — a client
   component rendering `ChessBoard` with fixture props, buttons to flip
   orientation/snapshot, and a payload logger:

```tsx
// src/components/chess/board-dev-harness.tsx — DEV ONLY, never imported by
// any screen. Extra local FENs live HERE, not in protocol.fixtures.ts (F3 owns
// that file). TODOs are yours.
"use client";
import { useState } from "react";
import { sampleFinishedSnapshot, sampleSnapshot } from "@/types/protocol.fixtures";
import { ChessBoard } from "./chess-board"; // + local fixtures below

// Local test FENs (do NOT edit src/types/protocol.fixtures.ts):
const INITIAL_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const PROMO_FEN = "4k3/P7/8/8/8/8/8/4K3 w - - 0 1";            // white pawn a7
const CHECK_FEN = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR"; // Qh4 gives check

export function BoardDevHarness() {
  // TODO: buttons to switch snapshot (initial / sample / promo /
  //       check / finished) and orientation ("white" | "black")
  // TODO: render <ChessBoard snapshot={…} orientation={…}
  //        onMoveSubmit={(m) => console.log("[BoardMove]", m)} />
  return null; // placeholder — replace
}
```

2. Temporarily mount it from a route file you do NOT own — the placeholder
   lobby route `src/app/page.tsx` (F1's) is the usual spot: replace the
   page function's body with `return <BoardDevHarness />;` + import. Keep
   the edit to one import + one render, and **REVERT IT BEFORE ANY
   PR/merge** (`git checkout -- src/app/page.tsx`); never commit it. Say so
   in the PR description.

3. Run `npm run dev` and open **http://localhost:3001** (the Rust backend
   owns :3000). All interaction checks — click, keyboard, optional drag —
   run right there in the browser. Use the devtools **device toolbar** for
   the phone-width pass (375 px) and the devtools rendering emulation (or
   the OS appearance switch) for the dark theme.

Alternative (once F4's screen exists): render inside the game screen with
mocked F3 props — better layout realism, worse Day-1 isolation. Either way
you need no server: `sampleSnapshot` (position after 1.e4, black to move,
`legal_moves: ["e7e5", "e7e6"]`) and `sampleFinishedSnapshot` (checkmate,
`legal_moves: []`) come from `@/types/protocol.fixtures`.

Why the logger matters: the harness's `onMoveSubmit` handler is your
assertion mechanism — you READ the exact payload in the console and compare
it character-by-character with the `game.move` payload in
`src/types/protocol.ts`.

## 2. Unit tests — vitest siblings (node environment, no DOM)

One `.test.ts` per pure module (file layout in 01 §1); they import functions
only, never render components:

- `fen.test.ts`
  - `fenToPieces(INITIAL_FEN)` → 32 entries; white pawns on rank 2, black
    pawns on rank 7, kings `e1`/`e8`, queens `d1`/`d8`.
  - `fenToPieces(PROMO_FEN)` → exactly the two kings + the `a7` pawn.
  - Malformed input (wrong rank count, a rank consuming ≠ 8 files, an
    unexpected character, an empty string) → empty map, NEVER a throw.
  - `checkedKingSquare(pieces, "white")` on the `CHECK_FEN` parse → `"e1"`;
    a missing king → `null`.
- `orientation.test.ts`
  - `squareAt(0, 0, "white") === "a8"`, `squareAt(7, 7, "white") === "h1"`,
    `squareAt(0, 0, "black") === "h1"`, `squareAt(7, 7, "black") === "a8"`.
  - Totality: for each orientation, the 64 `(r, c)` pairs yield 64 distinct
    square names.
- `move-targets.test.ts`
  - `targetsFor(["e7e5", "e7e6"], "e7")` → a Set of exactly `{"e5", "e6"}`;
    `targetsFor(…, "g8")` → empty.
  - `isPromotion(["a7a8q", "a7a8r", "a7a8b", "a7a8n"], "a7", "a8")` → `true`;
    `isPromotion(["e7e5"], "e7", "e5")` → `false`; a promotion move aimed at
    a different `to` never counts.

Run: `npm run test`. Keep the suites pure — no React imports, no jsdom. If
you believe you need DOM tests, that is a team PR first (RULES.md §5; the
vitest config is node-only on purpose).

## 3. The manual checklists

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
      several window widths, incl. the 375 px device-toolbar width).

### B. `sampleSnapshot` (after 1.e4) — last move + turn

- [ ] White pawn stands on **e4**, e2 is empty (board renders the snapshot
      fen, not local assumptions).
- [ ] `e2` and `e4` (from `last_move: { from: "e2", to: "e4", san: "e4" }`)
      show the `bg-last-move` highlight — and NOTHING else is highlighted.
- [ ] `turn` is `"black"`: with `orientation: "white"` the board is inert
      (white player watching black's turn) — clicks do nothing.

### C. Legal-move targets (`sampleSnapshot`, `orientation: "black"`)

- [ ] Click the e7 pawn → it becomes selected; **only `e6` and `e5`** show
      `bg-move-target` markers (exactly `snapshot.legal_moves` =
      `["e7e5", "e7e6"]`, nothing more, nothing less).
- [ ] Click another own piece (e.g. `g8`) → selection switches; it has no
      legal moves in the fixture, so NO target markers appear.
- [ ] Click the selected square again / any unhighlighted square → selection
      clears, position never changes.

### D. Submit fires the exact payload (harness logger)

- [ ] `orientation: "black"`, click `e7` then `e6` → console shows EXACTLY
      `{ from: "e7", to: "e6", promotion: null }` — field names, order
      irrelevant, `promotion` explicitly `null` (matches the `game.move`
      payload shape in `src/types/protocol.ts`).
- [ ] The board does NOT move the pawn itself — it stays until the harness
      swaps the snapshot prop (no optimistic update).
- [ ] A piece WITHOUT a listed legal move (e.g. `g8` knight) cannot submit.

### E. Promotion flow (local `PROMO_FEN` + `legal_moves: ["a7a8q","a7a8r","a7a8b","a7a8n"]`, `turn: "white"`, `orientation: "white"`)

- [ ] Click `a7`, then `a8`: the **dialog appears BEFORE any callback** (log
      stays silent) — a small centered dialog, NOT a bottom sheet.
- [ ] The dialog offers Queen/Rook/Bishop/Knight with readable labels.
- [ ] Choose Rook → log shows `{ from: "a7", to: "a8", promotion: "r" }`.
- [ ] Cancel/dismiss the dialog → NO callback, selection back (no submit).
- [ ] The Queen choice is the default: when the dialog opens, focus sits on
      the Queen button, and pressing Enter alone submits
      `{ from: "a7", to: "a8", promotion: "q" }`.

### F. Orientation (`PROMO_FEN`, `orientation: "black"`)

- [ ] Board flips 180°: `a1` now **top-right**, `h8` bottom-left; colors of
      squares keep their names' parity (A).
- [ ] Click-click still submits the SAME payload: pieces on `a7`→`a8`
      produce `{ from: "a7", to: "a8", … }` (display position ≠ square
      name).

### G. Inert board

- [ ] `sampleSnapshot` + `orientation: "white"` (not your turn) → no
      selection possible (keyboard focus browsing still works — §J).
- [ ] `sampleFinishedSnapshot` (checkmate, `result` set) → board renders
      the final position with last-move highlight but no interaction.
      (Its `fen` is the synthetic 1.e4 position — that's fine; this tests
      state-driven inertness, not mate visuals.)
- [ ] `disabled: true` passed by the harness → same inertness.
- [ ] Local `CHECK_FEN` snapshot with `game_status: "check"`, `turn:
      "white"` (white king `e1` in check from `Qh4`) → `e1` shows the
      `bg-check` highlight.

### H. Themes — both board token pairs

- [ ] Toggle the devtools rendering emulation (or the OS appearance switch)
      between light and dark: the board switches to the dark palette
      automatically (tokens from `global.css` via `prefers-color-scheme` —
      no JS, no reload), and highlights stay readable and distinguishable
      from each other in BOTH themes.
- [ ] No hardcoded color anywhere survives the theme switch (if a highlight
      stays the same color in both themes, you hardcoded something).

### I. Layout + performance sanity

- [ ] Phone width (device toolbar, 375 px): board fits the width, stays
      square, no clipping, nothing stretched, no horizontal page scroll.
- [ ] Rapid clicking / selecting many squares in a row: no visible lag or
      jank; press feedback is instant.
- [ ] Resize the window across widths — squares stay square at any width.

### J. Keyboard operability (RULES.md §2 — focus model in 01 §6)

- [ ] Tab reaches exactly ONE square (roving tabindex); Tab again leaves
      the board; Shift+Tab returns to the same square.
- [ ] Arrow keys move focus by displayed position, clamped at the edges;
      the page does NOT scroll while the board has focus.
- [ ] Enter/Space on your own piece → selected (targets appear); Enter/Space
      on a highlighted target → submits (same payload as §D).
- [ ] Escape while a selection is active → selection clears, nothing
      submitted.
- [ ] A visible focus ring shows on the focused square in BOTH themes.
- [ ] The whole promotion flow (§E) is doable with the keyboard alone:
      Enter selects `a7`, arrows to `a8`, Enter opens the dialog (Queen
      focused), Enter picks Queen.
- [ ] While a dialog is open, Tab stays within the dialog and the board
      beneath does not respond.

### K. Gates + cleanup

- [ ] `npm run typecheck` → zero errors. `npm run lint` → zero
      errors/warnings. `npm run test` → all green. `npm run build` succeeds.
- [ ] `src/app/page.tsx` REVERTED (`git status` clean for that file);
      decide with the team whether the harness file itself is committed
      (dev-only, never imported by screens) or dropped.
- [ ] `04-acceptance.md` checked off and the PR description lists which
      checklist sections you ran.

### L. Drag (ONLY if the optional drag enhancement is implemented — 02 §6)

- [ ] Dragging the e7 pawn onto `e5` submits the SAME payload as §D
      (drag reuses the click submit path).
- [ ] A press without movement is still a click (selection works).
- [ ] Dropping on an illegal square deselects; the board never changes.
- [ ] Escape mid-drag cancels it (piece snaps back, nothing submitted).
- [ ] On a touch screen, dragging a piece does not scroll the page.
