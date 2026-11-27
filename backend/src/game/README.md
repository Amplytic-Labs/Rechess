# Module `game` — onboarding doc set for **B1** (Chess Engine and Game Logic)

> **Read this file completely before writing any code.**
> It tells you what you are expected to do, how to do it, what the result must
> be, and exactly where the boundary to your teammates' modules is — so that
> nobody steps on anyone else's work.

---

## 1. Your mission

Deliver a small, well-tested Rust module that takes a move request (or a
lifecycle action) and either **rejects it with a typed error** or **returns the
updated authoritative game snapshot**. The server is the only authority on
rules — the frontend never decides whether a move is legal.

You are **not** building WebSockets, rooms, or anything network-related. Those
belong to B2 (rooms) and B3 (network). Your module must be usable by B2 with
no dependency on Axum, sockets, or frontend code.

The public API you must implement is **already frozen** in
[`contract.rs`](contract.rs) — read [`01-contract.md`](01-contract.md) for the
piece-by-piece explanation, [`02-implementation-guide.md`](02-implementation-guide.md)
for the how (shakmaty 0.30.3), [`03-testing.md`](03-testing.md) for the test
suite you ship, and [`04-acceptance.md`](04-acceptance.md) for the definition
of done and the handoff to B2.

## 2. Doc set index

| File | What it is |
|---|---|
| [`README.md`](README.md) | This file: mission, boundaries, change rule, day-one quickstart. |
| [`01-contract.md`](01-contract.md) | The frozen contract explained: every type in `contract.rs`, every trait method's exact behavior (validation order, edge cases), every `GameError`, how B2 consumes the engine, constructor conventions. |
| [`02-implementation-guide.md`](02-implementation-guide.md) | HOW to implement it with shakmaty 0.30.3: file layout, move lookup, the SAN-before-`play()` ordering pitfall, zobrist hashing for threefold repetition, fifty-move clock, snapshot assembly, a skeleton with TODO markers, and the verified FEN/SAN fixtures. |
| [`03-testing.md`](03-testing.md) | The complete test suite you ship (unit + integration test code that compiles against the real contract), and the run commands. **These tests cannot exist yet — read the note at its top.** |
| [`04-acceptance.md`](04-acceptance.md) | Definition-of-done checklist mapped to your assignment brief, the merge-gate commands, and the handoff checklist for B2. |

Existing source files in this folder:

| File | Role |
|---|---|
| [`contract.rs`](contract.rs) | The FROZEN public API: domain types + the `ChessEngine` trait. **You implement against it; you do not change it without a B1+B2 PR.** |
| [`testkit.rs`](testkit.rs) | `FakeEngine` + fixtures so B2/B3/B4 can test against the contract before your real engine lands. **Shared — not your deliverable; do not edit without a PR.** |
| [`mod.rs`](mod.rs) | Module wiring (`pub mod contract; pub mod testkit; pub use contract::*;`). You extend it with `pub mod engine;` (and a re-export) when your engine lands. |

## 3. What you own / what you don't touch

| You own (inside `backend/src/game/`) | You do NOT touch |
|---|---|
| `engine.rs` — the shakmaty-backed engine implementing the `ChessEngine` trait | `src/protocol/` (B3 + F3) and `docs/protocol-v1.md` |
| Any helper modules you add inside `src/game/` (e.g. `history.rs`) — keep them `pub(crate)`/private unless B2 needs them | `src/rooms/` (B2), `src/network/` (B3) |
| The module wiring in `mod.rs` (adding `pub mod engine;` + re-export of your concrete type) | `src/main.rs`, `src/lib.rs` (only via a PR if B3 asks you to expose something) |
| Your unit tests (`#[cfg(test)] mod tests` inside `engine.rs`) and `backend/tests/game_engine.rs` | `testkit.rs` — shared with B2/B3/B4; changes need a PR |
| The docs in this folder (this set) | `frontend/` (F-team), `backend/tests/testkit_demo.rs` (B4's example) |

Rules of thumb:

- Your public surface is `contract.rs` **plus** your constructor(s)
  (`new_game()`, see `01-contract.md` §6). Everything else stays private.
- Keep every `shakmaty` type inside `src/game/`. B2 must compile against your
  API without knowing shakmaty exists.
- If a change you need seems to require touching another module, talk to that
  module's owner first — do not edit their files.

## 4. The change rule (contract is frozen)

`contract.rs` is the frozen interface between you (B1) and B2. **Any signature
or semantic change to it is a PR agreed by B1 + B2** (B3 only if the wire
mirror in `src/protocol/` is affected — see the header comment of
`contract.rs`). Until such a PR exists, implement exactly what
[`01-contract.md`](01-contract.md) documents. Open points where the frozen
contract is ambiguous are listed in `01-contract.md` §7 — resolve them with B2
in a PR, do not silently pick a behavior that a room-actor test could depend
on.

## 5. Testing (pointer)

The full test suite you ship — code, expected values, run commands — lives in
[`03-testing.md`](03-testing.md). Quick reference:

```bash
cargo test game::              # unit tests inside src/game/
cargo test --test game_engine  # the engine integration suite
cargo test                     # everything — must stay green
```

(`testkit.rs`'s header comment refers to "README §Testing" — this is that
section; the contract tests B2/B3/B4 run against the fake are in
`backend/tests/testkit_demo.rs`, and your real engine must satisfy the same
contract.)

## 6. Definition of done (summary)

You are done when (full checklist and merge-gate commands in
[`04-acceptance.md`](04-acceptance.md)):

- [ ] `ShakmatyEngine` (or your concrete type) implements `ChessEngine`, plus
      an inherent `new_game()` constructor (and the `new_engine() ->
      Box<dyn ChessEngine>` factory helper if B2 asked for one).
- [ ] Every test in [`03-testing.md`](03-testing.md) is green
      (`cargo test game::` and `cargo test --test game_engine`), plus the whole
      existing suite (`cargo test`) stays green.
- [ ] `cargo build`, `cargo test`, `cargo fmt --check`, `cargo clippy` are clean.
- [ ] Your PR touches only files inside `backend/src/game/` (plus
      `backend/tests/game_engine.rs`).
- [ ] B2 can construct a game, apply moves and read snapshots purely through
      the contract — no reverse-engineering needed.

## 7. Day one quickstart

Read order:

1. This file.
2. [`contract.rs`](contract.rs) — the real types, line by line (15 minutes
   well spent; everything else refers back to it).
3. [`01-contract.md`](01-contract.md) — what each piece means, exactly.
4. [`02-implementation-guide.md`](02-implementation-guide.md) — how to build
   it with shakmaty 0.30.3 (includes the verified shakmaty API facts and the
   fixture FENs).
5. [`03-testing.md`](03-testing.md) — the tests you will ship; start them as
   soon as the skeleton compiles (TDD works well here).
6. [`04-acceptance.md`](04-acceptance.md) — pin the finish line on the wall.
7. Skim `backend/tests/testkit_demo.rs` — the import style your integration
   tests copy.
8. Skim `src/rooms/contract.rs` — only to see how B2 calls an engine (you
   implement the trait, you never import `rooms`).

Verify the base compiles before you start (Rust toolchain first, then cargo
commands from `backend/`):

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo build && cargo test
```

Expected baseline at the time this doc set was written: **build succeeds and
`cargo test` is green with 15 tests** (protocol round-trips + testkit fakes).
There is no engine yet — the game module currently contains only the frozen
contract, the testkit fake, and this documentation. That is the state you take
over from.
