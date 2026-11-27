# 04 — Token storage and security (`src/security/`)

> `player_token` is a **bearer credential**: anyone holding it can resume
> that player's seat (B2 verifies it against a stored hash; the master
> plan §6 issues 32 random bytes). Treat it exactly like a password you
> never show anyone. On the web it lives in `localStorage` — read §3 for
> what that does and does not protect against.

## 1. The rules (binding — RULES.md §3, §4, §6 and `docs/protocol-v1.md` §8.1)

1. The token appears on the wire **only** in the payloads of
   `room.created` and `room.joined` (sent once, to the initiating
   connection) and `session.resume` (you sending it back). Never in
   broadcasts — `PlayerPublic`/`GameSnapshot` never contain it.
2. On the client it lives **only** in `localStorage`, written by the one
   module in `src/security/`. Never a URL or query param, never
   `document.cookie`, never a log line, never an error report, never a
   zustand store.
3. `src/screens/**` never sees the token. F1/F4 call your hooks; the
   token stays inside `src/services/` + `src/security/` (their READMEs
   explicitly promise this: *"Never receive or touch `player_token`"*).

## 2. The wrapper (`src/security/player-token.ts`)

One small module, the ONLY code that touches `localStorage` for the
token. One key per room, so several rooms' tokens never clobber each
other, and so the resume-on-navigation flow (doc 02 §7) can look up
exactly the room in the URL:

```ts
// src/security/player-token.ts — SKELETON (browser only; never during
// SSR/render — the service calls it, doc 02 §1)
const keyFor = (roomCode: string) => `rechess.token.${roomCode}`;

export function saveToken(roomCode: string, token: string): void {
  // TODO: window.localStorage.setItem(
  //   keyFor(roomCode), JSON.stringify({ token }));
}

export function loadToken(roomCode: string): string | null {
  // TODO: read the key; JSON.parse inside try/catch; on ANY failure
  //       (missing key, parse error, wrong shape) clear the bad entry
  //       and return null — a corrupted value must never reach the
  //       resume path, and must never throw.
}

export function clearToken(roomCode: string): void {
  // TODO: window.localStorage.removeItem(keyFor(roomCode));
}
```

- **JSON.parse-guarded readers**: the stored value is JSON-encoded and
  every read goes through `JSON.parse` in a `try/catch` plus a shape
  check. Anything unexpected is treated as "no session" and cleared —
  never thrown, never used.
- `localStorage` is synchronous, so no async wrapper is needed — but
  keep the module: the storage location stays in ONE place, and the
  room-code-to-key mapping lives next to it.
- The room code is not a secret (it is in the URL and on screen) — it
  is part of the key, not a second credential.
- Browser only: this module is imported from client-side code and must
  never run during the server pre-render (the `typeof window` guard,
  doc 02 §1). Reading storage during render would also break hydration —
  the service reads it in effects/handlers, never in a component body.

## 3. The tradeoff, stated honestly

`localStorage` is per-origin browser storage, and **any JavaScript
running on the page can read it**. Do not paper over what that means:

- **XSS reads the token.** If any script on the origin is compromised
  (a dependency, an injection point), it can read
  `rechess.token.<roomCode>` and resume that seat. There is no browser
  mechanism that hides a value from same-origin JS while still letting
  your own code use it on demand.
- **No httpOnly option without backend changes.** An `httpOnly` cookie
  is withheld from JS by the browser — but it must be *issued by the
  server*, and a WebSocket message cannot set one. Making the session
  cookie-based would require the backend to issue an `httpOnly` cookie
  at room create/join and accept it on resume: a protocol + backend
  change, out of scope for v1.
- **Why it is accepted for the MVP:** there are no accounts and no PII;
  a token's blast radius is one anonymous seat for the remaining
  lifetime of one room (reconnect grace ~60 s; finished-room retention
  ~10 min; a server restart kills all rooms). The worst outcome of a
  leaked token is losing one seat of one anonymous game.

Mitigations that stay (each one shrinks the leak or the XSS surface):

- The token never goes in a URL, a cookie, or a log (§1; RULES.md §3/§6).
- Only the security module reads/writes it; screens and stores never
  see it.
- JSON.parse-guarded readers: corrupted or planted values die in the
  wrapper, never in the resume path.
- The app renders no raw HTML (no `dangerouslySetInnerHTML`) and React
  escapes everything else — the practical XSS surface is dependencies,
  which are pinned in `package-lock.json` (RULES.md §2 makes adding one
  a team PR).
- **Future hardening (a team PR, not this MVP):** a server-issued
  `httpOnly` session cookie at create/join, with the resume
  authenticated by that cookie — removes the token from JS-readable
  storage entirely. It needs B2/B3 + protocol changes; propose it, do
  not prototype it silently.

## 4. Write-once semantics

- **Write:** exactly when `room.created` / `room.joined` arrives (the
  only two events that ever carry a token). One write per session.
- **Read:** only when building a `session.resume` (doc 02 §7).
- **No refresh, no rotation:** v1 has no token renewal. Whatever the
  server sent at create/join is the token until the room dies.

## 5. Deletion — when and why

| Trigger | Action |
|---|---|
| Player explicitly leaves (`leaveRoom` action → `room.leave` accepted) | `clearToken(roomCode)` — the seat is gone, the token is worthless, and a stale token causes an `INVALID_TOKEN` loop on the next resume |
| `server.error INVALID_TOKEN` on resume | `clearToken(roomCode)`, set "session expired" state, back to home (F1 navigates) |
| `ROOM_NOT_FOUND` on resume (server restarted — rooms are in-memory, master plan §10 — or room expired) | `clearToken(roomCode)`, friendly "room no longer exists", back to home |
| Room reaches end of retention (B2 deletes finished rooms after ~10 min) | Nothing proactive — the token is simply invalid server-side; the next resume fails with `ROOM_NOT_FOUND`/`INVALID_TOKEN` and hits the rows above |
| User clears site data / browser storage | Gone — nothing for us to do (the web equivalent of uninstalling the app) |

## 6. What the token authorizes (mental model)

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

## 7. Logging rules (RULES.md §6 — "never log `player_token`")

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
