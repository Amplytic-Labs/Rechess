# `src/screens/lobby/` — Owner: **F1** (App Foundation and Lobby)

> **Read [`RULES.md`](../../../RULES.md) first** — it is binding for humans and
> AI assistants (Tailwind `className` only, tokens only, no component library,
> `next/navigation` only, server-authoritative state). Then read this doc set
> completely before writing code.

## 0. Doc set index (read in this order)

| File | What it covers |
|---|---|
| `README.md` (this file) | mission, boundaries, what already exists, quickstart |
| [`01-screens-guide.md`](./01-screens-guide.md) | HOW to build the home screen, the waiting room and the shared UI primitives |
| [`02-routes-and-navigation.md`](./02-routes-and-navigation.md) | the two fixed routes, `Link`/`useRouter`/`useParams`, flow diagram |
| [`03-manual-testing.md`](./03-manual-testing.md) | the browser manual test checklists (UI has no unit tests) |
| [`04-acceptance.md`](./04-acceptance.md) | definition of done + handoff contracts to F3 / F4 |

## 1. Your mission

Deliver everything that happens **before** the game, plus the shared UI
primitives the whole app is built from. A guest can open the app in a browser,
choose a nickname, create or join a room, see the room code and the waiting
state, and land in the game — with no account flow anywhere.

Concretely, three deliverables:

1. **The home screen** at `/` (`src/screens/lobby/home-screen.tsx`) — one
   screen with the nickname entry, the primary **Create room** action and the
   join section (room code + nickname). Both actions navigate to
   `/game/[roomCode]` on success.
2. **The waiting room** (`src/screens/lobby/waiting-room.tsx`) — not a route of
   its own: it is the panel rendered at `/game/[roomCode]` while the room
   status is `"waiting"` (see `02-routes-and-navigation.md`).
3. **The shared UI primitives** in `src/components/common/` — `Button`,
   `Input`, `Card`, `Dialog`, `Toast`. There is no component library in this
   stack; F1 builds these once and every module imports them
   (`01-screens-guide.md` §1).

You are **not** implementing the chessboard (F2), the WebSocket client or
stores (F3), or the game screen composition (F4). Your screens call F3's
hooks and never open a socket (RULES.md §4).

## 2. Ownership boundaries

| You own (write freely) | You do NOT own (PR / coordinate instead) |
|---|---|
| `src/screens/lobby/**` — the home screen, the waiting room + local validation helpers | `src/services/`, `src/store/`, `src/types/`, `src/security/`, `src/hooks/` — **F3** |
| `src/app/page.tsx` — the home route file (a thin wrapper over `HomeScreen`) | `src/screens/game/**` + `src/app/game/**` — **F4** |
| `src/components/common/**` — the shared UI primitives (initial set: `Button`, `Input`, `Card`, `Dialog`, `Toast`; RULES.md §0) | `src/components/chess/` — **F2** |
| | `src/app/layout.tsx`, `src/theme/`, `global.css`, configs (`next.config.ts`, `tsconfig.json`, `eslint.config.mjs`, `vitest.config.ts`, `package.json`) — shared; changes via PR. RULES.md §1.3: primitives beyond the initial set, and changes to the shared surface, also go through a short PR |

Working rule: if a screen genuinely needs something from a folder you don't
own (a new hook shape, a new token, a new primitive), open a PR or talk to
the owner — do not edit their folder in place.

## 3. What is already done (do not rebuild)

- **App shell**: `src/app/layout.tsx` (root layout) imports `global.css` and
  paints `<body className="bg-background text-foreground">`. Theming needs
  **no provider**: tokens live in `global.css` and follow the system theme.
