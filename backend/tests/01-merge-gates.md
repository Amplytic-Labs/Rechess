# 01 — Merge gates: the exact commands

> **B4 doc set, part 1.** Every module PR (B1/B2/B3) and every cross-folder PR
> passes this checklist on the PR branch *before* merge, run by you
> personally. Copy-paste, top to bottom. Owner: B4 — see
> [README.md](README.md) §2 for the merge-authority workflow this belongs to.

---

## 1. One-time setup

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

# Toolchain is pinned to 1.100.0 via rust-toolchain.toml, profile "minimal":
# rustfmt and clippy must be added once per machine.
rustup component add --toolchain 1.100.0-x86_64-unknown-linux-gnu rustfmt clippy
```

## 2. The gate checklist (run on the PR branch)

Run all six, in this order. A PR is mergeable only when every applicable gate
is green.

```bash
cd /home/z/my-project/Rechess/backend

# GATE 1 — whole tree compiles. This is the contract-drift catcher: if a PR
# changed a contract signature, every consumer of that contract fails HERE.
cargo build

# GATE 2 — all suites (unit + integration + doc-tests).
cargo test

# GATE 3 — per-module filters: attribute failures to the right owner and
# confirm the PR's own module is covered.
cargo test protocol::    # B3's wire types (4 tests today)
cargo test game::        # B1's engine   (0 tests today — grows when B1 lands)
cargo test rooms::       # B2's registry  (0 tests today — grows when B2 lands)
cargo test network::     # B3's WS layer (0 tests today — grows when B3 lands)

# GATE 4 — formatting (required as-is; see §4).
cargo fmt --all -- --check

# GATE 5 — static analysis, warnings denied.
cargo clippy --all-targets -- -D warnings

# GATE 6 — release build (the deployment artifact builds; runs in CI, run it
# locally before a release tag or a Dockerfile change).
cargo build --release
```

What each gate must show:

| # | Command | Merge when | Today (2026-11-27) |
|---|---|---|---|
| 1 | `cargo build` | exit 0 | **GREEN** — exits 0 with 4 expected unused-dependency manifest warnings (`rand`, `shakmaty`, `tower-http`, `uuid`) — see §5 |
| 2 | `cargo test` | `test result: ok` in **every** suite, 0 failed | **GREEN** — 15 tests / 3 suites (§6) |
| 3 | `cargo test <module>::` | PR's module filter runs ≥ 1 test if the PR touches that module | `protocol::` 4 passed; `game::`/`rooms::`/`network::` match 0 tests until those modules land |
| 4 | `cargo fmt --all -- --check` | exit 0, empty output | **GREEN** (as of 2026-11-27) — exit 0, empty output; the base was formatted |
| 5 | `cargo clippy --all-targets -- -D warnings` | exit 0 | **GREEN** — exit 0 (the 4 manifest warnings do not fail clippy; they are not rustc lints) |
| 6 | `cargo build --release` | exit 0 | **GREEN** — verified: `Finished release profile [optimized] target(s) in 1m 07s`, binary `target/release/rechess-backend` ≈ 2.0 MB (thin LTO + stripped) |

## 3. Gate 3 in detail: per-module attribution

`cargo test <filter>` matches by full test path. When a suite fails, re-run
the failing module's filter and look at which binary produced the failure:

- `protocol::` → unit tests inside `src/protocol/mod.rs` → **B3** (wire types)
- `game::` → unit tests inside `src/game/` → **B1** (engine)
- `rooms::` → unit tests inside `src/rooms/` → **B2** (registry/actors)
- `network::` → unit tests inside `src/network/` → **B3** (WS layer)
- a failure in `tests/<file>.rs` → whoever owns that scenario: today
  `tests/protocol_contract.rs` is B3's contract suite and
  `tests/testkit_demo.rs` is yours (B4); see §7 for the growth table.

Today, running the four filters produces exactly this (the "filtered out"
counts are the other suites' tests, which is expected — each filter runs
against every test binary but matches only its own):

```text
$ cargo test protocol::   → lib unittests:  4 passed   | main.rs: 0 | protocol_contract.rs: 0 (7 filtered out) | testkit_demo.rs: 0 (4 filtered out)
$ cargo test game::       → all four binaries: 0 passed (4, 7 and 4 filtered out)
$ cargo test rooms::      → all four binaries: 0 passed (4, 7 and 4 filtered out)
$ cargo test network::    → all four binaries: 0 passed (4, 7 and 4 filtered out)
```

## 4. Gate 4 (fmt): required as-is

The base was formatted (`cargo fmt --all -- --check` exits 0, verified
2026-11-27) — the gate is required as-is.

## 5. Gate 5 (clippy): the deliberate unused-dependency pins

`cargo build` and `cargo clippy` print four warnings today:

```text
warning: unused dependency `rand`        # B2: room codes / tokens
warning: unused dependency `shakmaty`   # B1: chess rules
warning: unused dependency `tower-http`  # B3: TraceLayer for network/
warning: unused dependency `uuid`        # B2: player IDs / command IDs
```

These are **expected and deliberate**. The pins exist so the versions are
frozen from Day 1 (master-plan version policy); the consuming code is B1/B2/B3
work that has not landed yet. The warnings disappear **on their own** as each
module's implementation lands.

Rules for this gate:

- Do not "fix" the warnings by removing the dependencies — that unpins the
  versions and breaks the freeze policy.
- `cargo clippy --all-targets -- -D warnings` **exits 0** today: the unused
  dependencies are cargo *manifest* warnings (`cargo::unused_dependencies`,
  warn by default), not rustc lints, so `-D warnings` does not promote them.
  The gate is green as-is; no action needed.
- If the team wants zero-warning logs (or a future toolchain starts failing on
  them), silence the lint **documented**, with a comment pointing here:

  ```toml
  # Cargo.toml (a short cross-folder PR — you curate it)
  [lints.cargo]
  unused_dependencies = "allow"   # deliberate Day-1 pins; see backend/tests/01-merge-gates.md §5
  ```

  (Verified on the pinned toolchain 1.100.0: this makes the manifest warnings
  disappear without touching the pins. Remove the allow once all of B1/B2/B3
  have landed, so the lint catches genuinely stale deps again.)
- Any *other* clippy warning must be fixed in the PR that introduces it —
  never `#[allow]`-ed ad hoc.

