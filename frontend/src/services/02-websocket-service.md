# 02 — The WebSocket service (`src/services/websocket.ts`)

> One connection per app. It is a **module-level singleton service, not a
> hook** — hooks (doc 03) wrap it; screens never see it. B3's plan
> (`backend/src/network/01-axum-websocket-guide.md`, `02-protocol-implementation.md`)
> is the server side of everything here; your client must match it.

## 1. The base that already exists (don't rebuild it)

- No socket library needed: React Native ships a global `WebSocket`
  class, callback-based (`onopen`, `onmessage`, `onclose`, `onerror`,
  `close()`, `send(string)`). `zustand ~5.1.0`, `expo-secure-store ~58.0.3`,
  `expo-crypto ~58.0.9`, `expo-constants ~58.0.16` are installed and
  SDK-matched (verify in `frontend/package.json` before adding anything).
- The wire types are done: `src/types/protocol.ts` (doc 01).

## 2. Connection URL derivation

Single endpoint: `GET /ws` — `ws://<host>/ws` in dev, `wss://<host>/ws` in
production (TLS at the reverse proxy). The server binds `0.0.0.0:3000` by
default (`RECHESS_HOST`/`PORT` env vars, `backend/src/main.rs`).

Use the `EXPO_PUBLIC_` env mechanism: variables prefixed `EXPO_PUBLIC_`
in `.env` are **inlined into the bundle at build/start time** and read
from `process.env` in client code. (If the team ever prefers config-file
values, `expo-constants` — `Constants.expoConfig.extra` — is the
alternative; `process.env.EXPO_PUBLIC_*` is the standard Expo way and
needs no code changes.)

```bash
# frontend/.env  — NEVER commit it (check .gitignore; if it's missing, PR)
EXPO_PUBLIC_API_URL=http://192.168.1.20:3000   # your machine's LAN IP in dev
# production build: EXPO_PUBLIC_API_URL=https://chess.example.com
```

Derive the WS URL from it (never hardcode an IP in code):

```ts
// sketch
function resolveWsUrl(): string {
  const base = process.env.EXPO_PUBLIC_API_URL; // e.g. "http://192.168.1.20:3000"
  if (!base) throw new Error("EXPO_PUBLIC_API_URL is not set — create frontend/.env");
  const withWs = base.replace(/^http:/, "ws:").replace(/^https:/, "wss:");
  return `${withWs.replace(/\/$/, "")}/ws`;
}
```

Gotchas worth knowing now:

- Inlined **at bundle time**: editing `.env` requires restarting
  `expo start` (and a reload of the app) — it is not runtime config.
