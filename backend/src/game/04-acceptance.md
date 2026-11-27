# 04 — Acceptance: definition of done, merge gate, handoff

This document is the finish line: every requirement from your assignment brief
that B1 owns, mapped to where it is satisfied; the exact merge-gate commands
(B4 runs them); and the handoff checklist B2 uses to accept your module.

The team-level definition of done (master plan): *"A task is done only when
its code is merged, its relevant tests pass, its interface is documented, and
the next owner can integrate it without reverse-engineering assumptions."*

---

## 1. Definition-of-done checklist (mapped to your brief)

Every item from the B1 brief (`02_B1_Chess_Engine.md`), with where it is
specified/implemented. Tick them in your PR description.

### Required work (brief §2)

| # | Requirement | Where it is satisfied | Done when |
|---|---|---|---|
| 1 | Create the Rust game module; evaluate the chess crate with a minimal PoC; confirm exact APIs, legal-move behaviour, limitations | PoC notes: [`02-implementation-guide.md`](02-implementation-guide.md) §14 (all values machine-verified against shakmaty 0.30.3); run the PoC yourself on Day 1 | PoC run on your machine matches §14; discrepancies raised with the team immediately, never silently approximated |
| 2 | Wrap the library in an application-owned API; keep crate types from leaking into the room manager | `contract.rs` (frozen) + your `engine.rs` behind it | `src/game/` public surface = contract types + constructors only; no `shakmaty` name appears in any public signature; B2 compiles against you without shakmaty |
| 3 | Accept a move as from/to + optional promotion; validate square syntax and promotion values before applying | `apply_move` validation order: [`01-contract.md`](01-contract.md) §5.1 | `InvalidSquare` for bad strings (`"e9"`, `"E2"`…); `PromotionRequired` / `UnexpectedPromotion` for mismatches; wire promotion letters (`"q"`/`"r"`/`"b"`/`"n"`) are parsed by B2/B3, you receive `Option<Promotion>` |
| 4 | Reject illegal moves, wrong-turn moves, moves after completion, unsupported promotion input — stable typed errors | [`01-contract.md`](01-contract.md) §4 (all six `GameError` variants with exact triggers) | every variant has a green test in [`03-testing.md`](03-testing.md); no panics, no stringly-typed errors |
| 5 | Expose current FEN, side to move, last move, SAN history, game status, legal moves | `GameSnapshot` (contract) + assembly guide: [`02-implementation-guide.md`](02-implementation-guide.md) §9 | snapshot has all 8 fields; `legal_moves` UCI-style lowercase; castling encoded as `"e1g1"`/`"e1c1"` |
| 6 | Detect checkmate and stalemate through the rules library; confirm how the library represents terminal states | [`02-implementation-guide.md`](02-implementation-guide.md) §8 (`is_checkmate()` / `is_stalemate()`; shakmaty has no "game over" object — it exposes predicates, you map them to `GameStatus`/`GameResult`) | Fool's mate → `Checkmate`; stalemate FEN fixture → `Stalemate` (both green in 03) |
| 7 | Application-level resignation, draw offer/acceptance, rematch reset — lifecycle, not board moves | [`01-contract.md`](01-contract.md) §5.2–5.5, §10 (board untouched) | resign/draw tests green; `reset_for_rematch` snapshot equals a fresh game |
| 8 | Track draw-claim conditions the library does not expose (repetition, fifty-move) if in agreed MVP rules | Repetition: §6 (zobrist `HashMap<u64, u8>`); fifty-move: §7 (`halfmoves() >= 100`); both implemented as **automatic** draws (documented policy, see open points) | threefold + fifty-move tests green; the auto-vs-claimable decision confirmed with B2 (see §5 below) |
| 9 | Keep a move history sufficient to reconstruct the game and a readable move list | `history_san` (+ the position itself is authoritative) | history tests green, including suffixes (`"Qh4#"`) |
| 10 | Unit tests + short module README showing how B2 calls the engine | This doc set; B2-callable API summary in [`01-contract.md`](01-contract.md) §6–§8 | all of [`03-testing.md`](03-testing.md) green; README/01/02/03/04 consistent with your final code |

### Deliverables (brief §7)

- [ ] **Compiling `game` module with a small, stable public API** —
      `engine.rs` implements the frozen `ChessEngine` trait; `mod.rs` wires
      `pub mod engine;` (+ re-export).
- [ ] **Unit tests for standard movement and all required special rules** —
      [`03-testing.md`](03-testing.md) §2 in `backend/tests/game_engine.rs`
      (+ optional `#[cfg(test)]` unit module).
- [ ] **Typed error model** — `GameError` from the contract, exact variant per
      input; no `panic!` on any input path.
- [ ] **Snapshot representation** — `GameSnapshot` with FEN, turn, status,
      result, last move, SAN history, legal moves, draw offer.
- [ ] **Short README/example showing B2 how to instantiate and use** — this
      README + [`01-contract.md`](01-contract.md) §6 (constructors) and §8
      (usage pattern); keep them updated if anything shifts in a B1+B2 PR.
- [ ] **Note recording the exact crate version and rules NOT provided by the
      crate** — see §6 below; keep it in sync with reality at merge time.

### Correctness checklist (brief §5 — covered by tests in 03)

- [ ] Normal movement, capture, blocked sliding paths (SAN/FEN assertions in
      every sequence + illegal-move tests)
- [ ] Turn enforcement (white/black alternation; out-of-turn rejected)
- [ ] Check: king-exposing moves rejected; check shown in snapshot status
- [ ] Checkmate/stalemate from known FEN fixtures
- [ ] Castling: both sides/flanks; rights absent, squares occupied, king in
      check or traversed square attacked → rejected
