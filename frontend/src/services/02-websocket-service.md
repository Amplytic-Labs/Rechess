# 02 — The WebSocket service (`src/services/websocket.ts`)

> One connection per app. It is a **module-level singleton service, not a
> hook** — hooks (doc 03) wrap it; screens never see it. It runs in the
> **browser**, imported (directly or via stores/hooks) from client
> components only; there are no Next API routes or server actions for
> game data — the Rust WebSocket server is the only data source
> (RULES.md §2). B3's plan (`backend/src/network/01-axum-websocket-guide.md`,
> `02-protocol-implementation.md`) is the server side of everything here;
> your client must match it.

## 1. The base that already exists (don't rebuild it)

- **No socket library**: every modern browser ships a global `WebSocket`
  class — callback-based (`onopen`, `onmessage`, `onclose`, `onerror`,
  `send(string)`, `close(code?, reason?)`, plus `readyState`). No
  dependency, no polyfill, no team PR (RULES.md §2 makes new
  dependencies team decisions — none are needed here).
- `zustand ~5.1.0` is installed and that is the whole runtime surface
  (verify `frontend/package.json` before adding anything). The wire
  types are done: `src/types/protocol.ts` (doc 01).
- The browser's `CloseEvent` carries `code` and `reason` — you need them
  to tell the accepted-`room.leave` close (1000, "left room") from
  errors (§8). A failed connection fires `onerror` and then `onclose` —
  make `onclose` the single authoritative transition point.
- **Where this code runs**: client components are pre-rendered on the
  server for the initial HTML, so your module's top level is ALSO
  evaluated in Node during `npm run build` and the dev render pass.
  Never touch `window`, `localStorage`, `WebSocket` or `document` at
  module top level — open sockets and read storage only inside
  functions called from effects/handlers, and guard any module-level
  browser-event registration with `typeof window !== "undefined"`.

## 2. The connection URL — the config module

Single endpoint: `GET /ws` — `ws://localhost:3000/ws` in local dev,
`wss://<host>/ws` in production (TLS at the reverse proxy, B4's
deployment doc). The server binds `0.0.0.0:3000` by default
(`RECHESS_HOST`/`PORT` env vars, `backend/src/main.rs`); `npm run dev`
serves the app at `http://localhost:3001`.

One small module owns the URL: `src/services/config.ts` — the ONLY code
that reads the environment. It reads `NEXT_PUBLIC_API_URL` and defaults
to `ws://localhost:3000/ws`, so same-machine local dev needs **no
configuration at all**:

```ts
// src/services/config.ts — the ONLY module that reads the WS URL env
const DEFAULT_WS_URL = "ws://localhost:3000/ws"; // same-machine local dev

/** Resolve the WebSocket endpoint from NEXT_PUBLIC_API_URL. */
export function resolveWsUrl(): string {
  const raw = process.env.NEXT_PUBLIC_API_URL?.trim();
  if (!raw) return DEFAULT_WS_URL;
  // Accept either a full ws(s)://…/ws URL or an http(s) base URL.
  const wsBase = raw
    .replace(/^http:\/\//i, "ws://")
    .replace(/^https:\/\//i, "wss://");
  return wsBase.endsWith("/ws")
    ? wsBase
    : `${wsBase.replace(/\/+$/, "")}/ws`;
}
```

Set it per developer in `frontend/.env.local` (gitignored — `.env*` is
in `.gitignore`; never commit it):

```bash
# frontend/.env.local — gitignored, per machine
# Same-machine local dev: NO entry needed (default ws://localhost:3000/ws).
# Real phone/tablet on your LAN (same network as the backend):
#   NEXT_PUBLIC_API_URL=ws://192.168.1.20:3000/ws
# Production build: NEXT_PUBLIC_API_URL=wss://chess.example.com/ws
```

Gotchas worth knowing now:

- `NEXT_PUBLIC_` variables are **inlined at build time** by Next.js —
  editing `.env.local` requires restarting `npm run dev`. It is not
  runtime config; the production value is baked in when B4 builds.
