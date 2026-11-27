# `src/components/chess/` — Owner: **F2** (Chessboard and Interaction Components)

> **Read [`frontend/RULES.md`](../../../RULES.md) first** — it is binding
> (no StyleSheet, uniwind `className` only, tokens only, no
> `expo-status-bar`, Reanimated not `Animated`). Then read this file
> completely before writing any code.

Welcome, F2. This is your module's complete onboarding doc set. It tells you
what to build, how to build it, and how to test it. Read it in order.

## 1. Your mission

Deliver the reusable 8×8 **chessboard** family that F4 embeds in the game
screen:

- the **board** (64 squares, square squares, both orientations),
- the **pieces** (all 12 types, one asset strategy),
- **highlights** (selected square, legal move targets, last move, check),
- the **promotion picker** (Queen/Rook/Bishop/Knight chooser).

You are NOT implementing chess rules. The Rust server is the only authority:
you render what `snapshot.fen` says, and you highlight only what
`snapshot.legal_moves` allows. Never compute legality, never mutate the
position locally — after a tap you fire the callback and wait for the next
snapshot (RULES.md §4).

## 2. Boundaries — presentational, always

Your components are **PRESENTATIONAL**. Data comes in via props; intents go
out via one callback. That is the whole architecture (master plan §8: the
board reports the attempted move; the state layer sends the command; the
server response updates the board).

| You own | You do NOT touch |
|---|---|
| `src/components/chess/**` (all files, all decisions) | `src/screens/**` (F1 lobby, F4 game) |
| The FEN → squares rendering adapter (`fenToPieces`) | `src/services/`, `src/store/`, `src/hooks/` (F3) — never import them |
| Orientation/flipping helpers | `src/types/protocol.ts` (F3) — import from it, never edit it |
| Piece rendering strategy | Sockets — exactly one WS exists in the app, owned by F3 |
| Selection + highlight state (local UI state only) | Navigation (`expo-router`) — a component never navigates |
| The promotion chooser UI | Chess-rules logic — no client-side legality engine, ever |

Never call a store (`useGameState`, `useRoomState`, …), never open a socket,
never read `player_token`, never navigate. Your components receive a
`GameSnapshot` plus a move-submission callback from F4's game screen and that
is ALL they know about the world. This is why F4 can also render your board
inside a dev harness with pure fixture props (see `03-manual-testing.md`).

## 3. This doc set

| File | What it covers | When to read |
|---|---|---|
| `01-board-architecture.md` | Component tree, `fenToPieces` FEN adapter (exact algorithm + skeleton), orientation, square colors, sizing, rendering performance | Before writing the first file |
| `02-pieces-and-interaction.md` | Piece asset options (recommendation), the tap-to-move state machine, highlights, promotion picker, disabled states, optional drag-and-drop | Days 2–6 |
| `03-manual-testing.md` | Dev harness setup + the full MANUAL checklist (there is no jest on this frontend — by design) | Before you build, and before every PR |
| `04-acceptance.md` | Definition of done mapped line-by-line to your brief; handoff notes for F4; open contract questions | Before declaring done |

## 4. Prop contract (the contract F4's README points at)

> F4's `src/screens/game/README.md` §3 references "F2's ChessBoard props
> contract: `src/components/chess/README.md` §4" — that is this section.
> **The exact final signature is agreed with F4** (they compose the game
> screen and own the screen-side wiring). The shape below is the
> recommended default: confirm it with F4 on Day 1, then freeze it. Any
> change afterwards goes through a PR that updates both READMEs.

```tsx
import type { GameSnapshot, PlayerColor, Promotion } from "@/types/protocol";

/**
 * Exactly the `game.move` payload from `src/types/protocol.ts` minus
 * `expected_revision` — F3's store owns the revision and the envelope
 * (`v`, `id`) when it sends. The board emits only the move itself.
 */
export interface BoardMove {
  from: string;                // "e2" — square name
  to: string;                  // "e4"
  promotion: Promotion | null; // "q" | "r" | "b" | "n", or null when not
                               // promoting — the wire field is ALWAYS
                               // present, so it is null, never undefined
}

export interface ChessBoardProps {
  /** The whole authoritative snapshot — F4 passes it from F3's store. */
  snapshot: GameSnapshot;
  /**
   * Which color sits at the bottom = the LOCAL player's color.
   * F4 gets it from F3's session state (`useRoomState().yourColor`) and
   * passes it down. The board cannot know which player is local — it must
   * never guess from `snapshot.players`.
   */
  orientation: PlayerColor; // "white" | "black"
  /**
   * Optional kill-switch for states NOT visible in the snapshot (a pending
   * submitted move, a dropped connection). Turn/finished logic the board
   * derives itself from the snapshot.
   */
  disabled?: boolean;
  /** Submit an attempted move. The board never applies it locally. */
  onMoveSubmit: (move: BoardMove) => void;
}
```

