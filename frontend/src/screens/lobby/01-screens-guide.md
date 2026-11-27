# 01 — Screens guide (HOW to build the lobby UI and the shared primitives)

Conventions for every component you build in `src/screens/lobby/` and
`src/components/common/`:

- **Client components**: every file that uses hooks, state or event handlers
  starts with `"use client";` — the app is a real-time WebSocket client
  (RULES.md §2). The route files in `src/app/` stay plain thin wrappers
  (`02-routes-and-navigation.md`); the screens carry the interactivity.
- **Styling**: Tailwind `className` ONLY — no static `style={{…}}` objects, no
  CSS files of your own, no hardcoded colors (`bg-[#…]` is banned by ESLint).
  Colors come from the semantic tokens in `frontend/global.css`:

  | Token (utility) | Use in the lobby |
  |---|---|
  | `bg-background` | screen background |
  | `bg-surface` / `bg-surface-muted` | cards, inset panels, the room-code plate |
  | `text-foreground` / `text-muted-foreground` | primary / secondary text |
  | `bg-primary` + `text-primary-foreground` | primary action (Create / Join button) |
  | `border-border` | dividers, outlines |
  | `text-destructive` / `bg-destructive` | validation + server error messages, the danger variant |
  | `text-success` | connected / ready / copied feedback |

  Dark mode is automatic (system theme, `prefers-color-scheme`) — never write
  `dark:` overrides for these, the token values switch themselves.
- **No component library**: buttons, inputs, cards, dialogs and toasts come
  from `@/components/common/…` — the primitives YOU build (§1 below). Never
  hand-roll per-screen copies, never install a UI kit (new dependency = team
  PR).
- **Responsive by default**: mobile portrait first, then tablet/desktop. No
  fixed page widths; columns and rows collapse to one column on narrow
  screens; every input/button keeps a comfortable size (`min-h-11` ≈ 44px
  touch target).
- **Accessibility is not optional**: real `<button>`/`<input>` elements,
  `htmlFor`-linked labels, `aria-invalid` + `aria-describedby` on errored
  inputs, and a **visible focus state on every control** (e.g.
  `focus-visible:outline-2 focus-visible:outline-offset-2
  focus-visible:outline-primary`). Keyboard operation is part of the manual
  checklist (`03-manual-testing.md`).
- **Fixtures**: while F3/backend are landing, import from
  `@/types/protocol.fixtures` (`sampleRoom`, `sampleRoomCreatedEvent`, …) to
  render screens with realistic data; swap for F3's hooks when they exist (§5).

---

## 1. Shared UI primitives — `src/components/common/` (yours)

There is no component library in the stack: F1 builds the shared primitives
once, and every module imports them (RULES.md §1.3). The initial set:

| Primitive | File | Responsibility |
|---|---|---|
| `Button` | `src/components/common/button.tsx` | all actions — variants `primary` / `secondary` / `danger`; `loading` + `disabled` states (spinner + `aria-busy`); renders a native `<button>` |
| `Input` | `src/components/common/input.tsx` | labeled text field — `label`, `hint`, `error` (wired to `aria-invalid`/`aria-describedby`), `maxLength`, `value`/`onChange` |
| `Card` | `src/components/common/card.tsx` | surface panel — `bg-surface`, `border-border`, rounded; layout via `className`/children, not a rigid slot system |
| `Dialog` | `src/components/common/dialog.tsx` | modal confirmations (e.g. "Leave the room?") — `role="dialog"`, `aria-modal`, focus trap, `Escape` closes, focus returns to the trigger |
| `Toast` | `src/components/common/toast.tsx` | transient non-blocking messages ("Copied!", recoverable errors) — `role="status"`/`aria-live="polite"`, auto-dismiss |

Loading and error presentation live inside the primitives (`Button` loading,
`Input` error) — do not build separate loading/error components in the MVP.
Import them granularly (`import { Button } from "@/components/common/button";`)
— no barrel file needed.

Rules (binding for every primitive):

- **Token-themed**: colors only via the token utilities in the table above;
  dark mode must work with zero per-component work. No hardcoded colors, no
  `bg-[#…]`.
- **Keyboard accessible**: native elements, operable with Tab / Enter /
  Escape alone; the `Dialog` traps focus and restores it on close.
- **Focus-visible**: every interactive element shows a clear focus state via
  `focus-visible:` utilities — never `outline-none` without a replacement.
- **Client components**: files that hold state/effects start with
  `"use client";`. A primitive with zero interactivity (`Card`) can stay a
  plain component — don't sprinkle the directive where it isn't needed.
- **No chess-specific UI** — that is F2's (`src/components/chess/`), never
  here. No lobby-specific copy either: primitives are generic.