- `localhost` in the browser really is the local machine — same-machine
  dev just works. From a phone, `localhost` means the phone itself: use
  the dev machine's LAN IP (and see the secure-context caveat in §3
  before real-device testing over plain http).
- Production must be `wss://` — plain `ws://` to a public host is a
  security bug (RULES.md §4), and an `https://` page cannot open an
  insecure `ws://` socket anyway (mixed-content blocking), so the
  reverse proxy must terminate TLS.

## 3. The send path

Every command goes out wrapped in a `ClientEnvelope` — this exact shape
from `protocol.ts`:

```ts
export type ClientEnvelope = { v: number; id: string } & ClientCommand;
```

So one `game.move` on the wire looks like the spec example:

```json
{ "v": 1, "id": "<uuid>", "type": "game.move",
  "payload": { "from": "e2", "to": "e4", "promotion": null, "expected_revision": 12 } }
```

Rules:

- `v` is **always** `PROTOCOL_VERSION` (from `protocol.ts`). The server
  version-gates every frame: `v !== 1` ⇒ `server.error
  UNSUPPORTED_VERSION` and then the server closes (code 1008).
- `id` is a fresh UUID per command from the browser's global
  `crypto.randomUUID()` — never reuse an id, never send without one;
  the server logs it and B2 plans dedup around it. It is a
  secure-context API: available on `localhost` and any `https://`
  origin — plain-http LAN origins do not have it, so real-device
  testing needs the dev server over HTTPS (doc 05 §2). Unit tests can
  call it too (Node ≥ 19 provides the same global) or take an injected
  id factory for determinism (doc 05 §1).
- `expected_revision` on state-changing commands is read from
  `gameStore` at send time (doc 03) — never from a stale closure.
- Send while disconnected must not crash: either reject the send and
  surface it through the store's error state, or (better, agreed with
  the team) queue exactly the pending user intent, not a replay log.
  Never leave the UI "silent dead".

## 4. The receive path

1. `onmessage` → `JSON.parse(event.data)` in a **try/catch**; on failure
   drop the frame (log at debug without the raw body). Malformed *client*
   input gets `server.error INVALID_PAYLOAD` from the server, but your
   parse must be defensive anyway.
2. Validate the envelope shape minimally (has `v`, `type`, `payload`) and
   cast to `ServerEnvelope` from `protocol.ts` — one typed parse function,
   e.g. `parseServerEnvelope(raw: string): ServerEnvelope | null`, so
   malformed server data dies in ONE place instead of crashing screens.
3. `switch` on `envelope.type` and route to your stores (doc 03):
   - `room.created` / `room.joined` — store room + identity, write the
     token through the security module (doc 04), resolve the pending
     action's promise so F1 can navigate to `/game/[roomCode]`.
   - `session.ready` — apply `payload.snapshot` as a full snapshot
     (atomic, doc 03 §3).
   - `game.state` — apply `payload` (it IS the `GameSnapshot`) under the
     revision guard (doc 03 §3).
   - `player.presence` — update the player's `connected` flag in
     `roomStore`.
   - `game.move_rejected` — clear `pending`, set the safe error,
     apply `payload.snapshot` if present.
   - `game.finished` — apply the final snapshot + result.
   - `server.error` — map `payload.error` to a friendly themed message
     (screen-facing state, never raw JSON); special-case `INVALID_TOKEN`
     (resume failed — doc 04 §5).
4. The envelope `revision` (present only on `game.state`,
   `game.move_rejected`, `game.finished`) is the input to the atomic
   revision guard.

## 5. Connection lifecycle state machine

```
        connect()                 onopen
 idle ───────────► connecting ──────────► connected
   ▲                    │                    │
   │                    │ onerror/            │ onclose (not user-initiated)
   │                    │ onclose              ▼
   │                    ▼                 reconnecting ──► (backoff) ──► connecting
   └──────── disconnect() ◄──────────── disconnected (user-initiated / gave up)
```

