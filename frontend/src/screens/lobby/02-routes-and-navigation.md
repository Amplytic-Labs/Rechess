# 02 — Routes and navigation

## 1. Route files (yours — keep them THIN)

RULES.md §3: route files in `src/app/*.tsx` stay thin — they import a screen
component from `src/screens/**` and render it. UI and state live in the
screen components, so they stay reviewable and router-independent.

| Route file | Renders | Status |
|---|---|---|
| `src/app/index.tsx` | `HomeScreen` from `@/screens/lobby/home-screen` | exists — **replace the placeholder body** |
| `src/app/create-room.tsx` | `CreateRoomScreen` from `@/screens/lobby/create-room-screen` | you create |
| `src/app/join-room.tsx` | `JoinRoomScreen` from `@/screens/lobby/join-room-screen` | you create |
| `src/app/waiting-room.tsx` | `WaitingRoomScreen` from `@/screens/lobby/waiting-room-screen` | you create |
| `src/app/game.tsx` (or F4's chosen name) | F4's game screen | **F4's file — do not create/edit** |

The thin-wrapper pattern (exact):

```tsx
// src/app/create-room.tsx
import { CreateRoomScreen } from "@/screens/lobby/create-room-screen";

export default function CreateRoomRoute() {
  return <CreateRoomScreen />;
}
```

`src/app/index.tsx` keeps its `export default function` shape — swap its
placeholder body for `<HomeScreen />` from `@/screens/lobby/home-screen`.
Import `router`, `Link`, `useLocalSearchParams` from `expo-router` only —
`react-navigation` imports are banned by ESLint.

`src/app/_layout.tsx` is done (GestureHandlerRootView → SafeAreaProvider →
HeroUINativeProvider → Stack). You normally need **no** layout change: unlisted
routes are rendered by the default `<Stack />`. If you want per-route header
titles, add `<Stack.Screen name="…" options={{…}} />` entries there — that
file is shared foundation, so make it a tiny coordinated PR, don't freestyle.
Screens may also draw their own headers; pick one and keep it consistent.

## 2. Navigation flow

```
                 ┌──────────────┐
                 │  /  (home)   │
                 └──────┬───────┘
           "Create room"│ "Join room"
          ┌─────────────┴──────────────┐
          ▼                            ▼
 ┌────────────────┐          ┌────────────────┐
 │ /create-room   │          │ /join-room     │
 │ nickname       │          │ code+nickname  │
 └───────┬────────┘          └───────┬────────┘
   createRoom() ok             joinRoom() ok
          └───────────┬────────────────┘
                      ▼
            ┌───────────────────┐   both seats filled
            │ /waiting-room     │ ──(room status "active",
            │ ?roomCode=XXXXXXXX│    server-authoritative)
            └─────────┬─────────┘ ────────────┐
                      │ back = leave room    ▼
                      ▼              ┌─────────────────┐
                back to /            │ /game (F4)     │
                                     │ ?roomCode=XXXX │
                                     └─────────────────┘
```

## 3. Push, params, and what may never be a param

```tsx
import { router, useLocalSearchParams } from "expo-router";

// navigate
router.push("/create-room");
router.push({ pathname: "/waiting-room", params: { roomCode: "TESTCODE1" } });

// read typed params (in the screen component)
const { roomCode } = useLocalSearchParams<{ roomCode: string }>();
```

- **Room code in params: OK.** It is a public identifier, not a credential.
- **`player_token` in params: NEVER** (RULES.md §3, brief §2). It never
  appears in route params, deep links, or logs; F3 keeps it in
  `expo-secure-store` and handles resume. Your screens never even hold it.
- Prefer `router.replace` **into** the game route (waiting → game is a
  forward transition — you don't want "back" from the game returning to a
  stale waiting room). Use `router.push` for home → create/join.
- Don't pass anything else — nickname lives in the store/F3's action call,
  not the URL.

## 4. Back handling (your choices to implement)

| From | Back button (header/Android hardware) does | Rationale |
|---|---|---|
| `/create-room`, `/join-room` | plain back to `/` (default `router.back()`) | no state to protect; just dismiss the keyboard/form |
| `/waiting-room` | confirm-then-leave: alert ("Leave the room? The other player will be notified.") → on confirm call F3's `leaveRoom()` then `router.dismissTo("/")`/`router.back()` | leaving is a real action — the seat frees up server-side |
| into `/game` | `router.replace`, so game back-navigation is F4's problem to define | keep one nav owner per screen |

Keep this table in the screen components as a short comment so reviewers see
the intent. If you need a blocker (disable hardware back), use Expo Router's
documented back-handler APIs — check the current SDK 58 docs
(https://docs.expo.dev/versions/v58.0.0/) before writing anything.

## 5. Keeping routes thin (checklist)

- Zero styling in route files — not even `className`.
- Zero state/logic in route files; params are read **inside the screen
  component** with `useLocalSearchParams`, not forwarded as props.
- One default export per route file, importing exactly one screen component.
