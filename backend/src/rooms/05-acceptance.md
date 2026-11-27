# 05 — Acceptance, Merge Gate, and B3 Handoff

> The definition of done for your module, the commands B4 runs on your PRs,
> and the exact checklist for handing the registry to B3. The open
> decisions table at the bottom is your meeting agenda with B1/B3 — nothing
> in it blocks starting Stage 1 of
> [03-implementation-guide.md](03-implementation-guide.md).

## 1. Definition of done — every brief requirement, mapped

Your brief's acceptance statement: *"Two authenticated anonymous guests can
occupy one room, play a game through the actor, disconnect and resume within
the grace period, and finish/rematch. Race tests pass and no other module can
directly mutate room state."*

| Brief requirement (§3 / §8) | Where it lands | Proven by |
|---|---|---|
| Crypto-secure room codes, unambiguous alphabet, collision check vs active rooms | `registry.rs` `generate_room_code`/`fresh_room_code` ([02 §7](02-architecture.md)) | `room_codes_are_8_chars_from_the_unambiguous_alphabet` |
| Create: host White, private credential via direct response | `registry.rs` `create_room` ([01 §2](01-contract.md)) | `create_seats_host_white_in_a_waiting_room` |
| Join only a `waiting` room; stable typed errors for malformed/full/active/expired | actor `Join` handler ([02 §3](02-architecture.md)) | `join_seats_guest_black…`, `join_unknown_code…`, `second_join_into_a_full_room_is_rejected` |
| Exactly one of two simultaneous joins wins | actor serialization — no extra code | B4 concurrency suite (passes by construction) |
| Players separate from public data; token stored hashed, never in broadcasts/logs | `Seat.token_verifier` ([02 §8](02-architecture.md)) | resume tests + code review (grep for the token in event types — structurally impossible) |
| One tokio task per room, sequential commands | `room_actor.rs` ([02 §1](02-architecture.md)) | every integration test |
| Engine integration + seat authorization before engine calls | `handle_move` et al. ([03 §5](03-implementation-guide.md)) | `a_player_not_in_the_room_cannot_act`, `engine_rejection_targets_only_the_mover` |
| Monotonic revision; reject stale `expected_revision` | actor revision handling ([01 §9](01-contract.md)) | `accepted_move_bumps_revision…`, `stale_expected_revision_is_rejected…` |
| Bounded per-player command-ID dedup window | `BoundedDedupCache` ([01 §10](01-contract.md)) | `dedup_window_is_bounded_and_recent` (pending the `command_id` PR) |
| Disconnect → grace deadline; resume cancels it and returns full snapshot | `lifecycle.rs` ([02 §9](02-architecture.md)) | `disconnect_broadcasts_presence_and_grace_expires_to_a_forfeit`, `resume_with_the_right_token…` |
| Status transitions waiting/active/finished/expired + retention expiry | `lifecycle.rs` + actor shutdown | grace/leave tests; B4 expiry/no-leak suite |
| Rematch: both agree; reset board/history/draw state; revision-safe identity | rematch handler ([02 §10](02-architecture.md)) | `rematch_needs_both_votes_and_resets_the_engine` |
| Bounded channels everywhere; defined queue-full behaviour; no locks across awaits | capacities 32/64, `try_send` broadcast, std-Mutex discipline ([02 §4–6](02-architecture.md)) | B4 backpressure suite + `clippy::await_holding_lock` gate |
| Wrong-room/wrong-seat token cannot move or resume | actor authz + `resume_room` | `a_player_not_in_the_room_cannot_act`, `resume_with_a_wrong_token…` |
| Room isolation (activity in A ≠ B) | one actor per room, per-room engine | B4 multi-room suite (use a fresh engine per room there — [04 §1.4](04-testing.md)) |
| Expiry removes room + actor without leaking task/channel resources | shutdown/drain pattern ([03 §6](03-implementation-guide.md)) | `leave_closes_the_room_and_the_actor`; B4 leak test |
| Server restart loses all rooms (documented MVP limitation) | this doc set + logs | documented in [02 §11](02-architecture.md) and below |
| Mapping `game` → `protocol` (GameSnapshot assembly) | `mapping.rs` ([01 §7](01-contract.md)) | `worked_example_after_e4_maps_to_the_wire_fixture` |