- `connecting` includes the socket handshake; `connected` from `onopen`.
- Any non-user-initiated `onclose` while a room session exists
  → `reconnecting` and schedule the backoff (§6). `disconnected` is
  terminal-ish: user-initiated (`disconnect()`, leave) or backoff
  exhausted/gave up.
- Connection status is **separate** from game status: `disconnected` does
  NOT mean the game is finished (master plan §8, F3 brief §6).
- Status lives in `connectionStore` (doc 03) so any screen can render a
  banner from `useConnection()`.

### The generation counter guard (do not skip this)

The classic reconnect bug: a slow/stale connection delivers an
`onmessage`/`onclose` **after** a new connection took over, wiping or
corrupting newer state. Guard every callback with a generation counter:

```ts
let generation = 0;

function openSocket(): void {
  const gen = ++generation;               // this attempt's id
  const ws = new WebSocket(resolveWsUrl());

  ws.onopen = () => {
    if (gen !== generation) return;      // stale socket — ignore
    /* set connected, reset backoff */
  };
  ws.onmessage = (event) => {
    if (gen !== generation) return;      // stale delivery — drop it
    /* parse + route to stores (§4) */
  };
  ws.onclose = (event) => {
    if (gen !== generation) return;      // an old close after we moved on
    /* decide: reconnect or disconnect, per state machine */
  };
  ws.onerror = () => { if (gen !== generation) return; /* … */ };
  activeSocket = ws;
}
```

Every re-connect increments `generation`; every callback from an older
generation is dropped. Same idea applies to pending action promises: a
`createRoom` promise must reject (not hang forever) if its connection
dies before the reply.

## 6. Reconnect: capped exponential backoff + jitter

- Base delay 0.5 s, doubling per attempt, **capped at 8 s**:
  0.5 s → 1 s → 2 s → 4 s → 8 s → 8 s …
  (`docs/protocol-v1.md` §7 illustrates capped exponential backoff +
  jitter with "e.g. 0.5 s → 30 s"; this client caps tighter, at 8 s, for
  a responsive browser game — within the spec's illustrative range. The
  cap, not its exact value, is what matters.)
- **Full jitter**: sleep `random(0, min(cap, base * 2^attempt))` before
  each retry. Jitter prevents synchronized reconnect stampedes after a
  server restart (100 rooms × clients all retrying on the same tick).
- Reset the backoff to base after a successful `session.ready`
  (or any sustained `connected` period).
- Stop retrying (go `disconnected`) when: the user left the room, the
  room is `finished` and retention ended, or the token is invalid
  (`INVALID_TOKEN` — §7). Never retry `room.leave` closes (§8).
- **Watchdog**: do not wait passively for the OS to notice a dead
  socket. Rely on `onclose`/`onerror` first, but add your own timer — if
  nothing (no frame, no ping window) arrives for ~60 s while `connected`,
  force-close the socket and go to `reconnecting`. Server arithmetic
  (B3 doc 01 §7): server Pings every 30 s and closes after 2 missed
  pongs ⇒ the server-side detector fires within ~60–90 s of silence;
  your client should not be lazier than the server.

### Browser triggers (this replaces mobile background/foreground)

`onclose`/`onerror` is not the only signal — the tab's own lifecycle
gives you more, and each of these can fire while a socket is silently
dead:

| Event | Meaning | Your action |
|---|---|---|
| `window` `online` | the browser regained network | if `reconnecting`: fire the next attempt immediately instead of sleeping out the remaining backoff delay |
| `window` `offline` | network lost | keep the backoff schedule (do NOT hammer retries with no network); the status banner reflects offline; `online` above resumes |
| `document` `visibilitychange` → `visible` | the user came back to the tab | if `reconnecting`: retry now. If `connected`: verify the socket is alive — a tab hidden long enough can hold a socket the network already dropped (the watchdog covers this too) |
| `window` `pageshow` with `event.persisted` | restored from the back/forward cache | do not trust the old socket: verify or close + reconnect + resume (§7) |

Register these listeners on first `connect()` (browser only — the
`typeof window` guard from §1) and remove them in `disconnect()`. Two
facts to internalize:

