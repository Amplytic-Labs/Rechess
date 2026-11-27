# Rechess — Frontend (Expo / React Native mobile app)

Expo SDK 58 · React Native 0.88 · React 19.3 · TypeScript (strict) ·
Expo Router · NativeWind 4.2.7 (Tailwind 3) · Reanimated 4.7 · zustand 5

> **Read [`RULES.md`](./RULES.md) before writing any code — it is binding
> for humans and AI assistants.** Then read your module's README (§ module
> map below).

## Run it

```bash
cd frontend
npm install        # once — deps are already committed in package-lock.json
npm start          # Expo dev server → scan the QR with Expo Go on your phone
```

- Simulator: `npm run ios` (needs macOS) / `npm run android`.
- Quick UI check without a device: `npm run web` (dev convenience only —
  the product is the mobile app).
- Point the app at the backend: F3's `src/services/README.md` documents the
  connection URL (LAN IP for dev, WSS in deployment).

## Quality gates — run all three before every push

```bash
npm run typecheck  # tsc --noEmit
npm run lint       # eslint (flat config; enforces RULES.md)
npm test           # jest (jest-expo preset)
```

CI runs the same three (B4's workflow). Main stays green.

## Module map & ownership

| Folder | Owner | Purpose | Read first |
|---|---|---|---|
| `src/screens/lobby/` + lobby routes | **F1** | home, create/join room, waiting room | `src/screens/lobby/README.md` |
| `src/components/chess/` | **F2** | board, pieces, highlights, promotion | `src/components/chess/README.md` |
| `src/services/` `src/store/` `src/types/` `src/security/` `src/hooks/` | **F3** | WebSocket client, zustand stores, protocol types, token storage | `src/services/README.md` |
| `src/screens/game/` + game route | **F4** | game screen, panels, history, results | `src/screens/game/README.md` |
| `src/theme/` + `tailwind.config.js` | shared (F1 curates) | design tokens | `src/theme/README.md` |

Wire protocol: `docs/protocol-v1.md` at the repo root (co-owned by B3 + F3).

## What is preconfigured (don't redo it)

- Expo Router with `src/app/_layout.tsx`: `GestureHandlerRootView`,
  `SafeAreaProvider`, `AppThemeProvider`, `Stack`, splash handling.
- NativeWind: `tailwind.config.js` (semantic tokens incl. board colors),
  `babel.config.js`, `metro.config.js`, `src/global.css`, `nativewind-env.d.ts`.
- Themed light/dark via `src/theme/ThemeProvider.tsx` (vars pattern) — dark
  mode works with **zero** per-component work when you use tokens.
- ESLint flat config (`eslint.config.js`) enforcing RULES.md: no StyleSheet,
  no expo-status-bar, no react-navigation, no inline styles/color literals.
- jest (jest-expo) + @testing-library/react-native; prettier with
  `prettier-plugin-tailwindcss` (className sorting) — run `npx prettier -w .`.

## Stack notes

- Status bar: configured app-wide in `app.json`
  (`userInterfaceStyle: "automatic"`); mounting `expo-status-bar` in code is
  **banned** (RULES.md §1.2).
- New Architecture / edge-to-edge are enforced by SDK 58 — do not fight them.
- Versions are pinned in `package.json` + `package-lock.json`; adding a
  dependency is a team decision (RULES.md §2).