## 6. What green looks like TODAY

Verified 2026-11-27 on the tree as it stands. This is your regression
baseline: if `cargo test` shows anything other than these numbers before a
module lands, something regressed.

| Suite (binary) | Tests | Test names | Status |
|---|---|---|---|
| `src/protocol/mod.rs` (lib unittests, `protocol::tests`) | 4 | `client_move_envelope_round_trips`, `every_client_command_round_trips`, `malformed_payload_is_rejected_not_panicking`, `server_state_envelope_round_trips` | green |
| `tests/protocol_contract.rs` | 7 | `doc_section_2_1_example_round_trips`, `game_state_payload_is_the_snapshot`, `only_room_created_joined_carry_player_token`, `revision_lives_on_the_envelope_only`, `malformed_and_unknown_commands_fail_cleanly`, `every_error_code_round_trips`, `promotion_serializes_as_lowercase_letter` | green |
| `tests/testkit_demo.rs` | 4 | `fake_engine_scripts_results_and_records_calls`, `fake_registry_serves_the_room_lifecycle`, `fake_handle_records_commands_and_emits_events`, `generic_code_works_against_any_registry` | green |
| `src/main.rs` (bin unittests) | 0 | — | green |
| Doc-tests | 0 | — | green |
| **Total** | **15** | | **green** |

Exact summary lines you should see from `cargo test`:

```text
running 4 tests   … test result: ok. 4 passed; 0 failed   (protocol::tests)
running 0 tests   … test result: ok. 0 passed; 0 failed   (main.rs)
running 7 tests   … test result: ok. 7 passed; 0 failed   (protocol_contract)
running 4 tests   … test result: ok. 4 passed; 0 failed   (testkit_demo)
Doc-tests         … test result: ok. 0 passed; 0 failed
```

