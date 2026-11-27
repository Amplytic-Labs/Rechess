This is the **Rechess** Expo/React Native mobile app. Prioritize mobile-first patterns, performance, and cross-platform compatibility.

## Project bindings (read before anything else)

1. **[`RULES.md`](./RULES.md) is binding** for every contributor and AI agent: no `StyleSheet`, no `expo-status-bar`, everything themed with tokens, expo-router only, one WebSocket service (F3), server-authoritative state.
2. **Work only inside your ownership map folder** (RULES.md §0). Each module has a README with its mission, contract, and test plan:
   - F1 → `src/screens/lobby/README.md`
   - F2 → `src/components/chess/README.md`
   - F3 → `src/services/README.md`
   - F4 → `src/screens/game/README.md`
3. Never add dependencies, edit configs (`app.json`, `metro.config.js`, `package.json`, `global.css`), or disable lint rules without a team PR.
4. Never commit or log `player_token` or any secret.

## Expo has changed — do not trust your training data

Expo ships breaking changes every SDK release. APIs you remember are likely renamed, moved, or removed. Before writing any code that touches an Expo, EAS, or React Native API:

1. Read the major version of the `expo` package in `package.json` (this project: SDK 58, React Native 0.88, React 19.3, New Architecture only, edge-to-edge enforced).
2. Fetch the matching versioned docs: `https://docs.expo.dev/versions/v58.0.0/`
3. For anything else, fetch https://docs.expo.dev/llms.txt — an index of all Expo docs with corrections to common LLM misconceptions. Follow its links to the specific page you need; never answer from memory.

## Styling

- **uniwind v1.14** (Tailwind CSS v4) via `className` only — see RULES.md §1. Semantic tokens live in `global.css` (repo root, CSS-first — no tailwind.config.js, no ThemeProvider); dark mode follows the system theme automatically. Third-party components get `className` via `withUniwind` in `src/components/common/styled.ts`.
- **HeroUI Native v1.1** components (Button, Card, Input, …) — always granular imports like `heroui-native/button`, never the root import (banned by lint; breaks bundle-size optimization).
- Animations with Reanimated 4.7; never the legacy `Animated` API.

## Commands

```bash
npx expo install <package>  # ALWAYS use instead of npm add — resolves SDK-compatible versions (and needs a team PR)
npx expo start              # start the dev server
npm run lint                # eslint (enforces RULES.md)
npm run typecheck           # tsc --noEmit
npx expo-doctor             # diagnose dependency and config issues
npx expo install --fix       # fix incompatible package versions
```

Run lint and typecheck before declaring any task done, plus your module's
manual test checklist (RULES.md §5 — the frontend has no unit-test runner
by design). CI (B4) runs the same.

## Navigation & Routing

- Use **Expo Router** for all navigation. Routes live in `src/app/` — every file there is a screen, `_layout.tsx` files define navigators. Keep non-route code (components, hooks, utils, screens) outside `src/app/`: route files are thin wrappers over `src/screens/**`.
- Import `Link`, `router`, and `useLocalSearchParams` from `expo-router`. Never import react-navigation packages (banned by lint).
- No secrets in route params — room code at most; `player_token` never (F3 owns it in secure storage).
- Docs: https://docs.expo.dev/router/introduction.md

## Styling

- NativeWind v4 (Tailwind 3 utilities) via `className` only — see RULES.md §1. Semantic tokens from `tailwind.config.js`; no arbitrary color values; dark mode works automatically through `src/theme/ThemeProvider.tsx`.
- Animations with Reanimated 4.7 (babel preset already configured); never the legacy `Animated` API.

## Rules

- If `ios/` and `android/` directories do not exist, they are generated (Continuous Native Generation). Never create or edit them by hand — configure native behavior in `app.json` and config plugins.
- Expo Go only includes its bundled native modules. After adding a library with native code, the app needs a development build: `npx expo run:ios|android` locally, or `eas build --profile development`.
- Prefer recommended Expo modules over third-party libraries, and check your available skills before adding dependencies. Docs: https://docs.expo.dev/versions/latest/index.md
