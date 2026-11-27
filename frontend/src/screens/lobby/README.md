# `src/screens/lobby/` — Owner: **F1** (Expo App Foundation and Lobby)

> **Read `frontend/RULES.md` first** — it is binding (no StyleSheet, no
> expo-status-bar, everything themed, ownership map). Then read this file
> completely before writing code.

---

## 1. Your mission

Deliver the app shell and everything that happens **before** the game:
navigation, home screen, create-room flow, join-room flow and the waiting
room. You own the reusable UI building blocks (`src/components/common/`)
the rest of the team composes their screens from.

You are **not** implementing the chessboard (F2), the WebSocket client or
stores (F3), or the game screen (F4). You call F3's hooks; you never open a
socket.

## 2. What you own / what you don't touch

| You own | You do NOT touch |
|---|---|
| `src/app/` route files for lobby routes (thin wrappers) | `src/components/chess/` (F2) |
| `src/screens/lobby/**` (the real screens) | `src/services/`, `src/store/`, `src/types/`, `src/security/` (F3) |
| `src/components/common/**` (buttons, inputs, cards…) | `src/screens/game/` (F4) |
| shared screen layout patterns | `src/theme/` (changes via PR) |

## 3. The base that already exists (don't rebuild it)

- Expo SDK 58 + TypeScript strict + Expo Router (`src/app/_layout.tsx` with
  `GestureHandlerRootView`, `SafeAreaProvider`, `AppThemeProvider`, `Stack`).
- NativeWind 4.2.7 + design tokens (`tailwind.config.js`) — `bg-surface`,
  `text-foreground`, `bg-primary`, … light/dark handled by
  `src/theme/ThemeProvider.tsx`.
- A placeholder home screen at `src/app/index.tsx` — **replace it** with your
  lobby home.
- `expo-clipboard` (copy room code), `expo-splash-screen` — already installed.

## 4. Route plan (yours to implement)

```
src/app/
├── _layout.tsx        # root layout (done — extend only if screens need global options)
├── index.tsx          # Home           → src/screens/lobby/home-screen.tsx
├── create-room.tsx    # Create Room    → src/screens/lobby/create-room-screen.tsx
├── join-room.tsx      # Join Room      → src/screens/lobby/join-room-screen.tsx
└── waiting-room.tsx   # Waiting Room   → src/screens/lobby/waiting-room-screen.tsx
```

Rules for routes (see RULES.md §3): route files stay **thin** — they import a
screen component and render it. UI + state live in the screen component so
it can be unit-tested without the router. Agree the game-screen route with
F4 (`src/app/game.tsx` is their file; your waiting-room navigates there once
both players are ready — coordinate the exact navigation contract with F3's
`useRoomState`).

### Suggested screen-component shape

```tsx
// src/screens/lobby/home-screen.tsx
import { router } from "expo-router";
import { AppButton } from "@/components/common/app-button";
import { SafeAreaView } from "react-native-safe-area-context";

export function HomeScreen() {
  return (
    <SafeAreaView className="flex-1">
      {/* themed, token-only styling — RULES.md §1.3 */}
    </SafeAreaView>
  );
}
```

`src/components/common/` — build at least: `app-button` (primary/secondary/
destructive variants, disabled + loading states), `app-input` (label, error
text), `room-code-display` (big code + copy button using `expo-clipboard`).
These are what F4 builds on — get their props right and document them in
doc comments.

## 5. Screen behaviour requirements (your brief)

- **Home:** Create Room + Join Room entry points, clear branding.
- **Create room:** nickname input → validation → loading state → F3's
  `createRoom(nickname)` → navigate to waiting room on success; show F3's
  error states on failure.
- **Join room:** nickname + room code inputs → normalize the code
  (uppercase, trim, strip spaces) → `joinRoom(code, nickname)` → waiting
  room; render server errors (`ROOM_NOT_FOUND`, `ROOM_FULL`, …) as friendly
  themed messages.
- **Waiting room:** room code (copy/share affordance), your nickname +
  color (White for host), opponent placeholder until they join, connection
  state. Navigate to the game route when both players are present (via F3's
  room state — never a local timer).
- Nickname: required, 1–20 chars, no control characters (validate before
  calling F3; the server re-validates).
- Buttons: disabled + loading states, no double-submits.
- Accessibility: readable contrast (tokens give you this), labels, adequate
  touch targets (≥ 44pt).

## 6. Your contract with F3 (agree names on Day 1)

You consume exactly these (proposed names — freeze with F3):

```ts
const { createRoom, joinRoom } = useRoomActions();        // loading/error included
const { roomCode, players, status, connection } = useRoomState();
// createRoom(nickname) → resolves { roomCode, yourColor }
// joinRoom(roomCode, nickname) → resolves { roomCode, yourColor }
```

Never receive or touch `player_token` — F3 stores it securely and handles
resume. Never put it in route params, deep links, or logs.

## 7. How to test your module independently

```bash
cd frontend
npm run typecheck && npm run lint
npm test            # jest (jest-expo preset) + @testing-library/react-native
```

- Unit-test validation helpers (nickname, room-code normalization) as pure
  functions — e.g. `src/screens/lobby/room-code.test.ts`.
- Component-test screens with F3's hooks **mocked** (jest mock of
  `useRoomActions`), covering: loading, error, success, disabled states.
- `npx expo start` and exercise the flows with a stubbed/mock F3 service
  until F3's real service lands (Day 3–4 vertical slice).

## 8. Deliverables (definition of done)

- [ ] Routes + screens: home, create-room, join-room, waiting-room (thin route files)
- [ ] `src/components/common/` reusable, themed, documented props
- [ ] Room-code copy/share affordance (`expo-clipboard`)
- [ ] Validation + loading/error/disabled states everywhere
- [ ] Mocked-hook component tests + validation unit tests
- [ ] This folder's README updated with your final component props (§4)

## 9. Key coordination

- **F3** daily: actions/hooks names, token-safe navigation.
- **F4**: the route + navigation contract into the game screen.
- Team: agree one visual language early (use the tokens; don't invent colors).
