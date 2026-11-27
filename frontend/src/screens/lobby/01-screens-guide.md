# 01 — Screens guide (HOW to build each lobby screen)

Conventions for all four screens in `src/screens/lobby/`:

- **Screen components** live here (`home-screen.tsx`, `create-room-screen.tsx`,
  `join-room-screen.tsx`, `waiting-room-screen.tsx`); the route files in
  `src/app/` just render them (see `02-routes-and-navigation.md`).
- **Styling**: uniwind `className` ONLY — no `StyleSheet`, no inline styles,
  no hardcoded colors (`bg-[#…]` is banned). Colors come from the semantic
  tokens in `frontend/global.css`:

  | Token (utility) | Use on a lobby screen |
  |---|---|
  | `bg-background` | screen background |
  | `bg-surface` / `bg-surface-muted` | cards, inset panels, the room-code plate |
  | `text-foreground` / `text-muted-foreground` | primary / secondary text |
  | `bg-primary` + `text-primary-foreground` | primary action (Create / Join button) |
  | `border-border` | dividers, outlines |
  | `text-destructive` / `bg-destructive` | validation + server error messages |
  | `text-success` | connected / ready state |

  Dark mode is automatic (system theme) — never write `dark:` overrides for
  these, the token values switch themselves.
- **Safe areas**: wrap every screen in `StyledSafeAreaView` from
  `@/components/common/styled` (never import `SafeAreaView` directly).
- **HeroUI Native 1.1**, granular imports only — `import { Button } from "heroui-native/button"`; the root `heroui-native` import is banned by
  ESLint. Verified props in the installed package
  (`node_modules/heroui-native/src/components/*/​*.types.ts`):
  - `Button`: `variant?: "primary" | "secondary" | "tertiary" | "outline" | "ghost" | "danger" | "danger-soft"`, `size?: ButtonSize`, `isDisabled?: boolean`, `isIconOnly?: boolean`, `children`.
  - `Input`: `variant?: "primary" | "secondary"`, `isInvalid?: boolean`, `isDisabled?: boolean`, plus the usual `TextInput` props (`value`, `onChangeText`, `placeholder`, … — confirm against `input.types.ts`).
  - Also available granularly: `heroui-native/card`, `heroui-native/spinner`, `heroui-native/toast`, `heroui-native/dialog`, `heroui-native/label`, `heroui-native/separator`, …
- **Fixtures**: while F3/backend are landing, import from
  `@/types/protocol.fixtures` (`sampleRoom`, `sampleRoomCreatedEvent`, …) to
  render screens with realistic data; swap for F3's hooks when they exist.
- Accessibility: labels on inputs, ≥ 44pt touch targets, token contrast.

---

## 1. Home screen — `src/screens/lobby/home-screen.tsx`

The app's entry point. Requirements: clear branding ("Rechess"), exactly two
actions — **Create room** (primary) and **Join room** — and **no** sign-up or
login entry anywhere (brief §6).

Skeleton (partial — the real work is yours):

```tsx
// src/screens/lobby/home-screen.tsx
import { Text, View } from "react-native";

import { Button } from "heroui-native/button";

import { router } from "expo-router";

import { StyledSafeAreaView } from "@/components/common/styled";

export function HomeScreen() {
  return (
    <StyledSafeAreaView className="flex-1">
      <View className="flex-1 items-center justify-center gap-4 px-6">
        <Text className="text-4xl font-bold text-foreground">Rechess</Text>
        <Text className="text-center text-base text-muted-foreground">
          Play chess with a friend — no account needed.
        </Text>
        {/* TODO(F1): primary action — Create room */}
        <Button variant="primary" className="min-w-[220px]" onPress={() => router.push("/create-room")}>
          Create room
        </Button>
        {/* TODO(F1): secondary action — Join room */}
        <Button variant="secondary" className="min-w-[220px]" onPress={() => router.push("/join-room")}>
          Join room
        </Button>
      </View>
    </StyledSafeAreaView>
  );
}
```

Layout notes: center the block vertically, `px-6` gutters, `gap-4` rhythm;
nothing else on this screen. Keep it this small.

## 2. Create room — `src/screens/lobby/create-room-screen.tsx`

Form: one nickname input → submit → waiting room.

