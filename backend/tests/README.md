# `backend/tests/` — Owner: **B4** (Backend QA, Integration and Deployment)

> You are the backend integration and quality owner, involved **from Day 1**,
> not after feature development ends. Feature owners (B1/B2/B3) write their own
> unit tests; you own cross-module tests, CI, deployment and measurement.

---

## 1. What this folder is for

Rust integration tests: each `.rs` file here is compiled as a separate test
binary against the crate. You own this folder. Feature owners must not add
their cross-module tests here without coordinating with you — their module
unit tests live in their own `src/*/` folders.

Nothing has been created here yet on purpose — the harness below is yours to
build. The rest of this file is the plan for how.

## 2. How to wire integration tests against the running server

Add the dev-dependency you need (not yet in `Cargo.toml` — you add it, it is
your call):

```toml
[dev-dependencies]
tower = { version = "0.5", features = ["util"] }  # .oneshot() on the Router
# optionally: tokio-tungstenite (raw WS client) if you drive real sockets
```

The crate's router needs to be reachable from tests. Today `build_router()`
lives inside `src/main.rs` (private). **Day-1 action:** ask B3 to move the
router into a library target so tests can call it, e.g.:

```rust
// tests/health_check.rs — your first smoke test
#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = rechess_backend::build_router();
    use tower::ServiceExt; // .oneshot()
    let res = app.oneshot(axum::http::Request::builder()
        .uri("/health").body(axum::body::Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
}
```

(The typical layout is `src/lib.rs` exposing `pub fn build_router()` plus the
modules; `main.rs` shrinks to config + serve. You own making that change
happen as a coordinated PR — it is structural, not feature work.)

For socket-level tests, bind an ephemeral port instead:

```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
let addr = listener.local_addr().unwrap();
// spawn axum::serve(listener, router) on a tokio task, connect real WS clients to addr
```

## 3. Required test scenarios (from your brief — build these incrementally)

- Create room → join → move → game finish (checkmate) → rematch, full flow.
- Reconnect: disconnect one client, resume with token, full snapshot returned.
- Draw offer/accept and resignation flows.
- Authorization: wrong token cannot move; room code alone is not authorization.
- Multiple rooms concurrently: **50 rooms / up to 100 connected players** as
  the initial benchmark workload; room A's activity never affects room B.
- Failure cases: malformed JSON, oversized payloads, unknown versions, dead
  connections, queue saturation.

## 4. Local repeatable test setup

Create a small harness (a helper module `tests/common/mod.rs` that starts the
server on an ephemeral port and hands out connected WS clients — use
`tokio-tungstenite` as a raw WS client). Make fixtures deterministic:
- FEN constants for edge-case positions (get them from B1),
- fixed seeds/short timeouts for lifecycle tests (B2 provides knobs),
- no real-time sleeps where `tokio::time::pause()` will do.

Document the exact command to run everything:

```bash
cd backend
cargo test                 # unit + integration
cargo test --test health_check    # one binary
```

## 5. CI workflow (GitHub Actions) — starter example

You own `.github/workflows/` at the repo root. Keep this shape, pinned to the
toolchain in `backend/rust-toolchain.toml`:

```yaml
# .github/workflows/backend.yml  (create this file)
name: Backend CI
on:
  pull_request: { paths: ['backend/**'] }
  push: { branches: [main], paths: ['backend/**'] }
defaults: { run: { working-directory: backend } }
jobs:
  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings    # agree the policy with the team first
      - run: cargo test
      - run: cargo build --release
```

Coordinate with F3/F4 so a frontend job (typecheck + lint) runs in the same
repo, even as a separate workflow file. Do not make main green by disabling
failing tests without approval and a tracked issue.

## 6. Dockerfile — starter example (create `backend/Dockerfile`)

```dockerfile
# Multi-stage, cargo-chef for layer caching, minimal runtime, non-root user.
FROM lukemathwalker/cargo-chef:latest-rust-1.100 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin rechess-backend

FROM debian:trixie-slim AS runtime
RUN useradd --system --uid 1001 rechess
COPY --from=builder /app/target/release/rechess-backend /usr/local/bin/
USER rechess
ENV PORT=3000
EXPOSE 3000
CMD ["rechess-backend"]
```

Health checks use `GET /health` (already implemented in `src/main.rs`).

## 7. Deployment & runbook checklist (document in backend/README.md §)

- [ ] Environment variables documented (`RECHESS_HOST`, `PORT`, `RUST_LOG`)
- [ ] Reverse proxy performs WebSocket upgrade + WSS (TLS termination) and
      allows long-lived connections (no aggressive timeouts)
- [ ] Logs structured, no `player_token` or secrets ever logged
- [ ] Room state is **in-memory only**: restart wipes rooms — document this
      limitation for the report/demo
- [ ] README has exact local + deployed connection instructions for the Expo
      client (LAN IP vs WSS URL)
- [ ] Performance report: CPU, RSS, connections, rooms, move latency at idle
      and under the 50-room/100-player load — **measured**, with hardware,
      OS, build profile and tooling recorded; never claimed from language
      choice alone

## 8. Coordination

- B1: convert rule edge cases into regression tests (fixtures).
- B2: room races + lifecycle tests; ask for deterministic knobs early.
- B3: protocol/malformed-input/authorization tests; co-own the two-client
  harness (don't build duplicate harnesses).
- F3/F4: end-to-end tests spanning the real client; make frontend typecheck
  failures visible in CI.
- Maintain the known-issues list and a release acceptance checklist; coordinate
  the freeze window before final submission.