- Browsers throttle timers in hidden tabs — a `setTimeout` can stretch
  toward once per minute after ~5 minutes hidden (an open WebSocket
  connection generally exempts the page from the harshest throttling in
  Chromium, but do not rely on it). This is acceptable here: your
  backoff caps at 8 s, and when the user returns, `visibilitychange`
  fires an immediate retry. Keep the timers running — pausing retries
  while hidden would burn B2's reconnect grace (~60 s), which is shorter
  than aggressive throttling anyway.
- These events are your safety net for the single-route flow: the game
  lives at `/game/[roomCode]` for the whole session, so a returning user
  must find a live session or a resume already in flight (§7).

## 7. The resume flow (reconnect AND navigation)

On any resume trigger — a reconnect attempt (§6), the tab becoming
visible/online again with a dead socket, or **landing on
`/game/[roomCode]`** — when a token for that room exists in storage:

1. Read the `room_code` (from the URL or the bound session) and the
   `player_token` through the security module (doc 04).
2. Connect if not connected, then send exactly:

```json
{ "v": 1, "id": "<uuid>", "type": "session.resume",
  "payload": { "room_code": "ABCD2345", "player_token": "<from storage>" } }
```

3. `session.ready` (`SessionReadyPayload: { player_id, color, snapshot }`)
   ⇒ **replace the whole local game state** with `payload.snapshot`
   (atomic `set()` in `gameStore`, doc 03 §3), restore identity, mark
   `connected`, clear `pending` (the server is the arbiter of whether
   the pending move landed — it is in the snapshot or it never happened).
   Note: `session.ready` itself carries no revision; per B3 doc 02 §9
   (D5) the server follows it with a `game.state` carrying the envelope
   `revision` — that is where your `expected_revision` comes from.
4. `server.error INVALID_TOKEN` ⇒ the session is gone: **clear the
   stored credentials** (doc 04 §5), set a friendly "session expired"
   state, and let F1 navigate back to home. Do not loop on resume with a
   dead token.
5. `ROOM_NOT_FOUND` on resume (server restarted / room expired): same
   treatment — clear credentials, home screen, friendly message.

### Resume-on-navigation (the key client flow)

The waiting room, the game and the result all live at the single route
**`/game/[roomCode]`** (F4) — so arriving at that route is itself a
resume trigger. The service derives the room code from the URL
(`window.location.pathname`, `/game/<code>` — the same value F4 reads
with `useParams()`) and runs this decision, entirely service-internal:

| Situation on arrival at `/game/[roomCode]` | Action |
|---|---|
| Live bound session for this room (e.g. `room.create`/`room.join` just resolved and F1 navigated here) | **No-op.** Never send a second `session.resume` on a bound connection (§8) |
| No bound session, token for this room in storage | connect + `session.resume` → `session.ready` |
| No token for this room | nothing to resume — F4 renders its "not in this room" state; no error |

Bootstrap points where the service re-checks this decision: its first
evaluation in the browser (fresh load, reload, deep link), `popstate`
(back/forward), the `pageshow` bfcache restore, and the §6 browser
events. Screens never trigger resume — they render the state it
produces (doc 03 §5). Deleting the token on leave/expiry is doc 04 §5.
A stored token for a *different* room is never resumed — only the room
in the URL counts.

## 8. Server behavior your client must match (from B3's docs)

