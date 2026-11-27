# `src/screens/game/` — Owner: **F4** (Game Screen and Frontend Integration)

> **Read [`frontend/RULES.md`](../../../RULES.md) first** — it is binding
> (Tailwind `className` only, tokens only, one WebSocket connection, server
> authority). Then read this doc set, in order, before writing any code.

This folder is documented as a **set of numbered docs**. This README is the
index; the numbered files are the actual instructions.

| # | File | What it covers |
|---|---|---|
| — | `README.md` (this file) | Mission, boundaries, definition of done, quickstart |
| 1 | `01-screen-composition.md` | Route file, responsive screen layout, F2 board wiring, F3 data wiring, skeleton with TODOs |
| 2 | `02-controls-and-results.md` | Draw offer / resign / rematch flows, result view, error-code message table |
| 3 | `03-connection-states.md` | Pending moves, rejected moves, opponent presence, reconnect banner, disabled states |
| 4 | `04-manual-testing.md` | The full MANUAL browser test checklist (UI is manual-tested — RULES.md §5) |
| 5 | `05-acceptance.md` | DoD mapped line-by-line to your brief + handoff notes and flagged gaps |

---

## 1. Your mission

You own the **in-game screen**: everything the player sees after entering the
room. The screen **composes** three other teammates' work:

- **F2's `ChessBoard`** (presentational) — you pass it the whole server
  snapshot plus your color, and it reports attempted moves back to you.
- **F3's hooks/actions** (one WebSocket connection, server-authoritative
  zustand stores) — the screen never opens a socket and never keeps a second
  copy of game state.
- **F1's lobby + common components** — the user arrives at your screen from
  the lobby and leaves through navigation you coordinate with F1; buttons,
  dialogs and toasts come from their shared primitives.

One route covers the whole experience: `/game/[roomCode]` renders the
waiting state, the live game and the result view — all driven by F3's store
state (room status + snapshot status). There are no other game routes.

Concretely you build: the game screen composition, player info bars,
turn indicator, move history list, draw/resign/rematch controls, the
result view, connection/presence banners, pending-move and disabled states.

## 2. Ownership boundaries

| You own | You do NOT own (compose / consume only) |
|---|---|
| `src/screens/game/**` (all files in this folder) | `src/components/chess/**` — **F2's board**; embed via its props contract, never edit |
| `src/app/game/[roomCode]/page.tsx` — the game **route file** (thin wrapper, RULES.md §3) | `src/services/`, `src/store/`, `src/hooks/`, `src/types/` — **F3's**; consume the hooks, never the raw socket |
| Player panels, turn indicator, move history, waiting/result states | `src/screens/lobby/**` + lobby routes — **F1's**; agree navigation in/out with F1 |
| Draw/resign/rematch controls + confirmations | `src/components/common/` — **F1's shared UI primitives** (Button, Dialog, Toast…); request changes via PR, never hand-roll copies |
| Connection banner, result view, pending/disabled states | `global.css` / `src/theme/` — shared; new tokens only via PR |
| Result-reason and error-code → friendly message maps (pure, in this folder) | Chess rules, legality, FEN mutation — the server is the only authority (RULES.md §4) |

## 3. Ground rules that apply to every line you write

1. **Tailwind `className` only** — no `style` prop except a minimal
   documented dynamic value that cannot be a utility (RULES.md §1).
2. **Tokens only** for every color: `background`, `surface`, `surface-muted`,
   `foreground`, `muted-foreground`, `primary`, `primary-foreground`,
   `border`, `destructive`, `success` (board tokens `board-light`,
   `board-dark`, `move-target`, `last-move`, `check` are F2's concern).
   No `bg-[#…]`, no color literals — dark mode follows the system with zero
   per-component work (it just works via `prefers-color-scheme`).
3. **Never optimistically apply a move.** After `onMoveSubmit` fires, set nothing on
   the board yourself — F3's store marks `pending` and the next `game.state`
   snapshot updates the board (RULES.md §4, protocol §5).
4. **Map every `ErrorCode` to a friendly themed message** — never show raw
   JSON, never show the wire `message` as-is if you have a mapping (see
   `02-controls-and-results.md` §5).
5. **`player_token` never** in URLs, logs, or anywhere you touch.
   You will likely never even see the token — F3 keeps it in its storage
   module.
6. **Client component, thin route.** `game-screen.tsx` starts with
   `"use client"` (the app is a real-time WebSocket client, RULES.md §2);
   `src/app/game/[roomCode]/page.tsx` stays a thin server wrapper that
   renders it. Buttons, dialogs and toasts come from F1's
   `src/components/common/` primitives — never hand-rolled duplicates.
7. **The Rust WebSocket server is the only data source.** No Next API
   routes, no server actions, no `fetch` — all reads and writes go through
   F3's hooks/actions (RULES.md §2, §4).
8. **Quality gates before every push:** `npm run typecheck` + `npm run lint`
   + `npm run test` + `npm run build` (RULES.md §5; CI runs the same).
   vitest exists for **pure logic only and F3 owns that suite** — do not add
   test files in this folder without a team decision. Your UI is verified
   manually in the browser via the checklist in `04-manual-testing.md`
   (this REPLACES the "component tests" mentioned in the original briefs).

## 4. Definition of done

See `05-acceptance.md` for the line-by-line mapping to your brief. In short,
you are done when:

- [ ] `src/app/game/[roomCode]/page.tsx` (thin route) + a complete game
      screen composition in this folder render the full experience —
      waiting, live game and result all on this one route.
- [ ] The board, history, panels and turn indicator update **only** from
      F3's server-derived state (no local board/turn copy).
- [ ] Draw offer / accept, resign (with confirmation dialog) and rematch
      (with waiting-for-opponent state) all work through F3's actions.
- [ ] Result view maps every `GameResultReason`; every `ErrorCode` maps to a
      friendly message; rejected moves never patch the board.
- [ ] Pending-move, not-your-turn, opponent-disconnected and
      reconnecting states are visible and disable the board correctly.
- [ ] Two-browser end-to-end pass recorded (checklist in
      `04-manual-testing.md` §2) with defects noted for the team.
- [ ] `npm run typecheck`, `npm run lint`, `npm run test` and
      `npm run build` pass with zero errors/warnings.

## 5. Quickstart

```bash
cd frontend
npm install          # once
npm run typecheck    # tsc --noEmit — run this constantly
npm run lint         # eslint (enforces RULES.md)
npm run test         # vitest — F3's pure-logic suite must stay green
npm run build        # production build (CI's gate)

npm run dev          # http://localhost:3001 — the Rust backend owns :3000
```

Build order (matches your brief's integration plan):

1. **Day 1–2 — fixtures, no backend.** Render the screen with
   `sampleSnapshot` / `sampleFinishedSnapshot` from
   `@/types/protocol.fixtures` instead of live hooks (the dev-only fixture
   variant documented in `04-manual-testing.md` §1). Wire F2's `ChessBoard`
   with the fixture snapshot.
2. **Day 3–4 — real state.** Swap the dev variant for F3's real hooks
   (`useGameState`, `useRoomState`, `useRoomActions`, `useConnection` — exact
   names in `01-screen-composition.md` §6) and prove two-browser move sync
   against the dev backend (`cd backend && cargo run`).
3. **Day 5+ — controls, results, connection UX**, then the full manual QA
   pass.

Verify continuously with `npm run typecheck` — the typed fixtures in
`src/types/protocol.fixtures.ts` double as realistic mock data, and
typechecking proves your screen compiles against the real wire types.
