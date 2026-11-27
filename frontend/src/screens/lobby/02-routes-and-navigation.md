# 02 — Routes and navigation

## 1. The two routes (FIXED)

The route set is **frozen** — do not add, rename or remove routes. Both are
already scaffolded as placeholders in `src/app/`:

| Route | Route file | Renders | Status |
|---|---|---|---|
| `/` | `src/app/page.tsx` | `HomeScreen` from `@/screens/lobby/home-screen` (**yours**) | placeholder — replace the body |
| `/game/[roomCode]` | `src/app/game/[roomCode]/page.tsx` | F4's game composition; while room `status === "waiting"` it renders F1's `WaitingRoom` from `@/screens/lobby/waiting-room` | **F4's file — do not edit** |

Why only two: the earlier five-screen plan (home / create / join / waiting /
game as separate routes) collapsed in the web migration — create + join
became sections of the home screen, and waiting / game / result are one URL
driven by the room + snapshot status the server owns. A shareable,
refresh-safe game URL is the point: `/game/TESTCODE1` survives a reload or a
direct open because F3's service resumes the session from its storage.

RULES.md §3: route files stay **thin** — they import a screen component from
`src/screens/**` and render it. UI and state live in the screen components,
so they stay reviewable and router-independent.

The thin-wrapper pattern (exact):

```tsx
// src/app/page.tsx
import { HomeScreen } from "@/screens/lobby/home-screen";

export default function HomePage() {
  return <HomeScreen />;
}
```

The screen component carries `"use client"` — the route file itself stays a
plain server component that only imports and renders it. Your home route
takes no params, so there is nothing else to do in it.

`src/app/layout.tsx` is done (root layout, imports `global.css`, paints
`bg-background text-foreground`). You normally need **no** layout change. If
you genuinely need one (e.g. page metadata), it is shared foundation — make
it a tiny coordinated PR, don't freestyle.

## 2. Navigation flow

```
   ┌──────────────────────────────────┐
   │  /                (home screen)  │
   │  nickname + create / join        │
   └─────────────────┬────────────────┘
                     │  createRoom() / joinRoom() ok
                     ▼
   ┌──────────────────────────────────────────────┐
   │  /game/[roomCode]                            │
   │    room status "waiting"  → WaitingRoom  (F1)│
   │    room status "active"   → game UI      (F4)│
   │    finished               → result view  (F4)│
   └──────────────────────────────────────────────┘
    ▲  reload / open the URL directly → F3's service resumes
```

No navigation ever happens waiting → game → result: the URL stays
`/game/[roomCode]` and the composition switches on server-owned state.

## 3. Link, useRouter, useParams — and what may never be in a URL

```tsx
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";

// programmatic navigation (after createRoom/joinRoom resolves, or Leave)
const router = useRouter();
router.push(`/game/${roomCode}`);
router.push("/");

// declarative links (e.g. a "back to home" text link)
<Link href="/">…</Link>

// read the room code inside a client screen component
const { roomCode } = useParams<{ roomCode: string }>();
```

- Import `Link` from `next/link`, `useRouter`/`useParams` from
  `next/navigation` — never `react-router` or similar (RULES.md §3).
- `useRouter().push` after an action resolves (create/join success, leave
  confirm); `Link` for static navigation targets.
- **Room code in the URL path: OK.** It is a public identifier, not a
  credential — that is what makes the game URL shareable. Uppercase it before
  navigating (`/game/TESTCODE1`).
- **`player_token` in a URL: NEVER** (RULES.md §3, brief §2). It never
  appears in paths, query strings, links or logs; F3's service keeps it in
  F3's storage module and resumes the session after navigation or reload.
  Your screens never even hold it.
- Don't put anything else in the URL — the nickname lives in the
  store/F3's action call, not the query string.

## 4. Leaving the room and the back button (your choices to implement)

The web has no controlled hardware back — define what the in-app
affordances do:

| From | What the user does | What happens |
|---|---|---|
| `/` (home) | nothing to leave | — |
| `/game/[roomCode]`, waiting | explicit **Leave** button (confirm inside a `Dialog`) | call F3's `leaveRoom()` (the seat frees server-side), then `router.push("/")` |
| `/game/[roomCode]`, any state | browser back / reload / open the URL directly | history navigation only — the room keeps running server-side; returning to `/game/TESTCODE1` (or reloading it) resumes via F3's service. No `beforeunload` blockers in the MVP |
| `/game/[roomCode]`, active/finished | in-game leave/resign | **F4's** call (`src/screens/game/02-controls-and-results.md`) |

Reload/direct-open with a stale or invalid stored session ⇒ F3's service
surfaces the error (`INVALID_TOKEN` etc.) and the UI returns to `/` with a
friendly message (`docs/protocol-v1.md` §8) — coordinate the exact toast
placement with F3; the home screen hosts it.

Keep this table as a short comment in the screen components so reviewers see
the intent.

## 5. Keeping routes thin (checklist)

- Zero styling in route files — not even `className`.
- Zero state/logic in route files; params are read **inside the screen
  component** with `useParams`, not forwarded as props. (For routes with
  params — F4's game route — passing the awaited `params` prop down instead
  is equally valid; that is F4's choice. Your home route has no params.)
- One default export per route file, importing exactly one screen component.
- All interactivity (`"use client"`) lives in `src/screens/**`, never in the
  route file.
