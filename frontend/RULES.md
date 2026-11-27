# Frontend RULES — binding for every contributor (humans AND AI assistants)

> If you are an AI agent (or a human) about to write frontend code: read this
> file top to bottom first. The linter enforces most of it, but the reasons
> matter. If a rule blocks you, do not disable it — ask the team.

## 0. Ownership map — where you are allowed to write

| Path | Owner | You write here |
|---|---|---|
| `src/screens/lobby/`, `src/app/` lobby routes, `src/components/common/` | **F1** | lobby screens, shared UI primitives (Button, Input, Card…), navigation |
| `src/components/chess/` | **F2** | board, squares, pieces, highlights, promotion UI |
| `src/services/`, `src/store/`, `src/types/`, `src/security/`, `src/hooks/` | **F3** | WebSocket client, zustand stores, protocol types, token storage |
| `src/screens/game/`, `src/app/game/**` | **F4** | game screen composition, panels, history, results |
| `src/theme/`, `global.css` | shared (F1 curates) | only via PR |
| configs (`next.config.ts`, `tsconfig.json`, `eslint.config.mjs`, `vitest.config.ts`, `package.json`) | shared (B4/F3 curate) | only via PR |

General rule: **work inside your folder.** Cross-folder changes go through a
short PR the affected owner approves.

## 1. Styling — three absolute rules

### 1.1 Style with `className` (Tailwind CSS v4) only

No `style={}` objects with static values, no CSS files of your own, no CSS
modules. Tailwind utilities for layout, **theme tokens** (§1.2) for colors.

```tsx
// ❌ FORBIDDEN
<div style={{ backgroundColor: "#fff", borderRadius: 8 }} />

// ✅ REQUIRED
<div className="rounded-lg bg-surface" />
```

The `style` prop has exactly **one** legitimate use: a dynamic *value* that
cannot be expressed as a utility, computed from props/state (e.g. a piece's
translate offset during a drag) — keep it minimal and comment why.

### 1.2 No hardcoded colors — use the design tokens

No hardcoded colors anywhere — not in code, not in classNames. Use the
semantic tokens defined in **`global.css`** (frontend root — CSS-first: no
`tailwind.config.js`, no theme provider, no theme-switching JS):

| Token | Use |
|---|---|
| `background` / `surface` / `surface-muted` | screen / card / inset backgrounds |
| `foreground` / `muted-foreground` | primary / secondary text |
| `primary` / `primary-foreground` | brand actions |
| `border` | dividers, outlines |
| `destructive` / `success` | resign / confirm semantics |
| `board-light` / `board-dark` / `move-target` / `last-move` / `check` | chess board (F2) |

```tsx
// ❌ FORBIDDEN — hardcoded colors, no dark mode
<div className="bg-[#ffffff] dark:bg-[#111111]" />

// ✅ REQUIRED — tokens; dark values switch automatically
<div className="bg-surface" />
<p className="text-muted-foreground">hi</p>
```

No arbitrary color values (`bg-[#…]`, `text-[#…]`) — banned by ESLint
(`no-restricted-syntax`). Dark mode follows the system theme
(`prefers-color-scheme` in `global.css`) with zero per-component work.
Preview it with the OS appearance switch or the browser devtools emulation.

### 1.3 Shared UI primitives live in `src/components/common/`

The app has **no component library** — buttons, inputs, cards, dialogs and
toasts are small primitives **F1** builds once in `src/components/common/`,
themed with tokens. Import from there instead of hand-rolling per-screen
copies; propose additions via a short PR. Chess-specific UI belongs to F2
(`src/components/chess/`), never here.

## 2. Framework & components

- **Next.js 16 (App Router)** with React 19 and TypeScript `strict`.
- Game UI is **client components** (`"use client"`): the app is a real-time
  WebSocket client. Route files (`src/app/**/page.tsx`) stay **thin**: they
  render screen components from `src/screens/**` — real UI lives in
  screens so it stays reviewable and router-independent.
