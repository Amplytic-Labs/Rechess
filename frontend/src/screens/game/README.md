# `src/screens/game/` — Owner: **F4** (Game Screen and Frontend Integration)

> **Read [`frontend/RULES.md`](../../../RULES.md) first** — it is binding
> (no StyleSheet, no expo-status-bar, tokens only, one WS connection, server
> authority). Then read this doc set, in order, before writing any code.

This folder is documented as a **set of numbered docs**. This README is the
index; the numbered files are the actual instructions.

| # | File | What it covers |
|---|---|---|
| — | `README.md` (this file) | Mission, boundaries, definition of done, quickstart |
| 1 | `01-screen-composition.md` | Route file, screen layout, F2 board wiring, F3 data wiring, skeleton with TODOs |
| 2 | `02-controls-and-results.md` | Draw offer / resign / rematch flows, result view, error-code message table |
| 3 | `03-connection-states.md` | Pending moves, rejected moves, opponent presence, reconnect banner, disabled states |
| 4 | `04-manual-testing.md` | The full MANUAL test checklist (there is no jest — RULES.md §5) |
| 5 | `05-acceptance.md` | DoD mapped line-by-line to your brief + handoff notes and flagged gaps |

---

## 1. Your mission

You own the **in-game screen**: everything the player sees after both players
are in the room. The screen **composes** three other teammates' work:

- **F2's `ChessBoard`** (presentational) — you pass it the authoritative
  FEN/legal moves and it reports attempted moves back to you.
- **F3's hooks/actions** (one WebSocket connection, server-authoritative
  zustand stores) — the screen never opens a socket and never keeps a second
  copy of game state.
- **F1's lobby + common components** — the user arrives at your screen from
  the waiting room and leaves through navigation you coordinate with F1.

Concretely you build: the game screen composition, player info bars,
turn indicator, move history list, draw/resign/rematch controls, the
result view, connection/presence banners, pending-move and disabled states.

## 2. Ownership boundaries

| You own | You do NOT own (compose / consume only) |
|---|---|
| `src/screens/game/**` (all files in this folder) | `src/components/chess/**` — **F2's board**; embed via its props contract, never edit |
| `src/app/game.tsx` — the game **route file** (thin wrapper, RULES.md §3) | `src/services/`, `src/store/`, `src/hooks/`, `src/types/` — **F3's**; consume the hooks, never the raw socket |
| Player panels, turn indicator, move history | `src/screens/lobby/**` + lobby routes — **F1's**; agree navigation in/out with F1 |
| Draw/resign/rematch controls + confirmations | `src/components/common/` — **F1's**; request changes via PR (e.g. a missing button variant) |
| Connection banner, result view, pending/disabled states | `global.css` / `src/theme/` — shared; new tokens only via PR |
| Result-reason and error-code → friendly message maps (pure, in this folder) | Chess rules, legality, FEN mutation — the server is the only authority (RULES.md §4) |

## 3. Ground rules that apply to every line you write

1. **uniwind `className` only** — no `StyleSheet`, no `style` prop except a
   minimal documented dynamic value (RULES.md §1).
2. **Tokens only** for every color: `background`, `surface`, `surface-muted`,
   `foreground`, `muted-foreground`, `primary`, `primary-foreground`,
   `border`, `destructive`, `success` (board tokens `board-light`,
   `board-dark`, `move-target`, `last-move`, `check` are F2's concern).
   No `bg-[#…]`, no color literals — dark mode must work with zero extra work.
3. **Never optimistically apply a move.** After `onMove` fires, set nothing on
   the board yourself — F3's store marks `pending` and the next `game.state`
   snapshot updates the board (RULES.md §4, protocol §5).
4. **Map every `ErrorCode` to a friendly themed message** — never show raw
   JSON, never show the wire `message` as-is if you have a mapping (see
   `02-controls-and-results.md` §5).
5. **`player_token` never** in route params, logs, or anywhere you touch.
   You will likely never even see the token — F3 keeps it in secure storage.
6. **HeroUI Native granular imports only** (`heroui-native/button`, never the
   root import); prefer a HeroUI component over hand-rolling a control.
7. **No jest, no test files.** Testing = `npm run typecheck` +
   `npm run lint` + the manual checklist in `04-manual-testing.md`
   (RULES.md §5 — this REPLACES the "component tests" mentioned in the
   original briefs).

## 4. Definition of done

See `05-acceptance.md` for the line-by-line mapping to your brief. In short,
you are done when:

- [ ] `src/app/game.tsx` (thin route) + a complete game screen composition in
      this folder render the full in-game experience.
- [ ] The board, history, panels and turn indicator update **only** from
      F3's server-derived state (no local board/turn copy).
- [ ] Draw offer / accept, resign (with confirmation dialog) and rematch
      (with waiting-for-opponent state) all work through F3's actions.
- [ ] Result view maps every `GameResultReason`; every `ErrorCode` maps to a
      friendly message; rejected moves never patch the board.
- [ ] Pending-move, not-your-turn, opponent-disconnected and
      reconnecting states are visible and disable the board correctly.
- [ ] Two-device end-to-end pass recorded (checklist in
      `04-manual-testing.md` §3) with defects noted for the team.
- [ ] `npm run typecheck` and `npm run lint` pass with zero errors/warnings.

## 5. Quickstart

```bash
cd frontend
npm install          # once
npm run typecheck    # tsc --noEmit — run this constantly
npm run lint         # eslint (enforces RULES.md)

npm start            # Expo dev server → Expo Go on your phone
npm run android      # or simulator
```

Build order (matches your brief's integration plan):

1. **Day 1–2 — fixtures, no backend.** Render the screen with
   `sampleSnapshot` / `sampleFinishedSnapshot` from
   `@/types/protocol.fixtures` instead of live hooks (the props-injected dev
   variant documented in `04-manual-testing.md` §1). Wire F2's `ChessBoard`
   with a fixture FEN.
2. **Day 3–4 — real state.** Swap the dev variant for F3's real hooks
   (`useGameState`, `useRoomState`, `useRoomActions`, `useConnection` — exact
   names in `01-screen-composition.md` §6) and prove two-device move sync
   against the dev backend (`cd backend && cargo run`).
3. **Day 5+ — controls, results, connection UX**, then the full manual QA
   pass.

Verify continuously with `npm run typecheck` — the typed fixtures in
`src/types/protocol.fixtures.ts` double as realistic mock data, and
typechecking proves your screen compiles against the real wire types.