**Validation (mirror `docs/protocol-v1.md` §3 so the server never has to
reject it):** nickname required, **1–20 characters**, **no control
characters** (`/[\u0000-\u001F\u007F]/`). Disable Submit while invalid or
while the request is in flight (loading state) — no double submits.

**What happens on submit:** your screen calls F3's action — you never build
an envelope or open a socket. The exact wire command F3 sends for you is
(from `src/types/protocol.ts`):

```ts
| { type: "room.create"; payload: { nickname: string } }
// wrapped by F3 into the envelope: { v: 1, id: "<uuid>", type: "room.create", payload: { nickname } }
```

The private reply (`RoomCreatedPayload` — the ONLY wire event carrying
`player_token`):

```ts
export interface RoomCreatedPayload {
  room_code: string;
  player_id: string;
  player_token: string; // F3 stores this securely — your screen NEVER sees/stores/logs it
  color: PlayerColor;   // "white" for the host
  room: RoomPublicState;
}
```

Your screen only consumes the **non-secret** result — room code + your color —
as returned by F3's `createRoom` action / `useRoomState()`. On success:
`router.push` to the waiting room (room code may travel as a route param;
`player_token` NEVER).

Skeleton:

```tsx
// src/screens/lobby/create-room-screen.tsx
import { useState } from "react";
import { Text, View } from "react-native";

import { Button } from "heroui-native/button";
import { Input } from "heroui-native/input";

import { StyledSafeAreaView } from "@/components/common/styled";

// TODO(F1): when F3 publishes hooks — e.g. import { useRoomActions } from "@/hooks/<F3's module>";
// TODO(F1): mock phase — drive the UI with sampleRoomCreatedEvent from "@/types/protocol.fixtures"

export function CreateRoomScreen() {
  const [nickname, setNickname] = useState("");
  // TODO(F1): trim + validate (1–20 chars, no control chars)
  // TODO(F1): submitting/loading state; call F3's createRoom(nickname)
  // TODO(F1): on success → router.push({ pathname: "/waiting-room", params: { roomCode } })
  // TODO(F1): on failure → friendly themed error (see the error table in §3 below)
  return (
    <StyledSafeAreaView className="flex-1">
      <View className="flex-1 justify-center gap-4 px-6">
        <Text className="text-2xl font-bold text-foreground">Create room</Text>
        {/* TODO(F1): <Input value={nickname} onChangeText={setNickname} ... /> + hint text */}
        {/* TODO(F1): <Button isDisabled={…}>Create</Button> with loading state */}
      </View>
    </StyledSafeAreaView>
  );
}
```

## 3. Join room — `src/screens/lobby/join-room-screen.tsx`

Form: room code + nickname → submit → waiting room.

**Room-code normalization (client-side, before calling F3):** trim outer
whitespace, uppercase, and reject anything that isn't exactly **8 characters
from the unambiguous alphabet `ABCDEFGHJKMNPQRSTUVWXYZ23456789`** after
normalization (no `I`, `L`, `O`, `0`, `1`). Per `docs/protocol-v1.md` §3 the
server uppercases case-insensitive input too, but a clean client normalization
(`code.trim().toUpperCase()`) gives instant feedback and avoids a round trip.
Wire command F3 sends for you:

```ts
| { type: "room.join"; payload: { room_code: string; nickname: string } }
```

The reply (`RoomJoinedPayload`) has the same rule: `player_token` belongs to
F3 only; you consume `color` ("black" for the guest) + room state.

**Error mapping table** — map every lobby-relevant `ErrorCode`
(`src/types/protocol.ts`) to a friendly themed message; never show raw JSON
(RULES.md §4). Build this as a small lookup (a `Record<ErrorCode, string>` or
a switch over the error) inside `src/screens/lobby/`:

| `ErrorCode` | Friendly message (suggested wording) | UI treatment |
|---|---|---|
| `ROOM_NOT_FOUND` | "No room with that code. Check the code and try again." | inline error on the code input |
| `ROOM_FULL` | "That room already has two players." | inline error on the code input |
| `ROOM_EXPIRED` | "That room has expired. Create a new one." | inline error + Create-room suggestion |
| `INVALID_PAYLOAD` | "Please check your nickname and room code." | inline validation (should be prevented client-side first) |
| `RATE_LIMITED` | "Too many attempts. Please wait a moment." | inline error, disable submit briefly |
| `UNSUPPORTED_VERSION` | "The app needs an update to connect." | themed banner |
| `INTERNAL` | "Something went wrong. Please try again." | themed banner + retry |