- **The Rust WebSocket server is the only data source.** Never add API
  routes, server actions or `fetch` calls that proxy or store game data.
  The app is served by `next start` (B4's deployment doc); all game traffic
  goes through F3's WebSocket service.
- Responsive by default: mobile portrait first, then tablet/desktop. No
  fixed page widths; multi-column layouts collapse to one column on narrow
  screens.
- Accessibility is not optional: keyboard-operable controls, visible focus
  states, `aria-*` where the visual alone does not carry the meaning.
- Animations: CSS transitions / Tailwind utilities. No animation or
  component library without a team PR.
- New dependency? It's a **team PR** — propose it, don't install it silently.

## 3. Navigation (F1/F4)

- **App Router file-based routes only** (`src/app/`). Route files stay thin
  (see §2); real UI lives in `src/screens/**`.
- Use `Link` from `next/link`, `useRouter`/`useParams` from
  `next/navigation`. Never add `react-router` or similar.
- **No secrets in URLs — ever.** Room code: yes (`/game/[roomCode]`).
  `player_token`: NEVER — it stays in F3's storage; F3's service resumes
  the session after navigation.

## 4. Data & state discipline (everyone; details in F3's README)

- **The server is authoritative.** Never compute legal moves, FEN changes,
  or game results client-side. Never optimistically apply your own move —
  show pending state and wait for `game.state`.
- Exactly **one** WebSocket connection per app (F3's service). Screens never
  open sockets — they call F3's hooks/actions.
- State lives in F3's zustand stores; screens subscribe with selectors.
  Components stay stateless where possible.
- Map every protocol error code (see `src/types/protocol.ts`) to a friendly,
  themed UI message. Never show raw JSON.
- Browser storage: `player_token` goes to F3's storage module only (see
  F3's security doc) — never `document.cookie`, never a URL, never a log.

## 5. Quality gates — run before every push

```bash
cd frontend
npm run typecheck   # tsc --noEmit — zero errors
npm run lint        # eslint — zero errors/warnings
npm run test        # vitest — pure-logic units (protocol, stores)
```

CI (B4's workflow) runs the same three. TypeScript `strict` is on; no `any`
in exported APIs.

### Testing strategy (deliberate)

- **Unit tests (vitest, node environment)**: pure logic only — the wire
  contract (`src/types/protocol.test.ts` is the pattern), F3's codec,
  stores, revision guard and reconnect/backoff logic as they land. F3 owns
  the suite; every store rule gets a test.
- **Wire contract**: `npm run typecheck` compiles the typed fixtures,
  vitest pins the envelope shapes, and `cargo test` on the backend
  round-trips the identical JSON (`backend/tests/protocol_contract.rs`).
- **UI**: manual testing in the browser (desktop + mobile viewport). Each
  module README lists its manual test checklist (steps + expected results)
  — run through it before declaring a task done, and note what you checked
  in the PR description.
- Fixtures in `src/types/protocol.fixtures.ts` double as realistic mock
  data for building UI before the backend is connected.

### Running locally

`npm run dev` serves at **http://localhost:3001** (the Rust backend owns
:3000 — see the repo README). The WebSocket URL comes from
`NEXT_PUBLIC_API_URL` (F3's config module reads it; default
`ws://localhost:3000/ws` for local dev, `wss://…` in production).

## 6. AI-assistant specifics

- Read **your module's README** before generating code; follow its contracts
  exactly (names, folders, prop signatures).
- Do not create files outside your ownership map (§0), do not edit
  `package.json`/configs to "fix" things, do not add dependencies.
- Do not disable or weaken ESLint rules or tsconfig strictness.
- If a rule conflicts with your task, or a contract needs a change: stop
  and surface it to the team instead of working around it.
- Never commit secrets, tokens, or `.env` files; never log `player_token`.
- Match the existing code style: double quotes, semicolons, functional
  components, `PropsWithChildren` for wrappers.
