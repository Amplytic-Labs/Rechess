# Rechess — Frontend (Next.js web client)

Next.js 16 (App Router) · React 19 · TypeScript strict · Tailwind CSS v4 ·
zustand 5 · vitest — a real-time online chess client for the browser, talking
to the Rust backend over WebSocket protocol v1.

> **Read [`RULES.md`](./RULES.md) before writing any code — it is binding
> for humans and AI assistants.** Then read your module's README (§ module
> map below).

## Run it

```bash
cd frontend
npm install
npm run dev        # http://localhost:3001 — the Rust backend owns :3000
```

Start the backend first (see the repo README). The WebSocket URL comes from
`NEXT_PUBLIC_API_URL` (F3's config module reads it; default
`ws://localhost:3000/ws` for local dev, `wss://…` in production — B4's
deployment doc).

## Quality gates — run before every push

```bash
npm run typecheck  # tsc --noEmit
npm run lint       # eslint (flat config; enforces RULES.md)
npm run test       # vitest — wire-contract and store unit tests
npm run build      # production build (CI runs this)
```

Plus your module's **manual test checklist** (RULES.md §5 — UI is verified
manually in the browser; each module README lists its steps). CI (B4's
workflow) runs the same gates.

## Module map & ownership

| Folder | Owner | Purpose | Read first |
|---|---|---|---|
| `src/screens/lobby/` + lobby routes | **F1** | home, create/join room, waiting room | `src/screens/lobby/README.md` |
| `src/components/chess/` | **F2** | board, pieces, highlights, promotion | `src/components/chess/README.md` |
| `src/services/` `src/store/` `src/types/` `src/security/` `src/hooks/` | **F3** | WebSocket client, zustand stores, protocol types, token storage | `src/services/README.md` |
| `src/screens/game/` + `src/app/game/**` | **F4** | game screen, panels, history, results | `src/screens/game/README.md` |
| `src/theme/` + `global.css` | shared (F1 curates) | design tokens | `src/theme/README.md` |
| `src/components/common/` | shared (F1 curates) | shared UI primitives (Button, Input, …) | `src/components/common/` |

Wire protocol: `docs/protocol-v1.md` at the repo root — already materialized
as TypeScript types in `src/types/protocol.ts` (F3 owns keeping it in sync
with the backend), with typed mock data in `src/types/protocol.fixtures.ts`
and unit tests in `src/types/protocol.test.ts`.

## What is preconfigured (don't redo it)

- Next.js 16 App Router skeleton: root layout (`src/app/layout.tsx`) with
  tokens imported from `global.css`, placeholder lobby route
  (`src/app/page.tsx`) and placeholder game route
  (`src/app/game/[roomCode]/page.tsx`). F1/F4 replace the placeholders with
  thin wrappers over `src/screens/**`.
- **Tailwind CSS v4** via `@tailwindcss/turbopack` (wired in
  `next.config.ts` — no PostCSS config, no `tailwind.config.js`): all
  semantic tokens (incl. board colors) live in `global.css`; dark mode
  follows the system automatically (`prefers-color-scheme`).
- ESLint flat config (`eslint.config.mjs`) enforcing RULES.md: no arbitrary
  color literals (`bg-[#…]` banned).
- vitest (`vitest.config.ts`, node environment) with the wire-contract
  suite in `src/types/protocol.test.ts`.
- Versions are pinned in `package.json` + `package-lock.json`; adding a
  dependency is a team decision (RULES.md §2).
