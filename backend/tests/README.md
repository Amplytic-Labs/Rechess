# `backend/tests/` — B4 doc set: QA, integration, CI & deployment

> **Owner: B4 — Backend QA, Integration and Deployment.**
> You are the backend integration and quality owner, involved **from Day 1**,
> not after feature development ends. Feature owners (B1/B2/B3) write their own
> unit tests; you own cross-module tests, CI, deployment and measurement.
> **You are the merge authority: no module PR merges until you have verified it
> yourself.**

---

## 1. Mission

Prove that the backend built by B1 (game engine), B2 (rooms) and B3 (network)
works as one system, can be deployed reproducibly, and behaves predictably
under multiple simultaneous rooms and network failures. Concretely you own:

| Area | What it means | Doc |
|---|---|---|
| Quality gates | `cargo build` / `test` / `fmt` / `clippy` policies, run identically locally and in CI | [01-merge-gates.md](01-merge-gates.md) |
| Per-module merge verification | You re-verify every B1/B2/B3 PR **independently** before merge | [01-merge-gates.md](01-merge-gates.md) |
| Integration suite | End-to-end scenarios (happy path, races, reconnect, expiry) against the frozen contracts | [02-integration-suite.md](02-integration-suite.md) |
| CI | The backend GitHub Actions workflow (and later the frontend job) | [03-ci-workflow.md](03-ci-workflow.md) |
| Deployment | Dockerfile, env vars, WSS reverse proxy, health checks, runbook, rollback | [04-deployment.md](04-deployment.md) |
| Performance | The 50-room / 100-player benchmark workload, measured and recorded | [05-performance.md](05-performance.md) |

You do **not** own: the chess rules implementation (B1), the room actor
implementation (B2), core protocol design (B3 + F3), or frontend UI (F1–F4).
Owners supply test cases; you turn them into system tests and gates.

## 2. The MERGE AUTHORITY workflow

A module PR from B1/B2/B3 is merged **only** after all three of these, in
order. "Independently" means you pull the PR branch and run the gates yourself
— you never merge on the owner's local report alone.

1. **Owner's module tests green.** The PR author ran their own module suite
   (e.g. `cargo test game::`) and the PR shows it passing. If a module PR
   arrives with no tests for the behaviour it adds, send it back — that is a
   review comment, not a reason for you to write their tests.
2. **B4 runs the merge gates.** The full copy-paste checklist from
   [01-merge-gates.md](01-merge-gates.md) on the PR branch: whole-tree build,
   all suites, the per-module filter for the touched module, fmt, clippy,
   plus the contract-change review rule (§4 of that doc). The whole-tree
   `cargo build` is what catches contract drift between modules; the
   per-module filters let you attribute a failure to the right owner.
3. **Integration smoke of the merged tree.** After merging, you re-run the
   entire tree (`cargo build && cargo test`) plus the integration smoke from
   [02-integration-suite.md](02-integration-suite.md) §4. Today that smoke is
   the 15 contract tests + `GET /health`; it grows into the real two-client
   WebSocket smoke as B3's `/ws` endpoint lands.

If any gate is red, the PR does not merge — the owner fixes it or the team
agrees (with a tracked issue) to disable a specific gate temporarily. **Never
make main green by disabling a failing test without approval and a tracked
issue** (that rule is binding, from your brief §6).

## 3. Doc set index

| File | Contents |
|---|---|
| `README.md` (this file) | Mission, merge-authority workflow, current state, quickstart |
| `01-merge-gates.md` | The exact gate commands as a copy-paste checklist; what green looks like today and as modules land; contract-change review rule |
| `02-integration-suite.md` | The ten required end-to-end scenarios: preconditions, driving code sketches, expected events with revisions; the manual two-device smoke checklist |
| `03-ci-workflow.md` | The complete `.github/workflows/backend-ci.yml` YAML, the later frontend job, branch-protection recommendations |
| `04-deployment.md` | Multi-stage Dockerfile, .dockerignore, env vars, WSS proxy configs, health check, runbook, rollback |
| `05-performance.md` | The 50-room/100-player benchmark plan: what to measure, how, results template, acceptance thresholds |

## 4. Current state (verified by running the commands — 2026-11-27)

The frozen **contracts** exist and are green; the **implementations** do not
exist yet. That is deliberate: the contract layer lets you build the whole
gate/CI/deployment machinery before any module lands, so every module PR
arrives into an already-verified world.

