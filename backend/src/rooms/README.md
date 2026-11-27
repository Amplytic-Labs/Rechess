# Module `rooms` — Owner: **B2** (Room Management and Game Lifecycle)

> **Onboarding documentation set for B2.** You implement the room registry and
> the room actors behind the frozen contract. This folder's docs tell you what
> to do, how to do it, and how to test it — in that order. Read them completely
> before writing any code.

---

## 1. Doc set index (read in this order)

| # | File | What it gives you |
|---|---|---|
| — | **`README.md`** (this file) | Mission, boundaries, change rule, definition of done, day-one quickstart. |
| 1 | [`01-contract.md`](01-contract.md) | The FROZEN traits (`RoomRegistry`, `RoomHandle`), every `RoomCommand` / `CommandOutcome` / `RoomEvent`, error→wire-code mapping, the game→protocol snapshot mapping (your integrator job), revision & dedup rules, deadlines. **Your bible for what the outside world expects.** |
| 2 | [`02-architecture.md`](02-architecture.md) | The actor design: one tokio task per room, channel capacities and overflow policy, registry storage choice, room codes and tokens, expiry via tokio time, rematch. |
| 3 | [`03-implementation-guide.md`](03-implementation-guide.md) | Concrete build order with partial skeleton code (registry → actor loop → join/resume → commands → events → expiry) and the pitfalls that bite. |
| 4 | [`04-testing.md`](04-testing.md) | The complete test suite you ship (in-module `#[cfg(test)]` + `backend/tests/rooms_actor.rs`), with full code. |
| 5 | [`05-acceptance.md`](05-acceptance.md) | Definition-of-done checklist mapped to your brief, merge-gate commands, handoff checklist for B3, open decisions. |

Also relevant, owned by others:

* `backend/src/rooms/contract.rs` — the frozen public API you implement (B3 consumes it).
* `backend/src/rooms/testkit.rs` — `FakeRegistry`/`FakeHandle` for B3/B4 (shared infrastructure; NOT yours to rewrite).
* `backend/src/game/contract.rs` + `game/testkit.rs` — B1's engine trait you own inside your actor, and `FakeEngine` for your tests.
* `backend/src/protocol/server_message.rs` — the wire structs you assemble (`GameSnapshot`, `RoomPublicState`, `PlayerPublic`, `MoveRecord`).
* `docs/protocol-v1.md` — the wire spec; **§5 (revision & idempotency semantics) is your bible**.
* `backend/tests/testkit_demo.rs` — working tests against the rooms contract; model your test style on these.

## 2. Mission

Deliver the in-memory multiplayer room system behind the frozen contract:

* a `RoomRegistry` (create / join / resume / lookup by room code),
* one room **actor** per room owning all mutable state (engine, seats, revision, dedup, draw offers, rematch votes, deadlines),
* anonymous player identity with private reconnect tokens (stored hashed),
* room lifecycle: `waiting → active → finished → expired`, with disconnect grace, waiting-room and finished-room expiry,
* rematch by mutual agreement via B1's engine,
* and the **mapping job**: you assemble the wire `protocol::GameSnapshot` from `game::GameSnapshot` + room data. Nobody else does this.

Two authenticated anonymous guests must be able to occupy one room, play a full game through your actor, disconnect and resume within the grace period, finish and rematch — with the actor as the single serialized point of truth.

## 3. Boundaries

**You implement** (inside `backend/src/rooms/` only):

* the `RoomRegistry` + `RoomHandle` traits from `contract.rs` with your real registry and room actors,
* room-code generation, seat allocation, token issuance/verification, revision counting, command-ID dedup,
* disconnect grace, resume, expiry, rematch lifecycle,
* this module's unit tests + `backend/tests/rooms_actor.rs`,
* updates to THIS documentation set as your implementation lands (final names, capacities, decisions).

**You do NOT touch** (changes there require a PR agreed with the owner):

| What | Owner | Why it's frozen/shared |
|---|---|---|
| `rooms/contract.rs` | B2 + B3 (jointly) | B3's network layer is generic over these traits; a signature change breaks B3 and B4's tests. |
| `rooms/testkit.rs` | shared | B3/B4 test against these fakes; it must keep matching `contract.rs`. |
| `game/contract.rs`, `game/testkit.rs`, `game/` impl | B1 (+ B2 for the contract) | You *consume* the engine trait; you never edit the engine. |
| `protocol/` (all wire types) | B3 (+ F3) | You *construct* these structs; renames break the wire format. |
| `network/`, `src/main.rs`, `src/lib.rs` | B3 | They wire your registry in; you expose it, they call it. |
| `frontend/` | F-team | Out of backend scope. |

