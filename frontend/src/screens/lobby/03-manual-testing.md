# 03 — Manual testing (UI is verified in the browser)

RULES.md §5: `npm run test` (vitest, node environment) covers **pure logic
only** (the wire contract, F3's stores/helpers as they land). Lobby screens
have no component tests — they are verified by **these manual checklists in
the browser**, plus the machine gates. Walk the relevant checklist before
declaring a task done, and note what you checked in the PR description. If
you add a genuinely pure helper (e.g. room-code normalization), a small vitest
unit next to it is welcome — but never a component/UI test (no testing
library is in the stack).

Two modes of testing:

- **Mock mode (before F3/backend land):** screens render against
  `@/types/protocol.fixtures` data (`sampleRoom`, `sampleRoomCreatedEvent`).
  No server needed.
- **Integration mode (from the vertical slice on):** F3's hooks wired to the
  real backend (`cargo run` in `backend/` — it listens on :3000; the frontend
  dev server is `http://localhost:3001`, and `NEXT_PUBLIC_API_URL` defaults
  to `ws://localhost:3000/ws`, so no `.env` is needed locally). Use **two
  separate browser contexts** for host + guest (one normal window + one
  private/incognito window, or two browser profiles) — two tabs of the same
  window share F3's token storage and would collide.

**Every run, three dimensions:**

- **Desktop + mobile viewport:** check every screen in a desktop window and at
  a phone width (browser devtools device toolbar, e.g. 375px). No fixed
  widths, nothing clipped, buttons full-width with gutters.
- **Keyboard:** tab through the form — every control reachable in a sensible
  order, focus ring visible on all of them, Enter submits, Escape closes the
  Dialog.
- **Dark mode:** switch the OS appearance (or devtools → Rendering → Emulate
  `prefers-color-scheme: dark`) and re-check each screen — tokens switch
  automatically, so any screen that looks wrong in dark mode has a hardcoded
  color somewhere.

---

## 1. Home screen (`/`)

Preconditions: `cd frontend && npm run dev` → open http://localhost:3001 in
the browser.

| # | Step | Expected |
|---|---|---|
| 1 | Page loads | "Rechess" branding + nickname + Create room + Join room section; no login/signup anywhere |
| 2 | Narrow viewport (375px, device toolbar) | nothing clipped; nickname, create action and join section stack in one column with gutters |
| 3 | Tab through the form | every control reachable in a sensible order; focus ring visible on each |
| 4 | Toggle dark mode → repeat 1–3 | identical layout, correct light/dark token values |

## 2. Create flow (on the home screen)

Preconditions: on `/`; mock mode or backend running.

| # | Step | Expected |
|---|---|---|
| 1 | Empty nickname, look at Create | button disabled; friendly inline hint |
| 2 | Type 21+ characters | input capped at 20 (maxLength) or validation error shown |
| 3 | Type a nickname with control characters (paste text with a newline) | validation error, submit stays disabled |
| 4 | Valid nickname → click Create | button enters loading/disabled state; double-clicks do nothing |
| 5 | Success (mock: `sampleRoomCreatedEvent`) | navigates to `/game/TESTCODE1` |
| 6 | Force a server error (stop the backend / mock an error code) | friendly themed message from the mapping table — never raw JSON; button re-enables for retry |
| 7 | Toggle dark mode → repeat 1–6 | theming correct; error text uses `text-destructive` |

## 3. Join flow (on the home screen)

Preconditions: on `/`; a room exists (integration: host created one in a
second browser context; mock: a fixture code).

| # | Step | Expected |
|---|---|---|
| 1 | Empty fields | Join button disabled |
| 2 | Room code in lowercase with spaces: ` testcode1 ` | normalization (trim + uppercase) applied before send; code shows as 8 uppercase letters |
| 3 | Code with invalid chars (`I`, `L`, `O`, `0`, `1` or 7 letters) | inline error before any request is sent |
| 4 | Valid code + nickname → Join | loading/disabled state; on success → `/game/TESTCODE1` as Black |
| 5 | Error path: code that doesn't exist | `ROOM_NOT_FOUND` → "No room with that code. Check the code and try again." |
| 6 | Error path: join a room that already has 2 players | `ROOM_FULL` → "That room already has two players." |
| 7 | Error path: an expired room's code | `ROOM_EXPIRED` → "That room has expired. Create a new one." |
| 8 | Error path: empty nickname forced past client checks | `INVALID_PAYLOAD` → "Please check your nickname and room code." |
| 9 | Toggle dark mode → repeat | all states themed correctly |

## 4. Waiting room (at `/game/[roomCode]`)

Preconditions: host browser context at `/game/TESTCODE1` (mock: `sampleRoom`).
Until F4's composition lands, verify the `WaitingRoom` panel through your
mock seam (render it with fixture data) — the route itself still shows F4's
placeholder.

| # | Step | Expected |
|---|---|---|
| 1 | Arrive as host | room code displayed large and prominent |
| 2 | Click **Copy** | code in the clipboard (paste anywhere to verify); "Copied" feedback (e.g. `text-success` or a Toast) |
| 3 | Look at seats | your nickname + White; opponent seat shows "Waiting for opponent…" placeholder |
| 4 | Connection state | indicator reflects F3's `useConnection()` (connected/reconnecting/disconnected) |
| 5 | Guest joins in the second browser context (integration) | opponent seat fills with their nickname + Black — no manual refresh |
| 6 | Both seats filled | game UI takes over **on the same URL** — no navigation, no page reload; browser back → `/`, forward → returns to the game, still working |
| 7 | Guest disconnects (close their window) (integration) | host sees opponent disconnected state — never a crash |
| 8 | Reload the page (`F5`) on the waiting room | session resumes via F3's service; waiting room returns with seats intact |
| 9 | Press **Leave** | confirm Dialog ("Leave the room?…"); confirm → room left (seat freed) and back on `/` |
| 10 | Toggle dark mode → repeat 1–9 | theming correct throughout |

## 5. Before you call it done (run EVERY time)

- [ ] `cd frontend && npm run typecheck` — zero errors
- [ ] `cd frontend && npm run lint` — zero errors **and** zero warnings
- [ ] `cd frontend && npm run test` — vitest green (wire contract + any pure helpers)
- [ ] `cd frontend && npm run build` — production build passes (CI runs it too)
- [ ] Every screen checked in light AND dark mode, desktop + 375px viewport, mouse + keyboard
- [ ] `grep -rE "#[0-9a-fA-F]{3,8}" src/screens/lobby/ src/app/page.tsx src/components/common/` — no hardcoded colors (no `bg-[#…]`, no color literals)
- [ ] No static `style={{…}}` objects in your diff (the `style` prop is for dynamic values that cannot be utilities only — RULES.md §1.1)
- [ ] No `player_token` in any URL, link, log or error message
- [ ] Server error paths show mapped friendly messages, never raw JSON
- [ ] Buttons: disabled while invalid, loading while pending, no double submits
- [ ] Walked the checklists above and noted results in the PR description