- `localhost` from a phone means the phone itself. Use your LAN IP, and
  make sure the phone and the dev machine are on the same network
  (B4's runbook documents this too).
- Production must be `https://` → `wss://`. Plain `ws://` to a public
  host is a security bug (RULES.md §4, master plan §6).

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
- `id` is a fresh **UUID per command** from `expo-crypto`:
  `import * as Crypto from "expo-crypto"; Crypto.randomUUID()`. The server
  logs it and B2 plans dedup around it — never reuse an id, never send
  without one.
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
     token to SecureStore (doc 04), resolve the pending action's promise
     so F1 can navigate.
   - `session.ready` — apply `payload.snapshot` as a full snapshot
     (atomic, doc 03 §3).
   - `game.state` — apply `payload` (it IS the `GameSnapshot`) under the
     revision guard (doc 03 §3).
   - `player.presence` — update the player's `connected` flag in
     `roomStore`.
   - `game.move_rejected` — clear `pendingMove`, set the safe error,
     apply `payload.snapshot` if present.
   - `game.finished` — apply the final snapshot + result.
   - `server.error` — map `payload.error` to a friendly themed message
     (screen-facing state, never raw JSON); special-case `INVALID_TOKEN`
     (resume failed — doc 04 §4).
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
- Any non-user-initiated `onclose`/`onerror` while a room session exists
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

## 6. Reconnect: exponential backoff + jitter

- Base delay 1 s, doubling per attempt, **capped at 30 s**:
  1 s → 2 s → 4 s → 8 s → 16 s → 30 s → 30 s …
  (`docs/protocol-v1.md` §7 suggests 0.5 s → 30 s; 1 s base is fine —
  the cap is what matters).
- **Full jitter**: sleep `random(0, min(cap, base * 2^attempt))` before
  each retry. Jitter prevents synchronized reconnect stampedes after a
  server restart (100 rooms × clients all retrying on the same tick).
- Reset the backoff to base after a successful `session.ready`
  (or any sustained `connected` period).
- Stop retrying (go `disconnected`) when: the user left the room, the
  room is `finished` and retention ended, or the token is invalid
  (`INVALID_TOKEN` — §7). Never retry `room.leave` closes (§8).
- **Watchdog**: do not wait passively for the OS to notice a dead NAT'd
  socket. Rely on `onclose`/`onerror` first, but add your own timer — if
  nothing (no frame, no ping window) arrives for ~60 s while `connected`,
  force-close the socket and go to `reconnecting`. Server arithmetic
  (B3 doc 01 §7): server Pings every 30 s and closes after 2 missed
  pongs ⇒ the server-side detector fires within ~60–90 s of silence;
  your client should not be lazier than the server.

## 7. The resume flow

On reconnect (or app foreground via `AppState` → `"active"`) when the app
was in a room:

1. Read `room_code` + `player_token` from SecureStore (doc 04).
2. Send exactly:

```json
{ "v": 1, "id": "<uuid>", "type": "session.resume",
  "payload": { "room_code": "ABCD2345", "player_token": "<from SecureStore>" } }
```

3. `session.ready` (`SessionReadyPayload: { player_id, color, snapshot }`)
   ⇒ **replace the whole local game state** with `payload.snapshot`
   (atomic `set()` in `gameStore`, doc 03 §3), restore identity, mark
   `connected`, clear `pendingMove` (the server is the arbiter of whether
   the pending move landed — it is in the snapshot or it never happened).
   Note: `session.ready` itself carries no revision; per B3 doc 02 §9
   (D5) the server follows it with a `game.state` carrying the envelope
   `revision` — that is where your `expected_revision` comes from.
4. `server.error INVALID_TOKEN` ⇒ the session is gone: **clear the
   stored credentials** (doc 04 §4), set a friendly "session expired"
   state, and let F1 navigate back to home. Do not loop on resume with a
   dead token.
5. `ROOM_NOT_FOUND` on resume (server restarted / room expired): same
   treatment — clear credentials, home screen, friendly message.

## 8. Server behavior your client must match (from B3's docs)

| Server behavior | Your client's obligation |
|---|---|
| Heartbeat: protocol-level Ping every 30 s; closes after 2 unanswered (≈60–90 s) | RN WebSocket is expected to auto-pong protocol pings — **verify on a real device** (§9). No app-level ping/pong exists in v1; never poll. |
| Malformed/binary frame → `server.error INVALID_PAYLOAD`, budget 3, then Close(1008) | Treat `INVALID_PAYLOAD` as a client bug — log it loudly in dev; the close after 3 is expected. |
| `v ≠ 1` → `UNSUPPORTED_VERSION` + close | Should never happen (you always send `PROTOCOL_VERSION`); if it arrives, surface "please update". |
| Auth state machine: a connection binds ONCE (`room.create`/`room.join`/`session.resume`) and in-room commands only work when bound | After `room.leave`, do NOT reuse the socket for a new room — tear down and open a fresh connection (B3 doc 02 §5.1/D3). |
| Accepted `room.leave` ⇒ server **closes the socket** (Close 1000, "left room") | Treat that close as **success**, not a reconnect trigger (D3). |
| Accepted command ⇒ NO ack; authoritative state arrives as broadcast `game.state` | Your pending-move pattern exists for exactly this reason (doc 03 §4). |
| Non-move rejections (`offer_draw`…) arrive as `server.error`; only failed `game.move` gets `game.move_rejected` (to the mover only) | Clear pending for both channels; map both to UI messages. |
| Rate limits: 10 cmds/s burst 20; create/join/resume 1 per 10 s per connection; violations → `server.error RATE_LIMITED` | Never auto-retry in a tight loop; gate buttons on pending state. A backoff retry hitting create/join throttling must not machine-gun. |
| 64 KiB max message/frame; text frames only | Your biggest message is tiny; just never build binary frames. |
| Server-initiated close after `game.finished` retention ends | Expected close → do not reconnect if the room is finished and the user hasn't asked for a rematch.

## 9. Heartbeat from the client side

- v1 has **no app-level heartbeat messages** — do not invent
  `ping`/`keepalive` command types (that would be a protocol change, doc 01).
- The server sends WebSocket protocol-level Ping frames; the native
  WebSocket layer is generally expected to answer with Pong
  automatically. **Do not assume — verify on a real device** (the
  manual checklist in doc 05 §3, scenario 4): connect, keep the app idle
  > 90 s, confirm the server does NOT drop you (i.e., pongs flow without
  your code doing anything). If a platform ever fails to auto-pong,
  surface it to the team; only a server-side policy change fixes it.
- Your only client-side heartbeat work is the §6 watchdog (detecting a
  dead socket the OS hasn't noticed) and NOT spamming the server.

## 10. Skeleton

Shape only — the real event routing, backoff, and resume logic is yours.
Keep the responsibilities, design the details as you see fit.

```ts
// src/services/websocket.ts — SKELETON
import * as Crypto from "expo-crypto";
import {
  ClientCommand,
  ClientEnvelope,
  PROTOCOL_VERSION,
  ServerEnvelope,
  ServerEvent,
} from "@/types/protocol";

type EventHandler = (envelope: ServerEnvelope) => void;
type ConnectionStatus = "idle" | "connecting" | "connected" | "reconnecting" | "disconnected";

// TODO(F3): generation counter, backoff timer, watchdog, socket ref.
// TODO(F3): route inbound events to stores (doc 03) — or keep stores
//           subscribed via onEvent; pick ONE routing direction and
//           document it.

export const websocketService = {
  connect(url: string): void {
    // TODO: open socket, wire guarded callbacks (§5), transition to
    //       "connecting" -> "connected" on open.
  },
  send(command: ClientCommand): void {
    // TODO: wrap in ClientEnvelope { v: PROTOCOL_VERSION,
    //       id: Crypto.randomUUID(), ...command }, JSON.stringify, send.
    //       Reject/queue while not "connected" (§3).
  },
  onEvent(handler: EventHandler): () => void {
    // TODO: register handler, return unsubscribe fn.
    //       Handlers must be generation-guarded too.
  },
  disconnect(): void {
    // TODO: user-initiated — no reconnect; generation++ so stale
    //       callbacks die; stop timers.
  },
  // TODO: resume flow (§7), backoff scheduler (§6), watchdog (§6),
  //       parseServerEnvelope(raw): ServerEnvelope | null (§4).
};
```

Manual verification for every behavior in this doc: `05-manual-testing.md`.