(`INVALID_TOKEN`, `NOT_YOUR_TURN`, `ILLEGAL_MOVE`, `STALE_REVISION`,
`GAME_FINISHED` belong to F3/F4 flows, not your forms.)

Skeleton: same shape as create-room — two `Input`s (code first with
`autoCapitalize="characters"` + `maxLength={8}`, then nickname) + Submit
button with disabled/loading states, then the error row rendered with
`text-destructive`. Reuse the nickname validation from §2.

## 4. Waiting room — `src/screens/lobby/waiting-room-screen.tsx`

Shown to host and guest after create/join succeeds.

Must show: the **room code prominently** (big `text-3xl font-bold`-ish plate
on `bg-surface-muted` with `border-border`), a **copy button** using
`expo-clipboard` (confirmed in `package.json`: `expo-clipboard ~58.0.4`):

```tsx
import * as Clipboard from "expo-clipboard";
// on press:
await Clipboard.setStringAsync(roomCode);
// then a small "Copied" confirmation (text-success or a toast from heroui-native/toast)
```

(An optional share affordance can use React Native's built-in
`Share.share({ message: roomCode })`.)

Plus: your nickname + assigned color (White for the host, Black for the
guest), the opponent's seat as a placeholder ("Waiting for opponent…")
until they join, and a connection state indicator driven by F3's
`useConnection()` (never a local timer). Read the room code from route
params (`useLocalSearchParams`) and the live room state from F3's
`useRoomState()` — fixture `sampleRoom` shows the one-player shape:

```ts
export const sampleRoom: RoomPublicState = {
  room_code: "TESTCODE1",
  status: "waiting",
  players: [{ id: "host", nickname: "Host", color: "white", connected: true }],
};
```

**Transition to the game:** when both seats are filled — i.e. room `status`
flips from `"waiting"` to `"active"` via F3's store — navigate to F4's game
route with the room code (`router.replace` — see
`02-routes-and-navigation.md`). Never navigate on a timer or on local
optimism; the server is authoritative.

Skeleton:

```tsx
// src/screens/lobby/waiting-room-screen.tsx
import { useState } from "react";
import { Text, View } from "react-native";

import { Button } from "heroui-native/button";

import { useLocalSearchParams } from "expo-router";

import { StyledSafeAreaView } from "@/components/common/styled";
// TODO(F1): F3's useRoomState()/useConnection(); mock with sampleRoom / sampleRoomCreatedEvent first

export function WaitingRoomScreen() {
  const { roomCode } = useLocalSearchParams<{ roomCode: string }>();
  const [copied, setCopied] = useState(false);
  // TODO(F1): copy via Clipboard.setStringAsync(roomCode)
  // TODO(F1): render players[0]/players[1] seats + "Waiting for opponent…" placeholder
  // TODO(F1): connection indicator from useConnection()
  // TODO(F1): when status === "active" → router.replace({ pathname: "/game", params: { roomCode } }) (confirm route with F4)
  return (
    <StyledSafeAreaView className="flex-1">
      <View className="flex-1 items-center gap-6 px-6">
        {/* TODO(F1): room code plate */}
        <Text className="text-3xl font-bold tracking-widest text-foreground">{roomCode}</Text>
        {/* TODO(F1): copy button + copied feedback */}
      </View>
    </StyledSafeAreaView>
  );
}
```

## 5. Building before F3 exists (fixture/mock mode)

Until F3's hooks land (Day 3–4), render each screen against the fixtures in
`src/types/protocol.fixtures.ts`: drive the waiting room with `sampleRoom`
(1 player) and a hand-written two-player `RoomPublicState`, and simulate
`room.created` with `sampleRoomCreatedEvent` (its `room` field is the same
public state). Keep the fixture usage behind a small local seam (a
`const MOCK = …` toggle or a prop) so swapping in F3's real hooks is a
one-line change, not a rewrite.