Why the whole `GameSnapshot` instead of flat `fen` + `legalMoves` + `lastMove`
props: F4 already has one snapshot object from F3's store, the snapshot is
the unit the server sends (see `docs/protocol-v1.md` §4.3), and one prop
means one memo identity — cheap re-render checks and zero drift between
"which fen" and "which legal moves" during an update. If F4 prefers the flat
form anyway, agree once and update this §4 + F4's README in the same PR.

**Flagged mismatches you should know about (fix via PR, don't edit others'
files):**

- The previous version of this README proposed `ChessBoardProps` with flat
  `fen` / `legalMoves` / `lastMove` / `checkSquare` props, `onMove`, jest
  tests, and tokens in `tailwind.config.js`. All of that is **superseded**:
  uniwind 1.14 + `global.css` tokens, the snapshot-based contract above, and
  no test runner.
- F4's README §6 mentions `npx jest game` — stale: RULES.md §5 says the
  frontend has **no unit-test runner by design** (typecheck + lint + manual
  checklists). Point it out to F4 so they can fix their README; do not add
  jest yourself.
- Your original brief (07_F2_Chessboard_Interaction.md) suggests
  `checkedKingSquare?: string` as a prop and an `onMove(from, to, promotion?)`
  callback. This doc set instead derives the king square from the FEN when
  `game_status === "check"` (02 §3 — that is board math, not rules), and uses
  a single `onMoveSubmit({ from, to, promotion })` object callback that
  matches the wire payload exactly. Confirm with F4, then freeze.

## 5. Definition of done (summary — full mapping in `04-acceptance.md`)

- [ ] `ChessBoard` renders any `snapshot.fen` in both orientations, all 12
      piece types, on a perfectly square board that scales to width.
- [ ] Tap-to-move: select own piece → legal destinations from
      `snapshot.legal_moves` only → submit via `onMoveSubmit` with the exact
      `game.move` payload shape.
- [ ] Highlights: selected, legal targets (`move-target`), last move
      (`last-move`), check (`check`), all from board tokens in `global.css`,
      all working in light AND dark mode.
- [ ] Promotion picker asks before submitting; `promotion` is `null` for
      non-promotion moves.
- [ ] Board is inert when it isn't the local player's turn and when the game
      is finished; local selection state resets when the snapshot changes.
- [ ] `npm run typecheck` and `npm run lint` clean; the manual checklist in
      `03-manual-testing.md` passed on iOS and Android, noted in the PR.
- [ ] This README §4 updated with the final agreed prop names.

## 6. Quickstart

```bash
cd frontend
npm install            # once — lockfile is committed
npm run typecheck      # tsc --noEmit — MUST be zero errors BEFORE you start
npm run lint           # eslint — MUST be zero errors/warnings BEFORE you start
```

Record that clean baseline first, so any error you see later is yours. Then
read `01-board-architecture.md` and build the board against fixture data —
no server needed:

```tsx
import { sampleSnapshot } from "@/types/protocol.fixtures";
// render <ChessBoard snapshot={sampleSnapshot} orientation="black"
//                    onMoveSubmit={(m) => console.log(m)} />
// (position after 1.e4 — black to move, e7-pawn has moves e7e5/e7e6)
```

How to actually see it on screen: the dev-harness approach in
`03-manual-testing.md` §1 (a scratch screen in your own folder, temporarily
pointed at by `src/app/index.tsx`, reverted before merge).

Quality gates before every push (RULES.md §5): `npm run typecheck` +
`npm run lint` + your module's manual checklist. There is **no jest on this
frontend by design** — never write test files; see RULES.md §5 for why.

## 7. Key coordination

- **F3** owns `src/types/protocol.ts`, the stores and the hooks. If a
  snapshot field is missing for you, ask F3 — never patch the type yourself.
- **F4** composes your board; prop names are a contract (§4 above). Agree on
  Day 1, freeze, change only via PR.
- **F1** curates `global.css` tokens; if you need a new board token (e.g. a
  dedicated `selected-square` color), that's a small PR against
  `global.css`, not a local hardcode.
- Piece asset strategy stays inside `src/components/chess/` — swapping it
  must never change the component contract.