| Server behavior | Your client's obligation |
|---|---|
| Heartbeat: protocol-level Ping every 30 s; closes after 2 unanswered (≈60–90 s) | Browsers answer protocol-level Pings with Pong automatically (RFC 6455); the JS API does not even surface them. Verify once end-to-end (doc 05 §3, S4). No app-level ping/pong exists in v1; never poll. |
| Malformed/binary frame → `server.error INVALID_PAYLOAD`, budget 3, then Close(1008) | Treat `INVALID_PAYLOAD` as a client bug — log it loudly in dev; the close after 3 is expected. |
| `v ≠ 1` → `UNSUPPORTED_VERSION` + close | Should never happen (you always send `PROTOCOL_VERSION`); if it arrives, surface "please update". |
| Auth state machine: a connection binds ONCE (`room.create`/`room.join`/`session.resume`) and in-room commands only work when bound | After `room.leave`, do NOT reuse the socket for a new room — tear down and open a fresh connection (B3 doc 02 §5.1/D3). |
| Accepted `room.leave` ⇒ server **closes the socket** (Close 1000, "left room") | Treat that close as **success**, not a reconnect trigger (D3). |
| Accepted command ⇒ NO ack; authoritative state arrives as broadcast `game.state` | Your pending-move pattern exists for exactly this reason (doc 03 §4). |
| Non-move rejections (`offer_draw`…) arrive as `server.error`; only failed `game.move` gets `game.move_rejected` (to the mover only) | Clear pending for both channels; map both to UI messages. |
| Rate limits: 10 cmds/s burst 20; create/join/resume 1 per 10 s per connection; violations → `server.error RATE_LIMITED` | Never auto-retry in a tight loop; gate buttons on pending state. A backoff retry hitting create/join throttling must not machine-gun. |
| 64 KiB max message/frame; text frames only | Your biggest message is tiny; just never build binary frames. |
| Server-initiated close after `game.finished` retention ends | Expected close → do not reconnect if the room is finished and the user hasn't asked for a rematch. |

## 9. Heartbeat from the client side

- v1 has **no app-level heartbeat messages** — do not invent
  `ping`/`keepalive` command types (that would be a protocol change, doc 01).
- The server sends WebSocket protocol-level Ping frames; the browser
  answers them with Pong automatically (RFC 6455 mandates it, and the
  JS API gives you no way to interfere). Verify once end-to-end in the
  manual checklist (doc 05 §3, S4): keep a tab idle > 90 s and confirm
  the server does NOT drop you.
- Your only client-side heartbeat work is the §6 watchdog (detecting a
  dead socket the OS hasn't noticed) and NOT spamming the server.

## 10. Skeleton

Shape only — the real event routing, backoff, and resume logic is yours.
Keep the responsibilities, design the details as you see fit.

```ts
// src/services/websocket.ts — SKELETON (client-side module, doc 02 §1)
import {
  ClientCommand,
  ClientEnvelope,
  PROTOCOL_VERSION,
  ServerEnvelope,
} from "@/types/protocol";
import { resolveWsUrl } from "@/services/config";

type EventHandler = (envelope: ServerEnvelope) => void;
type ConnectionStatus =
  | "idle" | "connecting" | "connected" | "reconnecting" | "disconnected";

// TODO(F3): generation counter, backoff timer, watchdog, socket ref.
// TODO(F3): route inbound events to stores (doc 03) — or keep stores
//           subscribed via onEvent; pick ONE routing direction and
//           document it.
// TODO(F3): browser triggers — online/offline/visibilitychange/pageshow
//           (§6) and the resume-on-navigation decision (§7); all
//           registered browser-only, removed on teardown.

export const websocketService = {
  connect(url: string = resolveWsUrl()): void {
    // TODO: new WebSocket(url), wire generation-guarded callbacks (§5),
    //       transition "connecting" -> "connected" on open. Never call
    //       during SSR/render (§1).
  },
  send(command: ClientCommand): void {
    // TODO: wrap in ClientEnvelope { v: PROTOCOL_VERSION,
    //       id: crypto.randomUUID(), ...command }, JSON.stringify, send.
    //       Reject/queue while not "connected" (§3).
  },
  onEvent(handler: EventHandler): () => void {
    // TODO: register handler, return unsubscribe fn.
    //       Handlers must be generation-guarded too.
  },
  disconnect(): void {
    // TODO: user-initiated — no reconnect; generation++ so stale
    //       callbacks die; stop timers, remove browser listeners.
  },
  // TODO: resume decision incl. resume-on-navigation (§7),
  //       backoff scheduler with full jitter (§6), watchdog (§6),
  //       parseServerEnvelope(raw): ServerEnvelope | null (§4).
};
```

Manual verification for every behavior in this doc: `05-manual-testing.md`.