**Change rule.** Any signature or semantic change to `rooms/contract.rs` (or to the parts of `game/contract.rs` you depend on) is a **PR agreed by B2 + B3** (plus B1 if the engine trait is affected), updating the contract, the testkit, `docs/protocol-v1.md` and `frontend` types in the same reviewed change. Never "just tweak" a frozen signature in your branch.

## 4. Definition of done (summary — full checklist in [`05-acceptance.md`](05-acceptance.md))

A task is done only when its code is merged, its tests pass, its interface is documented, and the next owner (B3) can integrate without reverse-engineering assumptions. Concretely:

- [ ] Real `RoomRegistry`/`RoomHandle` implementation behind the frozen traits; all channels bounded with documented overflow behaviour.
- [ ] Create → White host seat + private token; join → Black guest (first-wins on races); resume by token → full snapshot.
- [ ] Revision counter (monotonic from 0, bumped on every accepted state change) + stale-revision rejection with `STALE_REVISION`.
- [ ] Command-ID dedup window per player (bounded) — *pending the `command_id` contract gap, see [01-contract.md §10](01-contract.md)*.
- [ ] Disconnect grace (60 s) → resume cancels → full snapshot; waiting-room expiry (10 min) and finished-room retention (10 min) close rooms without leaking tasks/channels.
- [ ] Draw offer/accept, resign, rematch-by-both-votes via B1's engine (`reset_for_rematch`).
- [ ] The game→protocol snapshot mapping (`From` impls + `build_snapshot` in your module).
- [ ] The test suite of [04-testing.md](04-testing.md) green: `cargo test rooms::` + `cargo test --test rooms_actor`.
- [ ] This doc set updated with your final type/module names and decisions.

## 5. Testing

Your tests are specified in [04-testing.md](04-testing.md). Two commands gate your module:

```bash
cargo test rooms::            # in-module unit tests (registry, actor, codes, tokens, lifecycle)
cargo test --test rooms_actor  # integration tests through the public traits
```

Today only the fakes' tests exist (`backend/tests/testkit_demo.rs`); both commands match **zero** of your own tests until you write them. The fakes in `testkit.rs` and `game/testkit.rs` are *reference and infrastructure* — B3/B4 test against the fakes, you test your REAL implementation (using B1's `FakeEngine` only as the engine seam inside your actor).

## 6. Day-one quickstart

**Read order (half a day, before any code):**

1. This README, then [01-contract.md](01-contract.md) (every signature, against `contract.rs` itself).
2. `backend/src/game/contract.rs` — the engine you will own inside each actor.
3. [02-architecture.md](02-architecture.md), then [03-implementation-guide.md](03-implementation-guide.md).
4. `docs/protocol-v1.md` §5 (revision & idempotency) and §7 (reconnect flow).
5. `backend/tests/testkit_demo.rs` — the test style that already compiles.
6. [04-testing.md](04-testing.md) and [05-acceptance.md](05-acceptance.md).

**Verify the base builds and is green before you start** (toolchain env required for every cargo command):

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo build && cargo test
```

Expected today: builds clean, **15 tests pass** (4 in `protocol` unit tests, 7 in `tests/protocol_contract.rs`, 4 in `tests/testkit_demo.rs`). If that is not what you see, stop and sync with B4 before building on top of a broken base.

**Then** follow the build order in [03-implementation-guide.md](03-implementation-guide.md) — registry shell first, actor loop second, and so on, with a green checkpoint after each stage.

## 7. Key coordination

* **B1 (daily):** engine factory convention (`new_engine() -> Box<dyn ChessEngine>`), which `GameError` each engine call returns in each edge case (e.g. `accept_draw` with no active offer).
* **B3 (from Day 1):** registry/command/event interfaces, revision semantics, dedup (the `command_id` gap), room-code normalization ownership, capacities and drop policy.
* **B4 (from Day 1):** they will test your races and lifecycle — give them deterministic fixtures and short/deadline knobs early (see `RoomConfig` in [02-architecture.md](02-architecture.md)).
