# Rechess Backend

Authoritative multiplayer chess backend. **Rust · axum 0.8 · tokio · shakmaty**

> The server is the only authority on game state. The web client renders;
> it never decides move legality.

## Status

This is the **team starting base**: a compiling skeleton with the three
backend modules stubbed and full per-owner instructions in each module
folder. Nobody has implemented module logic yet — that is the owners' work,
guided by their READMEs.

## Module map & ownership

| Folder | Owner | Purpose | Start here |
|---|---|---|---|
| `src/game/` | **B1** | Chess rules wrapper (`shakmaty`), authoritative snapshots | `src/game/README.md` |
| `src/rooms/` | **B2** | Room registry, room actors, seats, tokens, lifecycle | `src/rooms/README.md` |
| `src/network/` + `src/protocol/` (B3 creates) | **B3** | WebSocket transport, wire protocol v1 | `src/network/README.md` |
| `tests/` + `.github/workflows/` | **B4** | Integration tests, CI, Docker, deployment | `tests/README.md` |
| `src/main.rs`, `Cargo.toml`, `rust-toolchain.toml` | shared (B4 curates) | Entry point, dependency pins | this file |

**Working agreements:** each owner works inside their folder; cross-folder
changes (main.rs, Cargo.toml) go through a short PR the affected owners
approve. `docs/protocol-v1.md` is co-owned by B3 + F3.

## Quick start

```bash
cd backend
cargo run                 # serves GET /health on 0.0.0.0:3000 by default
curl http://localhost:3000/health

cargo test                # module unit tests (as they land)
cargo test game::         # only B1's tests, etc.
```

## Pinned versions (Day-1 policy from the master plan)

Toolchain: Rust **1.100.0** (`rust-toolchain.toml`). Exact resolved crate
versions are in **`Cargo.lock` — always commit it.**

| Crate | Pinned | Purpose / owner |
|---|---|---|
| axum | 0.8.9 (+`ws`) | HTTP + WebSocket transport (B3) |
| tokio | 1.54.1 | Async runtime, actors (B2/B3) |
| serde / serde_json | 1.0.229 / 1.0.154 | Wire format (B3) |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23 | Structured logs |
| tower-http | 0.7.1 (+`trace`) | TraceLayer (B3) |
| futures-util | 0.3.34 | Split-socket streams (B3) |
| shakmaty | 0.30.3 | Chess rules (B1) — **GPL-3.0+, see note** |
| uuid / rand | 1.29.3 / 0.10.4 | Room codes, tokens (B2) |

> **License note:** `shakmaty` is GPL-3.0-or-later. The team must confirm on
> Day 1 that this is acceptable for the assignment; if not, B1 swaps to an
> alternative (`cozy-chess`, MIT) before building on it — see
> `src/game/README.md` §4.

## Configuration (B4 documents; keep this table in sync)

| Variable | Default | Purpose |
|---|---|---|
| `RECHESS_HOST` | `0.0.0.0` | Bind host (`127.0.0.1` for local-only) |
| `PORT` | `3000` | Bind port |
| `RUST_LOG` | `info` | Log verbosity, e.g. `rechess_backend=debug` |

## Deployment notes (B4 owns the full runbook — `tests/README.md` §5–7)

- Deploy behind a reverse proxy that terminates TLS and forwards **WSS** with
  WebSocket upgrade enabled and long timeouts.
- Runtime configuration comes only from environment variables.
- **In-memory room state:** one process, no database; a restart clears all
  rooms. This is an accepted MVP limitation — document it in demos.
- Build artifact: `cargo build --release` → `target/release/rechess-backend`;
  containerization plan is in `tests/README.md` §6.

## Linking plan (how the modules come together)

1. **Day 1:** B1 proves the shakmaty PoC; B2 + B3 freeze the registry/handle
   API; B3 + F3 freeze `docs/protocol-v1.md`.
2. **Day 3–4 vertical slice:** B1 engine (new game, e2-e4, snapshot) → B2
   actor applies it → B3 echoes a snapshot to a test client.
3. Everything after that grows inside the modules without structural
   changes; `src/main.rs` only ever wires routers and startup.