| Gate | Status today | Detail |
|---|---|---|
| `cargo build` | **GREEN** | Whole tree compiles. 4 expected `unused dependency` manifest warnings: `rand`, `shakmaty`, `tower-http`, `uuid` — deliberate pins for B1/B2/B3, see [01-merge-gates.md](01-merge-gates.md) §5 |
| `cargo test` | **GREEN — 15 tests, 3 suites** | `protocol::` unit tests (4), `tests/protocol_contract.rs` (7), `tests/testkit_demo.rs` (4) — full table in [01-merge-gates.md](01-merge-gates.md) §6 |
| `cargo fmt --all -- --check` | **RED** | Formatting diffs in 8 files. Needs one mechanical `cargo fmt --all` PR first; the gate turns on after it. See [01-merge-gates.md](01-merge-gates.md) §4 |
| `cargo clippy --all-targets -- -D warnings` | **GREEN (exit 0)** | The 4 unused-dep warnings are cargo *manifest* warnings, not clippy lints, so `-D warnings` does not fail them |

What exists today: `src/protocol/` (all wire types + round-trip tests),
`src/game/contract.rs` + `src/game/testkit.rs` (engine trait + `FakeEngine`),
`src/rooms/contract.rs` + `src/rooms/testkit.rs` (registry/handle traits +
`FakeRegistry`/`FakeHandle`), `src/lib.rs` (`build_router()` + `GET /health`),
`src/main.rs` (env config: `RECHESS_HOST`, `PORT`, `RUST_LOG`).

What lands later (and what you turn on when it does):

| Owner lands | New green you should see | Your gate action |
|---|---|---|
| **B1** game engine | `cargo test game::` goes 0 → N tests; new `tests/game_engine.rs` | Add `tests/game_engine.rs` run to your expectations; extend integration scenarios with real moves |
| **B2** rooms actors | `cargo test rooms::` goes 0 → N; new `tests/rooms_actor.rs` | Switch harness scenarios from `FakeRegistry` to the real registry (one-line swap, see [02-integration-suite.md](02-integration-suite.md) §2) |
| **B3** WS layer | `cargo test network::` goes 0 → N; new `tests/network_protocol.rs` + `tests/network_ws.rs` | Add the socket-level smoke (two real WS clients) to the merged-tree step |

## 5. Quickstart

```bash
# In this sandbox the Rust toolchain lives under /tmp/rust — run this before
# any cargo command (normally rustup reads backend/rust-toolchain.toml, pinned
# to 1.100.0, automatically):
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

# One-time: the pinned toolchain profile is "minimal", so fmt/clippy are not
# preinstalled — add them once:
rustup component add --toolchain 1.100.0-x86_64-unknown-linux-gnu rustfmt clippy

cargo build                        # GREEN (+4 expected unused-dep warnings)
cargo test                         # 15 tests, 3 suites — all green
cargo fmt --all -- --check        # RED today — see 01-merge-gates.md §4
cargo clippy --all-targets -- -D warnings   # GREEN (exit 0)

# Boot the server and check the health endpoint:
cargo run                          # serves GET /health on 0.0.0.0:3000 by default
curl http://localhost:3000/health
# {"service":"rechess-backend","status":"ok"}
```

## 6. Working agreements for this folder

- Each `.rs` file in `backend/tests/` is a separate integration-test binary
  against the crate. You own this folder; feature owners add cross-module
  tests here only in coordination with you. Their unit tests live in their own
  `src/*/` folders.
- Test-file naming convention (used by the module docs and this doc set):
  `protocol_contract.rs` (today), `testkit_demo.rs` (today), then
  `game_engine.rs` (B1), `rooms_actor.rs` (B2), `network_protocol.rs` +
  `network_ws.rs` (B3). When you add the B4-owned end-to-end suite, extend
  this convention (e.g. `websocket_two_player.rs`, `room_isolation.rs`,
  `concurrent_moves.rs`, `reconnect_flow.rs`, `game_lifecycle.rs`,
  `load_smoke.rs`).
- Dev-dependencies you will need (add via a short PR to `Cargo.toml` — you
  curate it together with the team): `tower` (feature `util`, for
  `.oneshot()` router calls) and `tokio-tungstenite` (raw WS clients for the
  socket-level suite). Keep `Cargo.lock` committed.
- Docs live here too; the CI YAML is documented in
  [03-ci-workflow.md](03-ci-workflow.md) and created by you at the repo root
  as `.github/workflows/backend-ci.yml`.
