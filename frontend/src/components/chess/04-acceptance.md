# 04 — Definition of done + handoff to F4

Line-by-line mapping of your brief
(`07_F2_Chessboard_Interaction.md` §2 "Required work", §5 interaction rules,
§6 visual checklist) to the doc set and the acceptance checks. Check an item
only after its `03-manual-testing.md` sections passed in the browser (desktop
+ phone-width), its vitest suite is green, and the four gates
(`npm run typecheck` / `lint` / `test` / `build`) are clean.

## 1. DoD checklist — brief requirement → how → check

| # | Brief requirement (§2) | Covered by | Accept when |
|---|---|---|---|
| 1 | Board accepts an authoritative FEN position | 01 §2 `fenToPieces` + README §4 (`snapshot.fen` from `GameSnapshot`) | vitest `fen.test.ts` + A, B |
| 2 | Render all 12 piece types reliably, one consistent asset strategy | 02 §1 (inline SVG recommended, Unicode glyphs an acceptable start; no SVG/chess library without a team PR) | A (all 12 types visible) + H (both themes) |
| 3 | White-at-bottom and Black-at-bottom; square mapping stays correct when flipped | 01 §3 `orientation.ts` | vitest `orientation.test.ts` + F (payload identical on the flipped board) |
| 4 | Click own piece → show destinations from server-provided legal moves | 02 §2 state machine + `move-targets.ts` | C + vitest `move-targets.test.ts` |
| 5 | On destination → `onMoveSubmit` with from/to/promotion; never mutate the board locally | 02 §2 (`BoardMove` = `game.move` payload minus `expected_revision`) | D: logger shows exact payload, board unchanged |
| 6 | Highlight last move, selected square, legal destinations, check king, disabled state | 02 §3 tokens `bg-last-move` / `bg-move-target` / `bg-check` / selected ring | B, C, G, H (distinguishable in both themes) |
| 7 | Promotion: explicit Q/R/B/N chooser, chosen value through the callback | 02 §4 (small centered dialog, Queen default; `Promotion` = `"q"\|"r"\|"b"\|"n"` from `@/types/protocol`) | E: dialog before callback, `promotion: "r"` for Rook, Enter alone picks Queen |
| 8 | Rejected-move feedback without visual desync | 02 §2 (no optimistic move — rejection is a no-op by construction) | D: board still renders previous `fen` |
| 9 | Scale to width, square squares, no clipping; don't fight the device chrome | 01 §5 CSS grid + `aspect-square w-full` (page chrome and scrolling belong to F4's layout) | A, I: square at all widths incl. 375 px, no clipping, no page scroll |
| 10 | Fixtures: initial / midgame / castling / en passant / promotion | 03 §1 local harness FENs + 03 §2 unit suites (extra FENs live in the harness, NOT in F3's `protocol.fixtures.ts`) | A, E + castling/en-passant just render after server snapshots (they are ordinary moves in `legal_moves`, e.g. `"e1g1"`, `"e5d6"` — nothing special to code) |
| 11 | Stale selection/promotion cleared on snapshot change | 02 §2 reset on `snapshot.fen` change; on reconnect F4 passes the fresh snapshot | G/B: no stale highlights after switching snapshots in the harness |
| 12 | Pending: disable additional submissions (preferred for MVP) | 02 §5 `disabled` prop from F4 | G: `disabled: true` → inert |
| 13 | (web) Keyboard-operable board — RULES.md §2 | 01 §6 roving-tabindex focus model + 02 §2 key transitions | J: Tab/arrow/Enter/Space/Escape all pass |
| 14 | (web) Accessible promotion dialog with clear piece labels | 02 §4 (`role="dialog"`, `aria-modal`, Queen focused, Escape cancels) | E + J |

§5 interaction rules → 02 §2 transition table (switch selection, deselect,
never alter position, clear on new snapshot). §6 visual checklist → A–I + J
(equal squares, consistent pieces, highlights visible not obscuring, no
stretch, accessible dialog with labels, distinguishable highlights,
whole-square click targets, keyboard playable).

## 2. Handoff notes for F4 (the contract consumer)

**What F4 receives** — the frozen contract (README §4):

```tsx
import type { GameSnapshot, PlayerColor, Promotion } from "@/types/protocol";

export interface BoardMove {
  from: string;
  to: string;
  promotion: Promotion | null; // null when not promoting — always present
}
export interface ChessBoardProps {
  snapshot: GameSnapshot;
  orientation: PlayerColor; // local player's color at the bottom
  disabled?: boolean;       // pending move / disconnected
  onMoveSubmit: (move: BoardMove) => void;
}
```

**What is intentionally left to F4** (do not build these):

- Wiring into the game screen: passing the snapshot from F3's store
  (`useGameState()`), `orientation` from the session color
  (`useRoomState().yourColor`), and `onMoveSubmit` → F3's `submitMove`.
- Pending-move state: F4 sets `disabled` while a submitted move awaits the
  next `game.state` (or its own in-flight lock); the board has no queue.
- Rejected-move UX: F3/F4 surface `game.move_rejected` (`ILLEGAL_MOVE`,
  `NOT_YOUR_TURN`, `STALE_REVISION`, `GAME_FINISHED`, …) as a themed toast;
  the board just keeps rendering the last authoritative `fen`.
- Turn indicator, connection banner, player panels, move history, result
  display — F4's screen per `src/screens/game/README.md` §4–5.
- `expected_revision` and the `{ v, id }` envelope when sending `game.move`:
  F3's store owns both (F3's `src/services/README.md` §5–6) — deliberately
  NOT in `BoardMove`.

