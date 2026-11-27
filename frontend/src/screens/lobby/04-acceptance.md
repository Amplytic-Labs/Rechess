# 04 — Acceptance, definition of done, and handoff contracts

## 1. Definition of done — mapped to your brief

Each row quotes a requirement from the F1 brief
(`06_F1_App_Foundation_Lobby.md` — written for the pre-migration mobile
stack); the check is what "done" means on the current Next.js web stack.

| Brief requirement | Done means |
|---|---|
| "Create or configure the … TypeScript application… commit the lockfile" | Pre-done by the base repo (Next.js 16 / React 19 / TS strict; lockfile committed). Nothing for you to redo; do not add dependencies. |
| "Set up … routes for Home, Create Room, Join Room, Waiting Room and Game" | The route set is FIXED and scaffolded: `/` = home (nickname + create/join) and `/game/[roomCode]` = waiting/game/result driven by room + snapshot status. The five planned screens collapse into these two routes; route files stay thin. |
| "Create shared theme tokens…" | Pre-done (`global.css`); your screens and primitives use ONLY those tokens. |
| "Build the home screen with Create Room and Join Room entry points" | `home-screen.tsx` + `src/app/page.tsx` render nickname + create + join; no account UI. |
| "Build a create-room flow with nickname input, validation and a loading/error state. Call F3's createRoom action" | 1–20 chars, no control chars; disabled/loading states; error table; calls F3's action — never a socket. |
| "Build a join-room flow with nickname and room code input. Normalize room code safely… show server errors clearly" | trim + uppercase + alphabet validation; ROOM_NOT_FOUND/ROOM_FULL/ROOM_EXPIRED/INVALID_PAYLOAD mapped to friendly themed text. |
| "Build waiting-room screen showing room code, host/guest identity, assigned color, opponent waiting state and connection state" | `WaitingRoom` rendered at `/game/[roomCode]` while `status === "waiting"`: room code prominent + copy (`navigator.clipboard`); your nickname + color; opponent placeholder; connection from F3's `useConnection()`. |
| "Add room-code copy/share affordance" | Copy button (`navigator.clipboard.writeText`); optional share via the Web Share API where available (`navigator.share`, feature-detected). |
| "Handle navigation transitions using stable room code… avoid putting reconnect tokens in route params, deep links or logs" | Room code only in the URL path; `player_token` never in a URL, link or log. |
| "Provide F3 with screen-level requirements and F4 with the route contract" | §2 and §3 below (names frozen); this doc set is the record. |
| "Nickname required; documented maximum length; prevent control characters" | maxLength 20 + control-char validation client-side (server re-validates). |
| "Room code input handles lowercase/uppercase and whitespace consistently" | trim + uppercase normalization before send. |
| "Show clear waiting, connecting, opponent joined, opponent disconnected and error states" | Waiting-room checklist rows 3–8 in `03-manual-testing.md` (waiting/connecting/joined/disconnected/resume); error states in create/join checklist §2.6 and §3.5–3.8. |
| "Buttons have disabled/loading states" | Verified in manual testing §2–3. |
| "Layouts work on common phone widths…" | Mobile-first responsive layout; nothing clipped at a 375px viewport (the browser chrome owns the screen insets). |
| "No account/signup UI" | Manual check on the home screen. |
| "Accessibility: readable contrast, meaningful labels, adequate touch targets" | Tokens give contrast; labels on inputs; keyboard operable + focus-visible; `min-h-11` (≈44px) targets. |
| "Instructions for running … devices" | `03-manual-testing.md` documents running: `npm run dev` → http://localhost:3001 (backend on :3000 for integration); browser, desktop + mobile viewport. |
| Acceptance (brief §8): "A guest can open the app, choose a nickname, create or join a room, see the room code and waiting state, and transition into the game without any account flow" | Full happy path passes the integration-mode walkthrough of `03-manual-testing.md` end to end in two browser contexts. |
| "Screens use F3's shared networking/state layer rather than direct sockets" | No `new WebSocket` and no game-data `fetch` anywhere in `src/screens/lobby/**`; no Next API routes or server actions. |

