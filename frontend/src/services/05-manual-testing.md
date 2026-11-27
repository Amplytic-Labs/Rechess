# 05 — Unit tests (vitest) and manual UI testing

> RULES.md §5 (testing strategy, deliberate): **unit tests (vitest, node
> environment) for pure logic** — that is F3's suite, §1 below — and **UI
> is verified manually in the browser** (desktop + mobile viewport),
> §2–§3.
>
> Historical note: the previous revision of this doc set had **no test
> runner** (jest had been dropped because the runner was broken upstream)
> and replaced the brief's "unit tests" deliverable with typecheck +
> manual checklists. That no-runner rule is **replaced**: vitest now owns
> the unit tier, and the brief's unit-test deliverable is back in §1.

## 1. The unit suite (`npm run test`)

`vitest.config.ts` is preconfigured: node environment, tests matched as
`src/**/*.test.{ts,tsx}`, `@` aliased to `src/`. The pattern to extend is
`src/types/protocol.test.ts` — pure logic, no DOM, no sockets, no real
timers. F3's tests land as siblings in your own folders (e.g.
`src/services/websocket.test.ts`, `src/store/game-store.test.ts`) and
must cover, at minimum:

**Codec (parse/serialize):**

- `parseServerEnvelope` accepts each of the 8 event types (build frames
  from `protocol.fixtures.ts`), including the envelope-level `revision`
  on the three stateful events;
- malformed frames return `null` and never throw: invalid JSON, non-object
  JSON, missing `v`/`type`/`payload`, wrong `v`;
- `parseServerEnvelope` rejects a `player_token` smuggled into an event
  that must not carry one (only `room.created`/`room.joined` may);
- the client serializer produces exactly `{ v, id, type, payload }`
  (sorted-keys assertion like `protocol.test.ts`), a fresh `id` per call
  (inject an id factory for determinism), and `promotion: null` always
  present on `game.move`.

**Store reducers:**

- `game.state` under the revision guard: a newer revision applies
  (snapshot + revision in one atomic update), an older/equal revision is
  dropped, a skipped revision (12 → 15) still applies;
- `game.finished`: final snapshot + result applied, `pending` cleared;
- `game.move_rejected`: `pending` cleared, safe error set, optional
  snapshot applied under the same revision guard;
- `session.ready` replaces the whole state unconditionally (the one
  exception to the guard, doc 03 §3);
- `player.presence` flips the player's `connected` flag in `roomStore`
  and nothing else;
- pending-move pattern: `submitMove` sets `pending` without touching
  `snapshot.fen`; duplicate submissions while `pending` are ignored
  (doc 03 §4).

**Duplicate / late events:**

- a duplicate `game.state` (same revision) is a no-op;
- a late `game.state` (older revision arriving after a newer one) is
  dropped;
- a stale socket's callback (generation guard, doc 02 §5) cannot apply
  state after a newer generation took over.

**Reconnect transitions:**

- the backoff schedule as a pure function: attempt 0 → 0.5 s, 1 → 1 s,
  2 → 2 s, 3 → 4 s, 4+ → 8 s (cap), full jitter always inside
  `(0, min(cap, base * 2^attempt))` — inject the RNG for determinism;
  reset to base after `session.ready`;
- stop conditions: user left / room finished / `INVALID_TOKEN` — no more
  attempts (doc 02 §6);
- `INVALID_TOKEN` on resume clears the stored token (doc 04 §5).

Design for testability: pure functions + injected dependencies (id
factory, RNG, clock). Use `vi.useFakeTimers()` for scheduling tests;
never open real sockets in unit tests. If you ever think you need a DOM
(jsdom), that is a team PR — `vitest.config.ts` says so explicitly.

## 2. Testing against the real backend

```bash
cd backend
cargo run                       # serves /health on 0.0.0.0:3000 by default
curl http://localhost:3000/health   # {"service":"rechess-backend","status":"ok"}
```

Frontend: `npm run dev` → **http://localhost:3001**. Same-machine dev
needs **no `.env.local`** (the config module defaults to
`ws://localhost:3000/ws`, doc 02 §2); set `NEXT_PUBLIC_API_URL` only for
non-default setups. For quick wire checks without the app, `websocat`
works:

```bash
websocat ws://localhost:3000/ws
# then paste: {"v":1,"id":"cmd-1","type":"room.create","payload":{"nickname":"Tester"}}
# expected reply: room.created with room_code, player_id, player_token
```

**Two players = two separate storage contexts.** `localStorage` is
per-origin and shared by all tabs of one browser — a second tab on the
same room would replay the same token and resume the same seat. Use a
normal window plus a private/incognito window (or two different
browsers).

**Mobile viewport:** use devtools device emulation on localhost —
responsive-layout checks work there. For a real phone, note the
secure-context caveat (doc 02 §3): `crypto.randomUUID()` exists only on
`localhost` and `https://` origins, so plain-http LAN testing from a
phone fails at the first command — serve the dev server over HTTPS
(e.g. `next dev --experimental-https`) for real-device testing.

## 3. The manual UI scenarios (run all before declaring done)

For each: steps, the store state you expect after each step, and what
the UI shows. Watch state via devtools (React panel, or a temporary
dev-only log); verify F1's waiting room and F4's game screen reflect it.

