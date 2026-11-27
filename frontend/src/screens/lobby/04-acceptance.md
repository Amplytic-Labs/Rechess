# 04 — Acceptance, definition of done, and handoff contracts

## 1. Definition of done — mapped to your brief

Each row quotes a requirement from the F1 brief
(`06_F1_App_Foundation_Lobby.md`); the check is what "done" means here.

| Brief requirement | Done means |
|---|---|
| "Create or configure the Expo TypeScript application… commit the lockfile" | Pre-done by the base repo (SDK 58 / RN 0.88 / React 19.3, lockfile committed). Nothing for you to redo; do not add dependencies. |
| "Set up Expo Router and define routes for Home, Create Room, Join Room, Waiting Room and Game" | Home/create/join/waiting route files exist and are thin (Game route is F4's — see §3). |
| "Create shared theme tokens…" | Pre-done (`global.css`); your screens use ONLY those tokens. |
| "Build the home screen with Create Room and Join Room entry points" | `home-screen.tsx` + `src/app/index.tsx` render both actions; no account UI. |
| "Build a create-room flow with nickname input, validation and a loading/error state. Call F3's createRoom action" | 1–20 chars, no control chars; disabled/loading states; error table; calls F3's action — never a socket. |
| "Build a join-room flow with nickname and room code input. Normalize room code safely… show server errors clearly" | trim + uppercase + alphabet validation; ROOM_NOT_FOUND/ROOM_FULL/ROOM_EXPIRED/INVALID_PAYLOAD mapped to friendly themed text. |
| "Build waiting-room screen showing room code, host/guest identity, assigned color, opponent waiting state and connection state" | Room code prominent + copy (`expo-clipboard`); your nickname + color; opponent placeholder; connection from F3's `useConnection()`. |
| "Add room-code copy/share affordance" | Copy button (Clipboard.setStringAsync); optional share via RN `Share`. |
| "Handle navigation transitions using stable room code… avoid putting reconnect tokens in route params, deep links or logs" | Room code only in params; `player_token` never in params/links/logs. |
| "Provide F3 with screen-level requirements and F4 with the route contract" | §2 and §3 below agreed with F3/F4 (names frozen Day 1); this doc set is the record. |
| "Nickname required; documented maximum length; prevent control characters" | maxLength 20 + control-char validation client-side (server re-validates). |
| "Room code input handles lowercase/uppercase and whitespace consistently" | trim + uppercase normalization before send. |
| "Show clear waiting, connecting, opponent joined, opponent disconnected and error states" | Waiting-room checklist rows 3–7 in `03-manual-testing.md`. |
| "Buttons have disabled/loading states" | Verified in manual testing §2–3. |
| "Layouts work on common phone widths and respect safe areas" | `StyledSafeAreaView` + narrow-device check. |
| "No account/signup UI" | Manual check home/create/join screens. |
| "Accessibility: readable contrast, meaningful labels, adequate touch targets" | Tokens give contrast; labels on inputs; ≥44pt targets. |
| Acceptance (brief §8): "A guest can open the app, choose a nickname, create or join a room, see the room code and waiting state, and transition into the game without any account flow" | Full happy path passes the integration-mode walkthrough of `03-manual-testing.md` end to end on two devices. |
| "Screens use F3's shared networking/state layer rather than direct sockets" | No `WebSocket`, no `send` anywhere in `src/screens/lobby/**`. |

## 2. Handoff to F3 — what your screens need from them

Per `src/services/README.md` §7, F3's proposed public API (freeze the names
together on Day 1 — that README is the source of truth if anything here
differs):

```ts
useRoomActions()  // createRoom, joinRoom, resumeRoom, leaveRoom, sendMove,
                  // offerDraw, acceptDraw, resign, rematch — each sets loading/error
useRoomState()    // roomCode, players, status, drawOffer, yourColor
useGameState()    // the snapshot + pending
useConnection()   // connection state for banners/disabled states
```

What your four screens consume:

- `createRoom(nickname: string)` — resolves with the **non-secret** result
  (room code + your color). `player_token` stays inside F3/secure storage;
  never return it to a screen. (Your brief proposed `RoomCreateResult`; F3's
  README doesn't pin the return type — freeze it together.)
- `joinRoom(roomCode: string, nickname: string)` — same rule.
- `leaveRoom()` — for the waiting-room back action.
- `useRoomState()` — `roomCode`, `players`, `status` (`"waiting"` →
  `"active"` drives the transition to the game), `yourColor`.
- `useConnection()` — connection indicator on the waiting room.
- Error surfaces: each action exposes loading + a mapped, displayable error
  (stable `ErrorCode`), so screens never parse raw server events.

**Flag — naming differences between the brief and F3's README:** the F1
brief sketched `useRoomState() → { roomCode, players, status, myColor, connectionStatus, error }`;
F3's README splits connection into its own `useConnection()` hook and names
the color field `yourColor`. **F3's README wins** — use `yourColor` +
`useConnection()`, and confirm the frozen names with F3 before you build.

**Flag — stale testing section in F3's README:** `src/services/README.md` §8
still mentions `npx jest`; RULES.md §5 (no jest, by design) overrides that.
Also: your hooks live in `src/hooks/` (F3-owned) — confirm the module path for
imports with F3.

## 3. Handoff to F4 — navigation into the game

- Trigger: waiting room observes room `status === "active"` (both seats
  filled, server-authoritative, via F3's store) → `router.replace` to the
  game route.
- Params: **room code only** (`router.replace({ pathname: "/game", params: { roomCode } })`). Never `player_token`, never the nickname.
- Route file name/path is F4's call (old plan: `src/app/game.tsx`) — agree
  the exact name on Day 1 and update `02-routes-and-navigation.md` §1 to
  match.
- The game screen takes over back-navigation from there (§4 of
  `02-routes-and-navigation.md`).

## 4. Open coordination items (keep this list current)

- [ ] Freeze hook/action names + import paths with F3 (Day 1).
- [ ] Freeze the game route name/path with F4 (Day 1–3).
- [ ] Freeze the action return types (`createRoom`/`joinRoom` results: room
  code + your color, tokenless).
- [ ] Any `src/components/common/` additions (e.g. a room-code display
  component) go through a short PR — shared, F1 curates.
- [ ] `src/app/_layout.tsx` changes (headers) — coordinated micro-PR only.