- **Additions go through a short PR** (RULES.md §1.3): a new primitive or a
  new variant is a team-visible change to shared surface.

`Button` skeleton (the pattern for the rest — the real surface is yours):

```tsx
// src/components/common/button.tsx
"use client";

import type { PropsWithChildren } from "react";

type ButtonVariant = "primary" | "secondary" | "danger";

interface ButtonProps extends PropsWithChildren {
  variant?: ButtonVariant;
  loading?: boolean; // spinner + aria-busy; blocks further presses
  disabled?: boolean;
  type?: "button" | "submit";
  onClick?: () => void;
}

export function Button({
  variant = "primary",
  loading = false,
  disabled,
  type = "button",
  onClick,
  children,
}: ButtonProps) {
  // TODO(F1): variant → token classes (primary: bg-primary text-primary-foreground;
  //           secondary: border-border bg-surface text-foreground;
  //           danger: bg-destructive text-background);
  //           min-h-11 touch target; focus-visible outline; loading/disabled states
  return (
    <button
      type={type}
      disabled={disabled || loading}
      aria-busy={loading}
      onClick={onClick}
      className="…" // TODO(F1): token classes — see the rules above
    >
      {children}
    </button>
  );
}
```

## 2. Home screen — `src/screens/lobby/home-screen.tsx`

The app's entry point, served at `/` (`src/app/page.tsx` renders it — see
`02-routes-and-navigation.md`). It carries the whole pre-game flow in one
screen — there is no separate create/join route:

- clear branding ("Rechess"), **no** sign-up or login entry anywhere
  (brief §6);
- one **nickname** input shared by both actions;
- the primary action **Create room**;
- the join section: a **room code** input + **Join room**.

Skeleton (partial — the real work is yours):

```tsx
// src/screens/lobby/home-screen.tsx
"use client";

import { useState } from "react";

import { useRouter } from "next/navigation";

import { Button } from "@/components/common/button";
import { Input } from "@/components/common/input";

// TODO(F1): when F3 publishes hooks — import { useRoomActions } from "@/hooks/<F3's module>";
// TODO(F1): mock phase — drive the UI with sampleRoomCreatedEvent from "@/types/protocol.fixtures"

export function HomeScreen() {
  const router = useRouter();
  const [nickname, setNickname] = useState("");
  const [roomCode, setRoomCode] = useState("");
  // TODO(F1): trim + validate the nickname (1–20 chars, no control chars)
  // TODO(F1): createRoom(nickname) — loading/disabled, no double submits
  // TODO(F1): joinRoom(normalize(roomCode), nickname) — same
  // TODO(F1): on success → router.push(`/game/${roomCode}`)
  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-6 px-6">
      <h1 className="text-4xl font-bold text-foreground">Rechess</h1>
      <p className="text-center text-base text-muted-foreground">
        Play chess with a friend — no account needed.
      </p>
      {/* TODO(F1): <Input label="Nickname" value={nickname} onChange={…} /> */}
      {/* TODO(F1): <Button variant="primary" className="min-w-[220px]">Create room</Button> */}
      {/* TODO(F1): <Input label="Room code" value={roomCode} onChange={…} /> */}
      {/* TODO(F1): <Button variant="secondary" className="min-w-[220px]">Join room</Button> */}
    </main>
  );
}
```

Layout notes: center the block vertically, `px-6` gutters, `gap-6` rhythm; on
narrow screens everything stacks in one column. Keep the screen this small —
the placeholder in `src/app/page.tsx` is the size reference. If the file
grows, split the two actions into local `create-room-form.tsx` /
`join-room-form.tsx` components in this folder; do not add routes for them.

### 2.1 Nickname validation

**Validation (mirror `docs/protocol-v1.md` §3 so the server never has to
reject it):** nickname required, **1–20 characters**, **no control
characters** (`/[\u0000-\u001F\u007F]/`), trimmed before send. Disable the
submitting button while the input is invalid or while the request is in
flight (loading state) — no double submits.

### 2.2 What happens on submit — create

