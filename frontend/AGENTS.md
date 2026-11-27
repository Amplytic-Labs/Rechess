This is the **Rechess** Next.js web client. Prioritize responsive, accessible web patterns — the app must read well on a phone, a tablet and a desktop.

## Project bindings (read before anything else)

1. **[`RULES.md`](./RULES.md) is binding** for every contributor and AI agent: Tailwind classNames only, no hardcoded colors (tokens only), one WebSocket service (F3), server-authoritative state, no secrets in URLs.
2. **Work only inside your ownership map folder** (RULES.md §0). Each module has a README with its mission, contract, and test plan:
   - F1 → `src/screens/lobby/README.md`
   - F2 → `src/components/chess/README.md`
   - F3 → `src/services/README.md`
   - F4 → `src/screens/game/README.md`
3. Never add dependencies, edit configs (`next.config.ts`, `tsconfig.json`, `eslint.config.mjs`, `vitest.config.ts`, `package.json`, `global.css`), or disable lint rules without a team PR.
4. Never commit or log `player_token` or any secret.

## Stack (fixed)

Next.js 16 (App Router) · React 19 · TypeScript strict · Tailwind CSS v4 (CSS-first: tokens in `global.css`, no config file, no theme provider; dark mode follows the system) · zustand 5 (F3's stores) · vitest (pure-logic unit tests only) · **no component library** — shared UI primitives live in `src/components/common/` (F1 owns).

## Commands

```bash
cd frontend
npm run dev         # http://localhost:3001 (the Rust backend owns :3000)
npm run typecheck   # tsc --noEmit — zero errors
npm run lint        # eslint (enforces RULES.md)
npm run test        # vitest — pure-logic units (protocol, stores)
npm run build       # production build (CI runs this)
```

Run typecheck + lint + test before declaring any task done, plus your module's manual test checklist (RULES.md §5). CI (B4) runs the same gates.

## Navigation & routing

- App Router file-based routes in `src/app/` — route files stay **thin** wrappers rendering screen components from `src/screens/**`; real UI never lives in a route file.
- Import `Link` from `next/link`, `useRouter`/`useParams` from `next/navigation`. Never add `react-router` or similar.
- Game UI is client components (`"use client"`). Never fetch game data through Next APIs — the Rust WebSocket server is the only data source, via F3's service.
- No secrets in URLs — room code at most (`/game/[roomCode]`); `player_token` never (F3 owns it in storage).

## Web specifics

- Responsive by default: mobile portrait first, then tablet/desktop — no fixed page widths; grids and flex rows collapse to one column on narrow screens.
- Accessibility is not optional: keyboard-operable controls, visible focus states, `aria-*` where the visual alone does not carry the meaning. The board must be playable by keyboard (F2's docs specify how).
- Check the exact `next` version in `package.json` before using newer APIs — App Router behavior differs across major versions. When unsure, consult the versioned docs (https://nextjs.org/docs) rather than training data.
