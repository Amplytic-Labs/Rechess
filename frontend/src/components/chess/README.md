# `src/components/chess/` — Owner: **F2** (Chessboard and Interaction Components)

> **Read [`frontend/RULES.md`](../../../RULES.md) first** — it is binding
> (Tailwind CSS v4 `className` only, semantic tokens from `global.css` only,
> `"use client"` DOM components, CSS transitions for animation, no new
> dependency without a team PR). Then read this file completely before
> writing any code.

Welcome, F2. This is your module's complete onboarding doc set. It tells you
what to build, how to build it, and how to test it. Read it in order.

## 1. Your mission

Deliver the reusable 8×8 **chessboard** family that F4 embeds in the game
screen:

- the **board** (64 squares, square squares, both orientations) — a DOM
  client component: a CSS grid of square elements,
- the **pieces** (all 12 types, one asset strategy — inline SVG you author
  yourself, with Unicode glyphs as an acceptable starting point),
- **highlights** (selected square, legal move targets, last move, check),
- the **promotion picker** (Queen/Rook/Bishop/Knight chooser in a small
  dialog overlay).

You are NOT implementing chess rules. The Rust server is the only authority:
you render what `snapshot.fen` says, and you highlight only what
`snapshot.legal_moves` allows. Never compute legality, never mutate the
position locally — after a click you fire the callback and wait for the next
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
| Piece rendering strategy | Sockets and any `fetch`/API-route code — exactly one WS exists in the app, owned by F3; the Rust server is the only data source (RULES.md §2, §4) |
| Selection + highlight state (local UI state only) | Navigation (`next/navigation`, App Router routes) — a component never navigates |
| The promotion chooser UI | Chess-rules logic — no client-side legality engine, ever |

Never call a store (`useGameState`, `useRoomState`, …), never open a socket,
never fetch, never read `player_token`, never navigate. Your components
receive a `GameSnapshot` plus a move-submission callback from F4's game screen
and that is ALL they know about the world. This is why F4 can also render
your board inside a dev harness with pure fixture props (see
`03-manual-testing.md`).

## 3. This doc set

| File | What it covers | When to read |
|---|---|---|
| `01-board-architecture.md` | Component tree, CSS-grid layout, `fenToPieces` FEN adapter (exact algorithm + skeleton), orientation, square colors, sizing, the keyboard/focus model, rendering performance | Before writing the first file |
| `02-pieces-and-interaction.md` | Piece asset strategy (recommendation), the click-to-move state machine (pointer AND keyboard), highlights, promotion dialog, disabled states, optional drag-and-drop | Days 2–6 |
| `03-manual-testing.md` | vitest suites for the pure helpers + dev harness setup + the full MANUAL browser checklist (UI is verified manually, desktop and phone-width — no simulator) | Before you build, and before every PR |
| `04-acceptance.md` | Definition of done mapped line-by-line to your brief; handoff notes for F4; open contract questions | Before declaring done |

## 4. Prop contract (the contract F4's README points at)

> F4's `src/screens/game/01-screen-composition.md` §4 references "F2's
> contract (`src/components/chess/README.md` §4)" — that is this section.
> **The exact final signature is agreed with F4** (they compose the game
> screen and own the screen-side wiring). The shape below is the frozen
> default; any change goes through a PR that updates both READMEs.

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
"which fen" and "which legal moves" during an update.

**History — superseded proposals (clearly marked; kept so nobody
re-proposes them):**

- An earlier edition of this README (the pre-migration mobile-native stack)
  proposed `ChessBoardProps` with flat `fen` / `legalMoves` / `lastMove` /
  `checkSquare` props, an `onMove(from, to, promotion?)` callback, and a
  native-mobile rendering/test plan. All of it is **superseded**: the board
  takes the whole `GameSnapshot` (above), derives the checked-king square
  internally, reports through `onMoveSubmit`, and the stack is Next.js +
  Tailwind tokens + vitest-for-pure-logic. The flat-props form stays
  superseded — do not reintroduce it.
- Your original brief (07_F2_Chessboard_Interaction.md §4) suggests
  `checkedKingSquare?: string` as a prop and an `onMove(from, to, promotion?)`
  callback. Superseded: this doc set derives the king square from the FEN when
  `game_status === "check"` (02 §3 — that is board math, not rules), and uses
  a single `onMoveSubmit({ from, to, promotion })` object callback that
  matches the wire payload exactly.

## 5. Definition of done (summary — full mapping in `04-acceptance.md`)

- [ ] `ChessBoard` renders any `snapshot.fen` in both orientations, all 12
      piece types, on a perfectly square board that scales to width.
- [ ] Click-to-move: select own piece → legal destinations from
      `snapshot.legal_moves` only → submit via `onMoveSubmit` with the exact
      `game.move` payload shape.
- [ ] Keyboard operable (RULES.md §2): squares focusable via roving tabindex
      (01 §6), Enter/Space picks and places, Escape cancels a selection,
      visible focus ring in both themes.
- [ ] Highlights: selected, legal targets (`move-target`), last move
      (`last-move`), check (`check`), all from board tokens in `global.css`,
      all working in light AND dark mode.
- [ ] Promotion dialog asks before submitting; `promotion` is `null` for
      non-promotion moves; the Queen choice is the default.
- [ ] Board is inert when it isn't the local player's turn and when the game
      is finished; local selection state resets when the snapshot changes.
- [ ] vitest: the pure helpers (`fen.ts`, `orientation.ts`, `move-targets.ts`)
      are covered by their `.test.ts` siblings — `npm run test` passes.
- [ ] `npm run typecheck`, `npm run lint`, `npm run test` and `npm run build`
      clean; the manual checklist in `03-manual-testing.md` passed in the
      browser (desktop + phone-width), noted in the PR.
- [ ] This README §4 updated with the final agreed prop names.

## 6. Quickstart

```bash
cd frontend
npm install            # once — lockfile is committed
npm run typecheck      # tsc --noEmit — MUST be zero errors BEFORE you start
npm run lint           # eslint — MUST be zero errors/warnings BEFORE you start
npm run test           # vitest — MUST be green BEFORE you start
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
`03-manual-testing.md` §1 (a scratch component in your own folder, temporarily
mounted from a route file, reverted before merge).

Quality gates before every push (RULES.md §5): `npm run typecheck` +
`npm run lint` + `npm run test` + `npm run build`, plus your module's manual
browser checklist. vitest covers **pure logic only** (node environment, no
DOM rendering) — the UI itself is verified manually (03).

## 7. Key coordination

- **F3** owns `src/types/protocol.ts`, the stores and the hooks. If a
  snapshot field is missing for you, ask F3 — never patch the type yourself.
- **F4** composes your board; prop names are a contract (§4 above). Agree on
  Day 1, freeze, change only via PR.
- **F1** curates `global.css` tokens; if you need a new board token (e.g. a
  dedicated `selected-square` — 02 §1.1), that's a small PR against
  `global.css`, not a local hardcode. (`piece-white`/`piece-black` already
  exist for the SVG upgrade.)
- Piece asset strategy stays inside `src/components/chess/` — swapping it
  must never change the component contract.