Your screen calls F3's action — you never build an envelope or open a socket.
The exact wire command F3 sends for you is (from `src/types/protocol.ts`):

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
  player_token: string; // F3 stores this in its storage module — your screen NEVER sees/stores/logs it
  color: PlayerColor;   // "white" for the host
  room: RoomPublicState;
}
```

Your screen only consumes the **non-secret** result — room code + your color —
as returned by F3's `createRoom` action / `useRoomState()`. On success:
navigate to the waiting room, which lives at `/game/[roomCode]` (the room code
goes in the URL path, uppercase; `player_token` NEVER — RULES.md §3).

## 3. Join flow (on the home screen)

Form: room code + nickname → submit → waiting room.

**Room-code normalization (client-side, before calling F3):** trim outer
whitespace, uppercase, and reject anything that isn't exactly **8 characters
from the unambiguous alphabet `ABCDEFGHJKMNPQRSTUVWXYZ23456789`** after
normalization (no `I`, `L`, `O`, `0`, `1`). Per `docs/protocol-v1.md` §3 the
server normalizes case-insensitive input too, but a clean client
normalization (`code.trim().toUpperCase()`) gives instant feedback and avoids
a round trip. Wire command F3 sends for you:

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

The room-code `Input` gets `maxLength={8}`; normalization handles case and
whitespace before send. Reuse the nickname validation from §2.1 for the join
submit.

## 4. Waiting room — `src/screens/lobby/waiting-room.tsx`

Not a route — it is the panel rendered at `/game/[roomCode]` while the room
status is `"waiting"` (F4's game composition mounts it; the contract is
`04-acceptance.md` §3). Shown to host and guest after create/join succeeds.

Must show:

- the **room code prominently** — big plate (`text-3xl font-bold
  tracking-widest` on `bg-surface-muted` with `border-border`, optionally
  inside a `Card`);
- a **copy button** using the browser clipboard API:

```tsx
// on click (guard for contexts where the API is unavailable)
if (navigator.clipboard) {
  await navigator.clipboard.writeText(roomCode);
  // then a small "Copied" confirmation (text-success or a Toast)
}
```

  `navigator.clipboard` requires a secure context — `http://localhost:3001`
  qualifies; keep a graceful fallback (the code renders as selectable text
  for manual copy) for older browsers.
- your nickname + assigned color (White for the host, Black for the guest);
- the opponent's seat as a placeholder ("Waiting for opponent…") until they
  join;
- a connection state indicator driven by F3's `useConnection()` (never a
  local timer);
- a **Leave** action: confirm inside a `Dialog` ("Leave the room? The other
  player will be notified.") → on confirm call F3's `leaveRoom()` →
  `router.push("/")`.

Read the room code from the URL (`useParams` — the URL is the address) and
the live room state from F3's `useRoomState()` (players/status are
server-authoritative — fixture `sampleRoom` shows the one-player shape):

```ts
export const sampleRoom: RoomPublicState = {
  room_code: "TESTCODE1",
  status: "waiting",
  players: [{ id: "host", nickname: "Host", color: "white", connected: true }],
};
```

### 4.1 Transition to the game — no navigation

When both seats are filled — i.e. room `status` flips from `"waiting"` to
`"active"` via F3's store — the waiting room simply unmounts and F4's game UI
takes over **on the same URL**. Never navigate on a timer or on local
optimism; the server is authoritative (RULES.md §4). This is also why the URL
is shareable: reloading `/game/TESTCODE1`, or opening it directly, resumes
the session via F3's service (token from F3's storage — never the URL).

Skeleton:

```tsx
// src/screens/lobby/waiting-room.tsx
"use client";

import { useState } from "react";

import { useParams, useRouter } from "next/navigation";

// TODO(F1): F3's useRoomState()/useConnection(); mock with sampleRoom / sampleRoomCreatedEvent first

export function WaitingRoom() {
  const { roomCode } = useParams<{ roomCode: string }>();
  const router = useRouter();
  const [copied, setCopied] = useState(false);
  // TODO(F1): copy via navigator.clipboard.writeText(roomCode) + "Copied" feedback
  // TODO(F1): render players[0]/players[1] seats + "Waiting for opponent…" placeholder
  // TODO(F1): connection indicator from useConnection()
  // TODO(F1): Leave → Dialog confirm → leaveRoom() → router.push("/")
  return (
    <section className="flex flex-1 flex-col items-center justify-center gap-6 px-6">
      {/* TODO(F1): room code plate */}
      <p className="text-3xl font-bold tracking-widest text-foreground">{roomCode}</p>
      {/* TODO(F1): copy button + copied feedback; seats; connection indicator; leave */}
    </section>
  );
}
```

## 5. Building before F3 exists (fixture/mock mode)

Until F3's hooks land, render each screen against the fixtures in
`src/types/protocol.fixtures.ts`: drive the waiting room with `sampleRoom`
(1 player) and a hand-written two-player `RoomPublicState`, and simulate
`room.created` with `sampleRoomCreatedEvent` (its `room` field is the same
public state). Keep the fixture usage behind a small local seam (a
`const MOCK = …` toggle or a prop) so swapping in F3's real hooks is a
one-line change, not a rewrite.
