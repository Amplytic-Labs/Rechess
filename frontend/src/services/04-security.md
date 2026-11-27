# 04 — Token security (`src/security/`)

> `player_token` is a **bearer credential**: anyone holding it can resume
> that player's seat (B2 verifies it against a stored hash; the master
> plan §6 issues 32 random bytes). Treat it exactly like a password you
> never show anyone.

## 1. The rules (binding — RULES.md §3, §6 and `docs/protocol-v1.md` §8.1)

1. The token appears on the wire **only** in the payloads of
   `room.created` and `room.joined` (sent once, to the initiating
   connection) and `session.resume` (you sending it back). Never in
   broadcasts — `PlayerPublic`/`GameSnapshot` never contain it.
2. On-device it lives **only** in `expo-secure-store`. Never
   AsyncStorage, never a plain object persisted anywhere, never a route
   param, never a deep link, never a log line, never an error report.
3. `src/screens/**` never sees the token. F1/F4 call your hooks; the
   token stays inside `src/services/` + `src/security/` (their READMEs
   explicitly promise this: *"Never receive or touch `player_token`"*).

## 2. The wrapper (`src/security/`)

One small module, the ONLY code that touches SecureStore for the token:

```ts
// src/security/player-token.ts — SKELETON
import * as SecureStore from "expo-secure-store";

const KEY = "rechess.player_token";      // one named key, never "token"
const ROOM_KEY = "rechess.room_code";    // non-secret, but resume pairs them

export async function saveCredentials(token: string, roomCode: string): Promise<void> {
  // TODO: SecureStore.setItem(KEY, token); SecureStore.setItem(ROOM_KEY, roomCode);
}

export async function loadCredentials(): Promise<{ token: string; roomCode: string } | null> {
  // TODO: SecureStore.getItem both; if either is missing, return null
  //       (treat as "no session").
}

export async function clearCredentials(): Promise<void> {
  // TODO: SecureStore.deleteItem both.
}
```

- `expo-secure-store` is already installed (`~58.0.3`) and
  `expo-secure-store` is already registered as a plugin in `app.json` —
  no config work needed.
- The API is async — the service must `await` it (never fire-and-forget
  a token write; a crash between `room.created` arriving and the write
  completing means an unresumable seat, which is acceptable, but a lost
  write you never noticed is not).
- The room code is not a secret (it's in route params and on screen) —
  storing it beside the token is a convenience so resume has both halves
  of the credential pair. If you prefer it in `roomStore`, fine — just
  keep the token's location unchanged.

## 3. Write-once semantics

- **Write:** exactly when `room.created` / `room.joined` arrives (the
  only two events that ever carry a token). One write per session.
- **Read:** only when building a `session.resume` (doc 02 §7).
- **No refresh, no rotation:** v1 has no token renewal. Whatever the
  server sent at create/join is the token until the room dies.

## 4. Deletion — when and why

| Trigger | Action |
|---|---|
| Player explicitly leaves (`leaveRoom` action → `room.leave` accepted) | `clearCredentials()` — the seat is gone, the token is worthless, and a stale token causes an `INVALID_TOKEN` loop on the next resume |
| `server.error INVALID_TOKEN` on resume | `clearCredentials()`, set "session expired" state, back to home (F1 navigates) |
| `ROOM_NOT_FOUND` on resume (server restarted — rooms are in-memory, master plan §10 — or room expired) | `clearCredentials()`, friendly "room no longer exists", back to home |
| Room reaches end of retention (B2 deletes finished rooms after ~10 min) | Nothing to do — the token is simply invalid server-side; the next resume fails with `ROOM_NOT_FOUND`/`INVALID_TOKEN` and hits the rows above |
| App uninstall | OS wipes SecureStore — nothing to do |

## 5. What the token authorizes (mental model)

- **Seat resume only.** In-room commands (`game.move`, …) are authorized
  by the *connection binding* (B3 doc 02 §5.2): the token proves who you
  are exactly once, at `session.resume`; after that, the socket is the
  authorization. Your client correspondingly never sends the token with
  in-room commands — `ClientCommand` has no token field on them.
- **Bearer semantics:** no player_id, no password, nothing else needed —
  possession is authorization. A leaked token = stolen seat (until the
  room ends).
- **Lifetime:** the token is valid only while the room lives
  server-side (B2: reconnect grace ~60 s; finished-room retention
  ~10 min; server restart kills all rooms). It is not a permanent
  identity — there are no accounts (master plan §3.1).
- Room codes are identifiers, not credentials — do not "protect" them
  like the token; they're meant to be shared with exactly one friend.

## 6. Logging rules (RULES.md §6 — "never log `player_token`")

- Never `console.log` the token, the `room.created`/`room.joined`
  payloads, or the `session.resume` payload — including inside caught
  errors and React error boundaries.
- If you log an envelope for debugging, log the **type and command id
  only**, and strip payloads for the three token-bearing paths. A
  `console.log(JSON.stringify(envelope))` anywhere in the receive path
  WILL eventually capture a token — grep your code for
  `JSON.stringify` before every PR.
- Error messages surfaced to screens use `ServerErrorPayload.message`
  (the spec guarantees they are display-safe), never raw payloads.