## 3. Open questions to settle (Day 1, with F3/F4)

1. Contract shape — settled: the whole-`snapshot` form is frozen (this §2
   and F4's `01-screen-composition.md` §4 both hold it); the flat
   `fen`/`legalMoves`/`lastMove` alternative stays superseded (README §4
   history note).
2. Confirm `BoardMove` as the `game.move` payload minus `expected_revision`,
   and that F4 adds nothing else (callback name `onMoveSubmit` agreed?).
3. Promotion-dialog dismissal: keep selection (recommended) vs clear — pick
   once, document in 02 §4.
4. Not-your-turn UX: fully inert (recommended MVP) vs allowing selection to
   browse targets without submitting — F4 decides, board implements the
   agreed one.
5. Whether the dev harness file (`board-dev-harness.tsx`) is committed
   dev-only or dropped after integration (03 §K).
6. If a dedicated `selected-square` token is wanted beyond `border-primary`
   ring — small PR to `global.css` (F1 curates), optional.
7. Piece asset strategy: confirm the SVG upgrade's timing (02 §1) — it is
   the recommended strategy but not required for MVP acceptance; the
   `piece-white`/`piece-black` tokens it needs already exist (02 §1.1).
8. `src/components/common/` (F1's shared primitives, referenced by RULES.md
   §1.3 and `frontend/README.md`) does not exist yet; the promotion dialog is
   F2-local until it does (02 §4). Revisit if/when F1 ships a Dialog
   primitive.

## 4. Milestone sanity (from your brief §8)

Day 1 contract frozen (§3 above) → Day 2 board + pieces from fixture FEN
(01) → Days 3–4 selection + targets + callback + unit suites (02 §2, 03 §2)
→ Days 5–6 promotion, last move, check, orientation, keyboard polish
(02 §3–4, 01 §6) → Days 7–8 snapshot-reset + integration defects with F4 →
Days 9–10 browser QA desktop + phone-width (03 full run) + handoff
(this file §2).

A task is done only when its code is merged, its gates pass (typecheck +
lint + test + build), its interface is documented here, and F4 can
integrate it without reverse-engineering assumptions (master plan §16).
