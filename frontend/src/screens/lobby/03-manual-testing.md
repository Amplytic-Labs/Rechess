# 03 — Manual testing (this replaces unit tests)

RULES.md §5: the frontend has **no unit-test runner by design** (jest-expo is
broken by an upstream expo-modules-core ESM bug). Quality gates are
`npm run typecheck` + `npm run lint` + **these manual checklists**. Walk the
relevant checklist before declaring a task done, and note what you checked in
the PR description. Never write jest tests or test files.

Two modes of testing:

- **Mock mode (before F3/backend land):** screens render against
  `@/types/protocol.fixtures` data (`sampleRoom`, `sampleRoomCreatedEvent`).
  No server needed.
- **Integration mode (from the Day 3–4 vertical slice):** F3's hooks wired to
  the real backend dev server. Backend must be running (see B3/B4's runbook;
  F3's `src/services/README.md` documents the dev connection URL, e.g. a LAN
  IP `ws://…`). Use **two devices/simulators** for host+guest flows.

**Dark mode, every time:** toggle the simulator appearance (iOS: Features →
Toggle Appearance; Android: system dark theme) and re-check each screen —
tokens switch automatically, so any screen that looks wrong in dark mode has
a hardcoded color somewhere.

---

## 1. Home screen

Preconditions: app launched (`npm start`, simulator or Expo Go).

| # | Step | Expected |
|---|---|---|
| 1 | App opens | "Rechess" branding + Create room / Join room buttons; no login/signup anywhere |
| 2 | Tap **Create room** | navigates to `/create-room` |
| 3 | Tap **Join room** | navigates to `/join-room` |
| 4 | Toggle simulator appearance → repeat 1–3 | identical layout, correct light/dark token values |
| 5 | Narrow device (e.g. 375pt / small Android) | nothing clipped, buttons full-width with gutters |

## 2. Create room

Preconditions: on `/create-room`; mock mode or backend running.

| # | Step | Expected |
|---|---|---|
| 1 | Empty nickname, look at Create button | button disabled; friendly inline hint |
| 2 | Type 21+ characters | input capped at 20 (maxLength) or validation error shown |
| 3 | Type a nickname with control characters (paste text with a newline) | validation error, submit stays disabled |
| 4 | Valid nickname → tap Create | button enters loading/disabled state; double-taps do nothing |
| 5 | Success (mock: `sampleRoomCreatedEvent`) | navigates to `/waiting-room` with the room code |
| 6 | Force a server error (stop the backend / mock an error code) | friendly themed message from the mapping table — never raw JSON; button re-enables for retry |
| 7 | Toggle dark mode → repeat 1–6 | theming correct; error text uses `text-destructive` |

## 3. Join room

Preconditions: on `/join-room`; a room exists (integration: host created one on
a second device; mock: a fixture code).

| # | Step | Expected |
|---|---|---|
| 1 | Empty fields | Join button disabled |
| 2 | Room code in lowercase with spaces: ` testcode1 ` | normalization (trim + uppercase) applied before send; code shows as 8 uppercase letters |
| 3 | Code with invalid chars (`I`, `L`, `O`, `0`, `1` or 7 letters) | inline error before any request is sent |
| 4 | Valid code + nickname → Join | loading/disabled state; on success → `/waiting-room` as Black |
| 5 | Error path: code that doesn't exist | `ROOM_NOT_FOUND` → "No room with that code. Check the code and try again." |
| 6 | Error path: join a room that already has 2 players | `ROOM_FULL` → "That room already has two players." |
| 7 | Error path: an expired room's code | `ROOM_EXPIRED` → "That room has expired. Create a new one." |
| 8 | Error path: empty nickname forced past client checks | `INVALID_PAYLOAD` → "Please check your nickname and room code." |
| 9 | Toggle dark mode → repeat | all states themed correctly |

## 4. Waiting room

Preconditions: host device on `/waiting-room` (mock: `sampleRoom`).

| # | Step | Expected |
|---|---|---|
| 1 | Arrive as host | room code displayed large and prominent |
| 2 | Tap **Copy** | code in clipboard (paste anywhere to verify); "Copied" feedback (e.g. `text-success`) |
| 3 | Look at seats | your nickname + White; opponent seat shows "Waiting for opponent…" placeholder |
| 4 | Connection state | indicator reflects F3's `useConnection()` (connected/reconnecting/disconnected) |
| 5 | Guest joins on the second device (integration) | opponent seat fills with their nickname + Black — no manual refresh |
| 6 | Both seats filled | app navigates to the game route (F4's) with the room code; back from game does NOT return to waiting |
| 7 | Guest disconnects (kill the guest app) (integration) | host sees opponent disconnected state — never a crash |
| 8 | Press back on the waiting room | confirm dialog; confirm → room left (seat freed) and back on home |
| 9 | Toggle dark mode → repeat 1–8 | theming correct throughout |

## 5. Before you call it done (run EVERY time)

- [ ] `cd frontend && npm run typecheck` — zero errors
- [ ] `cd frontend && npm run lint` — zero errors **and** zero warnings
- [ ] Every screen checked in light AND dark mode
- [ ] `grep -rE "#[0-9a-fA-F]{3,8}" src/screens/lobby/ src/app/` — no hardcoded colors (no `bg-[#…]`, no color literals)
- [ ] No `style={{…}}` inline styles; no `StyleSheet` anywhere in your diff
- [ ] No `player_token` in any route param, link, log or error message
- [ ] Server error paths show mapped friendly messages, never raw JSON
- [ ] Buttons: disabled while invalid, loading while pending, no double submits
- [ ] Walked the checklists above and noted results in the PR description