- [ ] En passant: legal capture, correct pawn removal, one-ply expiry
- [ ] Promotion: all four choices; missing/invalid promotion rejected
- [ ] History and FEN: snapshot after each move matches expected position and
      notation
- [ ] Lifecycle: resignation, accepted draw, reset do not corrupt board or
      history

## 2. Merge gate — the commands (B4 runs these; run them first)

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

cargo fmt --check        # formatted (run `cargo fmt` first)
cargo clippy -- -D warnings   # no warnings
cargo build              # compiles
cargo test               # EVERYTHING green: the 15 baseline tests + your suite
cargo test game::        # your in-module unit tests (if any)
cargo test --test game_engine   # the engine integration suite from 03-testing.md
```

**Gate rule:** all commands exit 0 AND none of the baseline tests changed
behavior. If `testkit_demo.rs` fails, you changed contract semantics by
accident — fix your engine, not the testkit.

## 3. PR checklist (reviewer's first pass)

- [ ] PR touches only: `backend/src/game/**` (including `mod.rs` wiring) and
      `backend/tests/game_engine.rs`.
- [ ] `contract.rs` and `testkit.rs` are byte-identical to `main` (unless the
      PR IS an agreed B1+B2 contract change, titled and reviewed as such).
- [ ] No `use crate::protocol`, `use crate::rooms`, no tokio/axum/serde
      imports inside `src/game/` (the engine is pure synchronous logic).
- [ ] No shakmaty type in any public signature of `src/game/`.
- [ ] No `Mutex`/`RwLock`-style interior locking (the room actor serializes
      calls — locking is a smell).
- [ ] No panics reachable from input: every bad input → `GameError`.
- [ ] The tests of [`03-testing.md`](03-testing.md) are present and green
      (with the doc updated in the same PR if a fixture legitimately changed).

## 4. Handoff checklist for B2 (what B2 needs working)

B2 accepts your module when ALL of these are true — go through it together on
Day 3–4 (the vertical-slice milestone):

- [ ] **Trait implementation** — `ShakmatyEngine: ChessEngine` with the exact
      six methods, semantically as documented in
      [`01-contract.md`](01-contract.md) §5.
- [ ] **`new_game()` constructor** — B2's only way to build a fresh game:
      `ShakmatyEngine::new_game()`, then store as `Box<dyn ChessEngine>` in
      the room actor (proven by `engine_works_behind_a_dyn_box` in 03).
- [ ] **(If agreed) the factory helper** — `pub fn new_engine() ->
      Box<dyn ChessEngine>` in `src/game/` (see
      [`01-contract.md`](01-contract.md) §6); otherwise B2 constructs the
      concrete type directly.
- [ ] **Snapshot mapping verified** — B2 can assemble the wire `GameSnapshot`
      from your domain snapshot plus room data without asking you anything:
      field-by-field table in [`01-contract.md`](01-contract.md) §3 and §8
      (`history_san` → `move_history`, `draw_offered_by` → `draw_offer`, …).
- [ ] **Error mapping verified** — every `GameError` variant maps to a
      protocol `ErrorCode` B2/B3 agreed (`docs/protocol-v1.md` §6; the table
      lives with B2).
- [ ] **Open points resolved or explicitly deferred** — see §5 below; B2 must
      not discover a semantic surprise in their room tests.
- [ ] **B2 can run the engine standalone** — with `cargo test game::` green,
      B2 rebuilds any failing scenario as a fixture FEN + move sequence and
      reproduces it without a running server.

## 5. Open points to close with B2 (before or at handoff)

These are the places where the frozen contract underdetermines behavior;
this doc set documents a recommended default for each. Close each in a small
B1+B2 PR (updating [`01-contract.md`](01-contract.md) §5/§7 and the
corresponding tests in the same change):

1. `GameAlreadyOver` precedence in `apply_move` (documented: checked FIRST,
   before square parsing, for determinism).
2. `accept_draw` with no active opponent offer (documented: no-op returning
   the unchanged snapshot; no `GameError` variant fits).
3. Does an applied move clear an active draw offer? (documented: yes —
   implicit decline.)
4. Fifty-move / threefold as **automatic** draws (documented) vs claimable
   draws (would need a contract change — a claim entry point).

## 6. Crate record (fill in at merge; brief deliverable)

Keep this section accurate — it is the "exact crate version and rules not
provided by the crate" deliverable:

| Item | Value |
|---|---|
| Chess crate | **shakmaty 0.30.3** (pinned in `backend/Cargo.toml`; exact build in `Cargo.lock`), default features (`std`, `magics`) |
| License | GPL-3.0-or-later — team decision recorded in `backend/README.md` / master plan |
| Provided by the crate | board representation, legal-move generation, FEN, SAN (incl. `+`/`#` via `SanPlus`), UCI, check/checkmate/stalemate predicates, insufficient-material predicate, halfmove clock (`halfmoves()`), zobrist hashing primitives |
| NOT provided — app-implemented (you) | threefold-repetition occurrence counting (zobrist map), fifty-move threshold detection (`>= 100`), draw offer/accept/resign/rematch lifecycle, auto-draw policy, turn/square/promotion validation errors |
| Known API pitfalls | `play()` consumes the position by value; `San` has no `+`/`#` suffix (use `SanPlus`); `Move::to()` on castling returns the rook square; `zobrist_hash` returns `Zobrist64` (newtype), not `u64`; `is_game_over()` does NOT include fifty-move or repetition |

If any row of this table stops being true (crate upgrade, feature change),
update it in the same PR as the change.