## 2. Handoff to F3 — what your screens need from them

The names below are **frozen** (also pinned in F3's
`src/services/03-stores-and-hooks.md` §5 — that doc is the source of truth if
anything here differs):

```ts
useConnection()   // connection slice (status, attempts, lastError) — banners/disabled states
useRoomState()    // { roomCode, players, status, yourColor, connection }
useGameState()    // { snapshot, pending }
useRoomActions()  // createRoom, joinRoom, submitMove, offerDraw, acceptDraw,
                  // resign, rematch, leaveRoom — each sets loading/error
```

(Resume is not a screen action — it is automatic inside F3's service
(`session.resume`); screens never call it. Reload, direct URL entry and
reconnect all resume through it.)

What your screens consume:

- `createRoom(nickname: string)` — resolves with the **non-secret** result
  `{ roomCode, yourColor }`. `player_token` stays inside F3's storage module;
  never return it to a screen.
- `joinRoom(roomCode: string, nickname: string)` — same rule
  (`{ roomCode, yourColor }`).
- `leaveRoom()` — for the waiting-room Leave action.
- `useRoomState()` — `roomCode`, `players`, `status` (`"waiting"` drives the
  waiting-room panel), `yourColor`, `connection`.
- `useConnection()` — connection indicator on the waiting room.
- Error surfaces: each action carries loading + a mapped, displayable error
  (stable `ErrorCode`), so screens never parse raw server events.

**Flag — naming differences between the brief and F3's contract:** the F1
brief sketched `useRoomState() → { roomCode, players, status, myColor,
connectionStatus, error }`; the frozen contract splits connection into its own
`useConnection()` hook (also carried in `useRoomState().connection`) and
names the color field `yourColor`. **The frozen contract wins.**

The hooks live in `src/hooks/` (F3-owned) — confirm the exact import module
paths (`@/hooks/…`) with F3.

## 3. Handoff to F4 — entering the game (same URL, no navigation)

- F1's entry navigation: after `createRoom`/`joinRoom` resolves, the home
  screen navigates `router.push(\`/game/${roomCode}\`)` — room code
  (uppercase) only; never `player_token`, never the nickname.
- From there the URL never changes: F4's composition at `/game/[roomCode]`
  renders **F1's `WaitingRoom`** (`@/screens/lobby/waiting-room`) while
  `useRoomState().status === "waiting"`, and its own game/result UI for
  `active`/`finished`. The waiting → game transition is a
  server-authoritative state flip, not a navigation.
- F4 owns `src/screens/game/**` + `src/app/game/**`; the composition switch
  (including what unmounts the waiting room) is F4's to implement.
- `WaitingRoom` is a client component reading only F3's hooks + `useParams` —
  no props contract needed beyond mounting it.
- Rematch: both players stay at `/game/[roomCode]` — F4's rematch flow, no
  new route.

## 4. Open coordination items (keep this list current)

- [ ] Re-sync F4's docs with the two-route contract — their docs still
      describe a `src/app/game.tsx` route and a navigation into the game;
      the scaffolded truth is `src/app/game/[roomCode]/page.tsx` and no
      waiting→game navigation.
- [ ] Confirm F3's hooks import module paths (`@/hooks/…`) once they land.
- [ ] F3's docs still name pre-migration storage/env pieces (their secure
      storage wrapper, the old env var name); the agreed env var is
      `NEXT_PUBLIC_API_URL` (default `ws://localhost:3000/ws`), read by F3's
      config module only — screens never read it.
- [ ] Primitives: additions beyond the initial set (Button, Input, Card,
      Dialog, Toast) go through a short PR (RULES.md §1.3); add a
      `src/components/common/README.md` documenting the set when the files
      land.