**S1 — create → join happy path.** Window A (normal): create room with
nickname → `connectionStore.status === "connected"`, `roomStore.roomCode`
set (8 chars), `yourColor === "white"`, `status === "waiting"`, one
player in `players`; the token is in `localStorage` under
`rechess.token.<roomCode>` (devtools → Application → Local Storage).
Window B (private): join with the code → `yourColor === "black"`, two
players, both windows show both players. A's `createRoom` resolved
`{ roomCode, yourColor }`.

**S2 — move + snapshot update.** A (white) plays e2e4 → on A:
`gameStore.pending` set while waiting; on `game.state` (revision 1):
`pending === null`, `snapshot.fen` updated, turn `"black"`,
`move_history` contains `"e4"`, `revision === 1`. B sees the same
snapshot. Then B plays a legal reply; snapshots sync again.

**S3 — move rejected flow.** B (black) tries to move on white's turn
(or an illegal square): `game.move_rejected` → `pending === null`,
`lastError` shows the safe message, board unchanged; then B moves
legally → works (server state not corrupted).

**S4 — heartbeat / idle.** Leave both windows idle > 90 s. Expected:
the server pings every 30 s, the browser auto-pongs (RFC 6455), both
connections stay `connected`. If a window does get dropped, the client
goes `reconnecting` → resumes → `session.ready` snapshot applied.

**S5 — disconnect/reconnect with resume.** With a game in progress:
devtools → Network → Offline on A (or drop the machine's network) ~10 s,
back online → A goes `reconnecting`; the `online` event fires the retry
immediately → `session.ready` with the current snapshot → `connected`,
same room, board identical to B's, `pending` cleared. Within B2's grace
(~60 s) the seat survives. Also: switch to another tab ~30 s and come
back — `visibilitychange` must not produce a duplicate session (the §7
guard in doc 02) and must resume if the socket died silently. Bonus:
stay offline > grace → resume fails with `INVALID_TOKEN`/`ROOM_NOT_FOUND`.

**S6 — stale-token path.** Stop the backend (rooms are in-memory, they
die), restart it, reload A's page at `/game/<roomCode>` → resume fails →
credentials cleared (check Local Storage in devtools), friendly message,
back at home; no reconnect loop.

**S7 — server-down backoff behavior.** Stop the backend while A is
connected. Watch `connectionStore.attempts` climb with delays
0.5 s → 1 s → 2 s → 4 s → 8 s → 8 s (plus jitter; no machine-gun — the
create/join/resume rate limit of 1 per 10 s is never hit). Restart the
backend → A recovers to `connected` and resumes. While down: banner
shown, board interactions disabled, no crash, no rate-limit errors.

**S8 — leave semantics.** Player leaves → server closes the socket
(Close 1000 "left room", B3 doc 02 D3) → the client treats it as
success: no reconnect loop, the room's token is removed from
`localStorage`. A new room needs a NEW connection (verify
create-after-leave works). Also verify the opponent gets
`player.presence` with `connected: false`.

**S9 — resume-on-navigation (the key client flow).** With a game in
progress in A at `/game/<roomCode>`:

1. Reload the page → the service reads the token for the room in the
   URL, sends `session.resume`, applies the `session.ready` snapshot —
   same board, same colors, `connected` again. No "join again" prompt.
2. Open the same URL in a fresh private window (no token there) → no
   resume happens; F4's "not in this room" state renders (and no token
   value leaks into the UI).
3. Navigate A away to home and back (browser back) → the service
   re-runs the decision: already bound → no-op; the room state is
   intact without a second `session.resume`.
4. After S8 (leave), reload → no token in storage → no resume attempt.

**Mobile viewport:** repeat S1–S2 at a phone-sized viewport (devtools
device emulation) — banners, the disabled board and the room-code
display all read and work at narrow widths.

## 4. Dev-only mock strategy (before the backend is up / for F1/F4)

F1/F4 need to develop before your real service works. Build a small
**fake service that implements the same hooks interface**, driven by the
fixtures (`src/types/protocol.fixtures.ts`), and keep it quarantined:

- Lives as one dev-only module inside your ownership (e.g.
  `src/services/mock-websocket.ts`) or as a hook-level fake
  (`useMockRoomActions` etc. implementing the frozen names from doc 03).
- **Behind a flag**, e.g. `NEXT_PUBLIC_USE_MOCK_WS=1` in `.env.local`
  (gitignored) — the real service stays the default. No fixture data in
  the production bundle: gate imports so dead-code elimination can drop
  the mock when the flag is absent, and delete the module before release
  if it isn't needed.
- Clearly marked at the top of the file: dev-only, never ships, no real
  tokens (fixtures use `"secret-token"`).
- Behavior: emit `sampleRoomCreatedEvent` on `createRoom`,
  `sampleSnapshot` on the first `game.state`, advance a mock revision on
  each `submitMove`, emit `sampleFinishedSnapshot` on resign/rematch,
  simulate disconnect after a timeout to let screens test banners.
- Screens import ONLY the frozen hooks, so switching mock ↔ real is one
  flag, zero screen edits. Master plan §16: replace mocks with the real
  server by Day 4 — the mock does not linger.

## 5. Recording results

Note in each PR description: which unit tests were added (and
`npm run test` green), and which scenarios (S1–S9) you ran, in which
browser(s) and at which viewports (desktop + mobile). If a scenario
couldn't be run (e.g. no second storage context), say so — an untested
claim is worse than a listed gap.
