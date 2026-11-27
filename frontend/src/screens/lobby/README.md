# `src/screens/lobby/` — Owner: **F1** (App Foundation and Lobby)

> **Read [`RULES.md`](../../../RULES.md) first** — it is binding for humans and
> AI assistants (no `StyleSheet`, no `expo-status-bar`, tokens only,
> expo-router only, server-authoritative state). Then read this doc set
> completely before writing code.

## 0. Doc set index (read in this order)

| File | What it covers |
|---|---|
| `README.md` (this file) | mission, boundaries, what already exists, quickstart |
| [`01-screens-guide.md`](./01-screens-guide.md) | HOW to build home / create / join / waiting screens |
| [`02-routes-and-navigation.md`](./02-routes-and-navigation.md) | route files, `router.push`, typed params, flow diagram |
| [`03-manual-testing.md`](./03-manual-testing.md) | the manual test checklists (replaces unit tests) |
| [`04-acceptance.md`](./04-acceptance.md) | definition of done + handoff contracts to F3 / F4 |

## 1. Your mission

Deliver everything that happens **before** the game: the lobby home screen,
the create-room flow, the join-room flow and the waiting room, plus the thin
Expo Router route files that render them. A guest can open the app, choose a
nickname, create or join a room, see the room code and the waiting state, and
hand off into the game — with no account flow anywhere.

You are **not** implementing the chessboard (F2), the WebSocket client or
stores (F3), or the game screen (F4). Your screens call F3's hooks and never
open a socket (RULES.md §4).

## 2. Ownership boundaries

| You own (write freely) | You do NOT own (PR / coordinate instead) |
|---|---|
| `src/screens/lobby/**` — all four screen components + local validation helpers | `src/components/common/` — shared, F1 curates; additions/changes go through a short PR (RULES.md §0) |
| Lobby route files in `src/app/`: `index.tsx`, `create-room.tsx`, `join-room.tsx`, `waiting-room.tsx` (thin wrappers only) | `src/services/`, `src/store/`, `src/types/`, `src/security/`, `src/hooks/` — **F3** |
| | `src/screens/game/` + the game route — **F4** |
| | `src/components/chess/` — **F2** |
| | `src/app/_layout.tsx`, `src/theme/`, `global.css`, configs (`app.json`, `package.json`, …) — already done; changes via PR |

Working rule: if a screen genuinely needs something from a folder you don't
own (a new `withUniwind` wrapper, a new hook shape), open a PR or talk to the
owner — do not edit their folder in place.

## 3. What is already done (do not rebuild)

- **App shell**: `src/app/_layout.tsx` mounts
  `StyledGestureHandlerRootView` → `SafeAreaProvider` →
  `HeroUINativeProvider` → `Stack`. Theming needs **no provider**: tokens
  live in `global.css` and follow the system theme.
- **Styling**: uniwind 1.14 (Tailwind CSS v4) via `className` only. Semantic
  tokens in `frontend/global.css` — `background`, `surface`,
  `surface-muted`, `foreground`, `muted-foreground`, `primary`,
  `primary-foreground`, `border`, `destructive`, `success`, `board-light`,
  `board-dark`, `move-target`, `last-move`, `check`. No
  `tailwind.config.js`, no ThemeProvider; dark mode is automatic.
- **Third-party `className` interop**: `StyledSafeAreaView` and
  `StyledGestureHandlerRootView` in `src/components/common/styled.ts`.
- **Wire contract**: `docs/protocol-v1.md` materialized as TypeScript in
  `src/types/protocol.ts` (F3 owns), with typed mock data in
  `src/types/protocol.fixtures.ts` (`sampleSnapshot`, `sampleRoom`,
  `sampleFinishedSnapshot`, `sampleRoomCreatedEvent`, `sampleMoveCommand`).
  Build your UI against these fixtures before the backend/F3 exist.
- **HeroUI Native 1.1** installed with peers; import granularly
  (`heroui-native/button`, `heroui-native/card`, `heroui-native/input`, …).
- **`expo-clipboard ~58.0.4`** already in `package.json` — use it for
  copy-room-code (see `01-screens-guide.md` §Waiting room).
- A **placeholder home** at `src/app/index.tsx` — you replace its body.

## 4. Definition of done (summary — full checklist in `04-acceptance.md`)

- Home, create-room, join-room and waiting-room screens render with token-only
  styling, in light AND dark mode, with thin route files.
- Nickname (1–20 chars, no control characters) and room-code (8 chars,
  unambiguous alphabet) validation runs client-side before F3 actions.
- Lobby error codes map to friendly themed messages; buttons have
  disabled + loading states; no double submits.
- `npm run typecheck` and `npm run lint` pass with zero errors/warnings.
- The manual test checklist in `03-manual-testing.md` has been walked through
  and noted in the PR description.
- No `player_token` ever appears in a route param, deep link or log.

## 5. Day-one quickstart

```bash
cd frontend
npm install                      # once — lockfile is committed
npm run typecheck                # baseline: must pass on the untouched tree
npm run lint                     # baseline: must pass on the untouched tree
npm start                        # Expo dev server → QR for Expo Go, or i/a for simulators
```

Then:

1. Read `RULES.md`, this file, then `01-screens-guide.md` →
   `02-routes-and-navigation.md` → `03-manual-testing.md` → `04-acceptance.md`.
2. Skim the real files your screens sit on: `src/app/_layout.tsx`,
   `src/app/index.tsx`, `src/components/common/styled.ts`, `global.css`,
   `src/types/protocol.ts`, `src/types/protocol.fixtures.ts`.
3. Run the app (`npm start`) and see the placeholder home screen — that is
   your starting point.
4. Build the home screen first (smallest), then create → join → waiting,
   validating with typecheck + lint after each screen.
