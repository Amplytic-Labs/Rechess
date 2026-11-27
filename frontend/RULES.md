# Frontend RULES — binding for every contributor (humans AND AI assistants)

> If you are an AI agent (or a human) about to write frontend code: read this
> file top to bottom first. The linter enforces most of it, but the reasons
> matter. If a rule blocks you, do not disable it — ask the team.

## 0. Ownership map — where you are allowed to write

| Path | Owner | You write here |
|---|---|---|
| `src/screens/lobby/`, `src/app/` routes for lobby, `src/components/common/` | **F1** | lobby screens, buttons/inputs, navigation |
| `src/components/chess/` | **F2** | board, squares, pieces, highlights, promotion UI |
| `src/services/`, `src/store/`, `src/types/`, `src/security/` | **F3** | WebSocket client, zustand stores, protocol types, token storage |
| `src/screens/game/` | **F4** | game screen composition, panels, history, results |
| `src/theme/`, `tailwind.config.js` | shared (F1 curates) | only via PR |
| configs (`app.json`, `babel/metro/tailwind`, `package.json`) | shared (B4/F3 curate) | only via PR |

General rule: **work inside your folder.** Cross-folder changes go through a
short PR the affected owner approves.

## 1. Three absolute rules

### 1.1 No `StyleSheet` objects. Ever.

Style with **NativeWind `className`** only. Tailwind utilities for layout,
and **theme tokens** (§1.3) for colors.

```tsx
// ❌ FORBIDDEN
const styles = StyleSheet.create({ card: { backgroundColor: "#fff", borderRadius: 8 } });
<View style={styles.card} />;

// ✅ REQUIRED
<View className="rounded-lg bg-surface" />
```

The `style` prop has exactly **two** legitimate uses:
1. `style={vars({ "--color-x": "…" })}` for dynamic theme variables,
2. a dynamic *value* that cannot be expressed as a utility, computed from
   props/state (e.g. a piece's translate offset) — keep it minimal and
   comment why.

Anything else → className. Banned by ESLint (`no-restricted-imports` +
`no-restricted-syntax` + `react-native/no-inline-styles`).

### 1.2 Do not use `expo-status-bar`

Never import it, never mount `<StatusBar>` in any screen or component. The
status bar is configured **once, app-wide, in `app.json`** (light/dark
follows `userInterfaceStyle: "automatic"`, and SDK 58 is edge-to-edge).
Two components mounting StatusBar in different screens is a classic source
of flicker and inconsistency — one config, zero mounts.
Enforced by ESLint (`no-restricted-imports` on `expo-status-bar`).

### 1.3 Every component you touch must be THEMED

No hardcoded colors anywhere — not in code, not in classNames. Use the
semantic tokens from `tailwind.config.js` (values defined in
`src/theme/ThemeProvider.tsx` + `tailwind.config.js`):

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
<View className="bg-[#ffffff] dark:bg-[#111111]" />
<Text style={{ color: "#333" }}>hi</Text>

// ✅ REQUIRED — tokens; dark values come from ThemeProvider automatically
<View className="bg-surface" />
<Text className="text-muted-foreground">hi</Text>
```

No arbitrary color values (`bg-[#…]`, `text-[#…]`), no `react-native` color
APIs with literals (`no-color-literals` rule). Dark mode must work with zero
extra work — tokens switch via `src/theme/ThemeProvider.tsx`. Preview dark
mode with the device simulator's appearance switch.

## 2. Styling & components

- **NativeWind v4.2.7** (Tailwind 3 utilities) is the styling system. RN core
  components accept `className` directly. **Third-party** components need a
  one-time `cssInterop` mapping at the app root — see the pattern in
  `src/app/_layout.tsx` (`GestureHandlerRootView`). Don't scatter cssInterop
  calls into components; put them in `_layout.tsx` or the component's own
  module.
- Animations: **Reanimated 4.7** (no manual babel config — handled by
  `babel-preset-expo`). Never use the `Animated` API from react-native.
- Gestures: `react-native-gesture-handler` (~3.3); everything is inside
  `GestureHandlerRootView` already.
- Safe areas: `react-native-safe-area-context` (`SafeAreaView` /
  `useSafeAreaInsets`), never padding hacks.
- Lists: `FlatList` accepts `className` and `contentContainerClassName`.
- New dependency? It's a **team PR** — propose it, don't install it silently.

## 3. Navigation (F1/F4)

- **Expo Router only** (file-based routes in `src/app/`). Never import
  `react-navigation` directly (banned by ESLint).
- Route files (`src/app/*.tsx`) stay **thin**: they render screen components
  from `src/screens/**`. Real UI lives in screens, so it can be tested
  without the router (see F1's README).
- **No secrets in route params or deep links.** Room code: yes.
  `player_token`: NEVER — it stays in secure storage (F3).

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

## 5. Code quality gates — run before every push

```bash
cd frontend
npm run typecheck   # tsc --noEmit — zero errors
npm run lint        # eslint — zero errors/warnings
npm test            # jest (jest-expo preset) — your module's tests pass
```

CI (B4's workflow) runs the same three. TypeScript `strict` is on; no `any`
in exported APIs. Component tests use `@testing-library/react-native`
(behavior + props, not styling).

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