## 7. How the table grows as modules land

The integration-test file names below are the **convention** used across the
module docs; keep new suites consistent with them. When a module lands, the
merge checklist for its PR is: this doc's gates + the module's new suites
green + the scenario work from [02-integration-suite.md](02-integration-suite.md)
that the module unlocks.

| Lands | New per-module unit tests | New integration files (in `tests/`) | New expected totals |
|---|---|---|---|
| Today | `protocol::` ×4 | `protocol_contract.rs` (7), `testkit_demo.rs` (4) | 15 tests, 3 suites |
| **B1** game engine | `game::` ×N (B1's brief: initial position, castling incl. through check, en passant expiry, 4 promotions, check/checkmate/stalemate/insufficient material, FEN/SAN) | `game_engine.rs` | `cargo test game::` ≥ 1; total grows by N + suite |
| **B2** rooms | `rooms::` ×N (create/join/reject codes, capacity race, invalid token, expiry, disconnect/grace, rematch agreement) | `rooms_actor.rs` | `cargo test rooms::` ≥ 1; total grows by N + suite |
| **B3** WS layer | `network::` ×N (envelope round-trips incl. all doc examples, auth gate, payload limits, rate limits, heartbeat) | `network_protocol.rs`, `network_ws.rs` | `cargo test network::` ≥ 1; total grows by N + 2 suites |

When B1 lands you also flip [02-integration-suite.md](02-integration-suite.md)
scenarios from `FakeEngine`-scripted outcomes to real moves (the fool's-mate
fixture already exists: `game::testkit::foolsmate_snapshot()`); when B2 lands
you swap `FakeRegistry` for the real registry; when B3 lands you add the
socket-level suites. Each flip is its own PR, reviewed by you.

## 8. Review checklist for contract changes (the same-PR rule)

`docs/protocol-v1.md` is the single source of truth for the wire. A PR that
changes **any** protocol surface must update **all** mirrors in the same PR —
this is the drift-prevention rule from the master plan, and you are its
enforcer at merge time. Block the merge (or request the missing piece) if:

- [ ] `backend/src/protocol/` (Rust types) updated for any envelope / command
      / event / error-code change — and `tests/protocol_contract.rs` fixtures
      extended with the exact new examples.
- [ ] `frontend/src/types/protocol.ts` (F3's TypeScript types) updated in the
      same PR with the same fields, and `frontend/src/types/protocol.fixtures.ts`
      updated (they are compiled by `npm run typecheck`).
- [ ] `docs/protocol-v1.md` updated: tables, examples, and §6 error codes.
- [ ] Any rename/removal of a wire field is called out explicitly in the PR
      description (silently changing semantics is forbidden); the PR carries
      B3's *and* F3's approval.
- [ ] Revision semantics unchanged or re-specced: `revision` stays on the
      envelope only, `player_token` stays only in `room.created`/`room.joined`
      targeted events (both asserted today by
      `tests/protocol_contract.rs`).
- [ ] If `game/` or `rooms/` contract traits changed: the change carries the
      affected co-owners' approval (B1+B2 for `game::contract`; B2+B3 for
      `rooms::contract`, B1 if the engine trait is affected), per the change
      rules at the top of those files.
- [ ] The PR does not weaken a gate: no `#[ignore]` without a tracked issue,
      no clippy `#[allow]` beyond the documented §5 case, no removed test.

## 9. Frontend gates (separate, same authority)

Frontend PRs (F1–F4) are gated by their own commands — there is **no frontend
test runner by design** (see `frontend/RULES.md` §5); UI quality is verified by
manual checklists in the module READMEs:

```bash
cd frontend
npm run typecheck   # tsc --noEmit — zero errors (also compiles protocol fixtures)
npm run lint        # eslint — zero errors/warnings
```

Your job is that these run in CI on the same PR so drift is visible — the job
definition is in [03-ci-workflow.md](03-ci-workflow.md) §3. The wire-contract
cross-check is already covered on the backend side by
`tests/protocol_contract.rs` round-tripping the same JSON that
`protocol.fixtures.ts` types.
