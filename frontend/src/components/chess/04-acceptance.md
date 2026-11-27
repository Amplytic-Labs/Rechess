# 04 — Definition of done + handoff to F4

Line-by-line mapping of your brief
(`07_F2_Chessboard_Interaction.md` §2 "Required work", §5 interaction rules,
§6 visual checklist) to the doc set and the acceptance checks. Check an item
only after its `03-manual-testing.md` section passed on iOS AND Android and
`npm run typecheck` + `npm run lint` are clean.

## 1. DoD checklist — brief requirement → how → check

| # | Brief requirement (§2) | Covered by | Accept when |
|---|---|---|---|
| 1 | Board accepts an authoritative FEN position | 01 §2 `fenToPieces` + README §4 (`snapshot.fen` from `GameSnapshot`) | A: initial position; B: 1.e4 position |
| 2 | Render all 12 piece types reliably, one consistent asset strategy | 02 §1 (recommendation: Unicode MVP → SVG stretch) | A (all 12 types visible, iOS + Android) |
| 3 | White-at-bottom and Black-at-bottom; square mapping stays correct when flipped | 01 §3 `orientation.ts` | F: flipped board, taps still report correct square names |
| 4 | Tap own piece → show destinations from server-provided legal moves | 02 §2 state machine | C: only `legal_moves`-listed targets highlighted |
| 5 | On destination → `onMoveSubmit` with from/to/promotion; never mutate the board locally | 02 §2 (`BoardMove` = `game.move` payload minus `expected_revision`) | D: logger shows exact payload, board unchanged |
| 6 | Highlight last move, selected square, legal destinations, check king, disabled state | 02 §3 tokens `bg-last-move` / `bg-move-target` / `bg-check` / selected ring | B, C, G, H (distinguishable in both themes) |
| 7 | Promotion: explicit Q/R/B/N chooser, chosen value through the callback | 02 §4 (`Promotion` = `"q"\|"r"\|"b"\|"n"` from `@/types/protocol`) | E: picker before callback, `promotion: "r"` for Rook |
| 8 | Rejected-move feedback without visual desync | 02 §2 (no optimistic move — rejection is a no-op by construction) | D: board still renders previous `fen` |
| 9 | Scale to width, square squares, no clipping | 01 §5 `aspect-square w-full` + flex rows | A, I: square at all widths incl. 375 dp |
| 10 | Fixtures: initial / midgame / castling / en passant / promotion | 03 §1 local harness FENs (extra FENs live in the harness, NOT in F3's `protocol.fixtures.ts`) | A, E + spot-check castling/en-passant renders after server snapshots (02 §2: they are just moves in `legal_moves`, e.g. `"e1g1"`, `"e5d6"` — nothing special to code) |
| 11 | Stale selection/promotion cleared on snapshot change | 02 §2 reset on `snapshot.fen` change; on reconnect F4 passes the fresh snapshot | G/B: no stale highlights after switching snapshots in the harness |
| 12 | Pending: disable additional submissions (preferred for MVP) | 02 §5 `disabled` prop from F4 | G: `disabled: true` → inert |

§5 interaction rules → 02 §2 transition table (switch selection, deselect,
never alter position, clear on new snapshot). §6 visual checklist → A–I
(equal squares, consistent pieces, highlights visible not obscuring, no
stretch, accessible picker, distinguishable highlights, whole-square touch
targets).

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
  (`useRoomState().yourColor`), and `onMoveSubmit` → F3's `sendMove`.
- Pending-move state: F4 sets `disabled` while a submitted move awaits the
  next `game.state` (or its own in-flight lock); the board has no queue.
- Rejected-move UX: F3/F4 surface `game.move_rejected` (`ILLEGAL_MOVE`,
  `NOT_YOUR_TURN`, `STALE_REVISION`, `GAME_FINISHED`, …) as a themed toast;
  the board just keeps rendering the last authoritative `fen`.
- Turn indicator, connection banner, player panels, move history, result
  view — F4's screen per `src/screens/game/README.md` §4–5.
- `expected_revision` and the `{ v, id }` envelope when sending `game.move`:
  F3's store owns both (F3's `src/services/README.md` §5–6) — deliberately
  NOT in `BoardMove`.

**Stale references F4 should fix in their README via PR** (flagged, not
edited by us): §6 still says `npx jest game` — RULES.md §5 (no test runner by
design) supersedes it; §3's pointer "F2's ChessBoard props contract:
`src/components/chess/README.md` §4" remains valid — that section now holds
the snapshot-based contract above (the old flat `fen`/`legalMoves`/jest
proposal is superseded).

## 3. Open questions to settle (Day 1, with F3/F4)

1. Confirm the contract shape: whole `snapshot` (recommended) vs flat
   `fen`/`legalMoves`/`lastMove` props — then freeze and update both READMEs.
2. Confirm `BoardMove` as the `game.move` payload minus `expected_revision`,
   and that F4 adds nothing else (callback name `onMoveSubmit` agreed?).
3. Promotion-picker dismissal: keep selection (recommended) vs clear — pick
   once, document in 02 §4.
4. Not-your-turn UX: fully inert (recommended MVP) vs preview-selection — F4
   decides, board implements the agreed one.
5. Whether the dev harness file (`board-dev-harness.tsx`) is committed
   dev-only or dropped after integration (03 §J).
6. If a dedicated `selected-square` token is wanted beyond `border-primary`
   ring — small PR to `global.css` (F1 curates), optional.
7. Piece asset stretch goal: confirm the SVG upgrade (02 §1b) is in-scope
   before spending days on it — MVP acceptance does not require it.
8. `src/components/common/README.md` is referenced by RULES.md §2 /
   `frontend/README.md` but does not exist yet (only `styled.ts` does) — F1
   to add; affects nothing in this module beyond the reference.

## 4. Milestone sanity (from your brief §8)

Day 1 contract frozen (§3 above) → Day 2 board + pieces from fixture FEN
(01) → Days 3–4 selection + targets + callback (02 §2) → Days 5–6 promotion,
last move, check, orientation polish (02 §3–4, 01 §3) → Days 7–8
snapshot-reset + integration defects with F4 → Days 9–10 device QA (03 full
run) + handoff (this file §2).

A task is done only when its code is merged, its gates pass, its interface
is documented here, and F4 can integrate it without reverse-engineering
assumptions (master plan §16).