**Done means all of the above are merged, green, and this doc set updated
with your final type names.**

## 2. Merge gate (what B4 runs on every rooms PR)

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

cargo fmt --check                 # team rule: fmt clean
cargo clippy -- -D warnings       # includes await_holding_lock — keep it on
cargo test                        # full suite: the 15 pre-existing tests + yours
cargo test rooms::                # your in-module unit tests
cargo test --test rooms_actor     # your integration tests
```

Plus B4's own suites once your registry is importable: two-simultaneous-joins,
concurrent-moves serialization, 50-room isolation, backpressure, expiry
without leaks. When B4 reports a failure there, the fixtures are yours to fix
— deterministic knobs come from `RoomConfig`
([03 §3](03-implementation-guide.md)).

## 3. Handoff checklist for B3

When your registry is real, B3 links it in `src/network/` + `main.rs`. Give
them exactly this:

- [ ] **Construction snippet** (final names from your module):

  ```rust
  use rechess_backend::rooms::config::RoomConfig;
  use rechess_backend::rooms::registry::{EngineFactory, InMemoryRegistry};
  use std::sync::Arc;

  // main.rs / network setup:
  let new_engine: EngineFactory = Arc::new(|| Box::new(<B1sEngine>::new_game())); // B1's constructor
  let registry = Arc::new(InMemoryRegistry::new(RoomConfig::default(), new_engine));
  // B3 puts it in axum state and is generic over RoomRegistry — never naming
  // InMemoryRegistry outside wiring.
  ```

- [ ] **Generic consumption pattern** — already proven against the fakes in
  `tests/testkit_demo.rs::generic_code_works_against_any_registry`; your
  registry drops into the same code unchanged.
- [ ] **Per-connection model** (B3's side, so you build compatible
  semantics): after `create_room`/`join_room`/`resume_room`, the connection
  task stores `handle + player_id`; every command becomes a `RoomCommand`
  with that `player`; tokens are presented **only** on those three
  handshake operations (see open decision #2 below).
- [ ] **Event routing**: one `subscribe()` per connection; forward events
  where `target.is_none() || target == Some(my_player_id)`
  ([01 §6](01-contract.md)).
- [ ] **Error mapping tables** they rely on: [01 §2.1](01-contract.md) (registry errors → wire codes), §6.1/§6.2 (rejections/engine errors → codes).
- [ ] **Capacities & overflow policy on record**: command channel 32
  (senders await → backpressure), event queues 64 per subscriber (full →
  event dropped + warn; closed → pruned) — [02 §4–5](02-architecture.md).
- [ ] **Room death**: `send` → `Err(RoomClosed)` and subscribers' `recv()`
  → `None` are B3's signal to close the socket gracefully (no `room.closed`
  wire event exists — open decision #10).
- [ ] **What B3 must never do** (enforce in review): construct or touch
  `RoomState`/the engine, hold a registry guard across an await, call
  `subscribe` more than once per connection expecting ordering, or log
  `player_token`.
- [ ] **A "works against my registry" smoke test** you hand B3: one file,
  two connections, one move — the vertical-slice seed for Day 4.

## 4. Open decisions to settle (B2 drives each; none blocks Stage 1)

| # | Question | Recommendation on record | Who decides |
|---|---|---|---|
| 1 | **Dedup `command_id` is missing from the frozen `RoomCommand`** — protocol §5 requires per-player ID dedup, the enum carries no ID | PR adding `command_id: String` to the 5 state-changing variants; dedup in the actor | B2+B3 (contract PR) |
| 2 | **Per-command authentication model**: `RoomCommand` carries only `player`, so identity is connection-scoped (bound at create/join/resume by B3) — is that the agreed model, or should commands carry tokens? | Connection-scoped binding; actor still rejects unknown players with `InvalidToken` | B2+B3 |
| 3 | **Room-code normalization ownership**: `contract.rs` says B3 normalizes (upper/trim); `protocol/client_message.rs` comment says "B2 normalizes"; protocol-v1.md §3 says "server uppercases" | B3 normalizes at the gate (matches the frozen contract); B2 defensively validates 8-char/alphabet | B2+B3 |
| 4 | **Does `join_room` bump the revision** (→ 1) and broadcast the first `game.state`? | Yes — host's "game starts now" signal; every later snapshot is strictly newer | B2+B3 |
| 5 | **First rematch vote emits nothing** — `GameSnapshot` has no rematch-votes field, so F4 cannot show "opponent wants a rematch" | Keep no-broadcast for v1; a `room.rematch_vote` event is a clean protocol-v1.1 extension | B2+B3+F4 |
| 6 | **Grace-expiry result**: forfeit surfaces as `resignation` — no `timeout` reason exists in v1 | Forfeit as `Resignation` now; propose `TIMEOUT` reason in a protocol PR | B2+B3 |
| 7 | **Move in a still-`waiting` room** → which code? | `NotYourTurn` + "game has not started yet"; a `GAME_NOT_STARTED` code is a v1.1 candidate | B2+B3 |
| 8 | **`accept_draw` with no active offer** — which `GameError` does B1's engine return? (contract doesn't enumerate it) | Agree it is `IllegalMove`; add to the B1+B2 interface agreement | B1+B2 |
| 9 | **Token hashing dependency**: no crypto crate is pinned | PR adding `sha2 = "0.10"`; interim (documented) plaintext-in-memory only if the PR is blocked | B2 (PR), B4 (review) |
| 10 | **Room-closed wire behaviour**: no `room.closed` event exists; B3 must infer from `RoomClosed`/closed channels and B3/F4 decide the UX | Keep socket-close + client "room is gone" handling in v1 | B3+F3 |
| 11 | **Engine factory convention** with B1: constructor name | `fn new_game() -> Self` on B1's engine type; B2's factory `Arc<dyn Fn() -> Box<dyn ChessEngine> + Send + Sync>` ([03 §4](03-implementation-guide.md)) | B1+B2 |
| 12 | **`MoveRejected.snapshot`**: always `Some(current)`? | Yes — heals a stale mover without a resume (the fake demo's `None` is just a wire-shape example) | B2+B3 |
| 13 | **Tombstone retention** for `JoinRoomError::Expired` vs `NotFound` | Bounded (10 min, `RoomConfig`) ring of expired codes | B2 (B4 review) |

## 5. Known limitations to keep documented (not bugs)

* **Server restart loses every room** (in-memory MVP). Never imply
  persistence in logs, events or docs; B4 records it in the deployment
  runbook. Multi-instance scaling is explicitly out of scope for v1.
* Rate limiting create/join/resume (master plan §6) is B3's gate concern —
  you provide the hooks (cheap `lookup`, no locks) but no limiter.
* 50-room benchmarking is B4's; your part is only that per-room cost stays
  flat (no per-room background tasks beyond the actor itself, no polling).

## 6. Milestones recap (brief §9 → [03 §2](03-implementation-guide.md) stages)

| Brief milestone | Your stages |
|---|---|
| Day 1: state sketch, API + B1/B3 agreement | Doc set + open decisions table above |
| Day 2: actor skeleton, create/join, channels | Stages 1–2 |
| Days 3–4: first move through B1's engine | Stages 3–4 (vertical slice with B3+F3 on Day 4) |
| Days 5–6: lifecycle, draw/resign/rematch, expiry | Stage 5 + first half of 6 |
| Days 7–8: reconnect, stale handling, backpressure | Stage 6 + the `command_id` PR |
| Days 9–10: tests, race fixes, B4 support | Full gate green, handoff §3 complete |