- **Routes (FIXED — do not add more)**: `/` (`src/app/page.tsx`, a placeholder
  you replace) and `/game/[roomCode]`
  (`src/app/game/[roomCode]/page.tsx`, **F4's** placeholder). No separate
  create / join / waiting / result routes — see
  `02-routes-and-navigation.md`.
- **Styling**: Tailwind CSS v4 via `@tailwindcss/turbopack` — `className`
  only, CSS-first. Semantic tokens in `frontend/global.css`: `background`,
  `surface`, `surface-muted`, `foreground`, `muted-foreground`, `primary`,
  `primary-foreground`, `border`, `destructive`, `success`, and the board
  tokens (`board-light`, `board-dark`, `move-target`, `last-move`, `check` —
  F2's). No `tailwind.config.js`, no theme provider; dark mode is automatic
  (`prefers-color-scheme`).
- **No component library**: `src/components/common/` does not exist yet —
  you create it with the initial primitive set (`01-screens-guide.md` §1).
  Never hand-roll per-screen copies, never install a UI kit (new dependency
  = team PR).
- **Wire contract**: `docs/protocol-v1.md` materialized as TypeScript in
  `src/types/protocol.ts` (F3 owns), with typed mock data in
  `src/types/protocol.fixtures.ts` (`sampleSnapshot`, `sampleRoom`,
  `sampleFinishedSnapshot`, `sampleRoomCreatedEvent`, `sampleMoveCommand`).
  Build your UI against these fixtures before the backend/F3 exist.
- **Toolchain**: `npm run typecheck` / `lint` / `test` (vitest — the
  wire-contract suite in `src/types/protocol.test.ts`) / `build` all pass on
  the untouched tree; ESLint (flat config) enforces RULES.md (arbitrary color
  literals like `bg-[#…]` are banned); dependencies are pinned with a
  committed lockfile.

## 4. Definition of done (summary — full checklist in `04-acceptance.md`)

- Home screen renders nickname + create + join with token-only styling, in
  light AND dark mode, behind a thin route file; the waiting room renders at
  `/game/[roomCode]` while `status === "waiting"`.
- The shared primitives (`Button`, `Input`, `Card`, `Dialog`, `Toast`) exist
  in `src/components/common/` — token-themed, keyboard accessible, with
  visible focus states.
- Nickname (1–20 chars, no control characters) and room-code (8 chars,
  unambiguous alphabet) validation runs client-side before F3 actions.
- Lobby error codes map to friendly themed messages; buttons have disabled +
  loading states; no double submits.
- `npm run typecheck`, `npm run lint` and `npm run test` pass with zero
  errors/warnings.
- The manual test checklist in `03-manual-testing.md` has been walked through
  in the browser (desktop + mobile viewport) and noted in the PR description.
- No `player_token` ever appears in a URL, link or log.

## 5. Day-one quickstart

```bash
cd frontend
npm install                      # once — lockfile is committed
npm run typecheck                # baseline: must pass on the untouched tree
npm run lint                     # baseline: must pass on the untouched tree
npm run test                     # baseline: the vitest wire-contract suite passes
npm run dev                      # http://localhost:3001 (the Rust backend owns :3000)
```

The dev server runs fine without the backend (build against fixtures first);
for integration testing start the Rust backend on :3000 (see the repo README).

Then:

1. Read `RULES.md`, this file, then `01-screens-guide.md` →
   `02-routes-and-navigation.md` → `03-manual-testing.md` → `04-acceptance.md`.
2. Skim the real files your screens sit on: `src/app/layout.tsx`,
   `src/app/page.tsx`, `src/app/game/[roomCode]/page.tsx`, `global.css`,
   `src/types/protocol.ts`, `src/types/protocol.fixtures.ts`.
3. Open http://localhost:3001 and see the placeholder home screen — that is
   your starting point.
4. Build in this order: the primitives (`src/components/common/`) → the home
   screen → the waiting room, running typecheck + lint after each step.

## 6. Frozen F3 contract (quick reference)

The full handoff (returns, error surfaces, module paths) is
`04-acceptance.md` §2 — this is the cheat sheet. The names are **frozen**
(also pinned in F3's `src/services/03-stores-and-hooks.md` §5); do not rename
them without a joint PR:

```ts
useRoomActions()  // createRoom, joinRoom, submitMove, offerDraw, acceptDraw,
                  // resign, rematch, leaveRoom — each sets loading/error
useRoomState()    // { roomCode, players, status, yourColor, connection }
useGameState()    // { snapshot, pending }
useConnection()   // connection slice — banners/disabled states
```

Action signatures your screens call (each resolves with the **non-secret**
result — room code + your color; `player_token` never reaches a screen):

```ts
createRoom(nickname: string)                  // resolves { roomCode, yourColor }
joinRoom(roomCode: string, nickname: string)  // resolves { roomCode, yourColor }
leaveRoom()                                   // the waiting-room leave action
```
