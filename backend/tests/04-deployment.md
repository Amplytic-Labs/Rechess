# 04 — Deployment

> **B4 doc set, part 4.** The repeatable deployment artifact (multi-stage
> Docker image), runtime configuration, WSS reverse proxy, health check, and
> the runbook (deploy, verify, rollback). Owner: B4. You create
> `backend/Dockerfile` and `.dockerignore` at the repo root — the file
> contents below are exact.

---

## 1. Verified facts this plan builds on (2026-11-27)

Ran on the current tree with the pinned toolchain (Rust 1.100.0,
`rust-toolchain.toml`):

- `cargo build --release` — **green**: `Finished release profile [optimized]
  target(s) in 1m 07s`; binary `target/release/rechess-backend`,
  ~2.0 MB (thin LTO + stripped symbols, per `[profile.release]` in
  `Cargo.toml`).
- The release binary boots and serves the health endpoint. Verified live:
  `RECHESS_HOST=127.0.0.1 PORT=3777 ./target/release/rechess-backend` (and the
  debug build likewise) logs
  `Rechess backend started — health endpoint at GET /health address=127.0.0.1:3777`,
  and `curl http://127.0.0.1:3777/health` returns **HTTP 200**,
  `content-type: application/json`, body:
  `{"service":"rechess-backend","status":"ok"}` (43 bytes).
- Graceful shutdown is **not implemented** today (`axum::serve` without
  `with_graceful_shutdown`): SIGTERM drops connections immediately. Rooms are
  in-memory and lost either way (§8). If the team wants drain-on-stop, that
  is a small B3 change to `main.rs` — decide before first deployment.

## 2. `backend/Dockerfile` (you create this file)

Multi-stage build: pinned Rust image builder → minimal non-root runtime.
Build context is the **repo root** (that is what makes the `.dockerignore`
in §3 meaningful).

```dockerfile
# backend/Dockerfile — owner: B4 (backend/tests/04-deployment.md)
# Build from the REPO ROOT so .dockerignore applies:
#   docker build -f backend/Dockerfile -t rechess-backend:<tag> .

# ---- Stage 1: builder — pinned Rust toolchain image -------------------------
FROM rust:1.100 AS builder
WORKDIR /app

# Copy the whole backend (Cargo.toml + Cargo.lock + src + tests).
# Cargo.lock is committed — that is what makes the build reproducible.
COPY backend/ .

RUN cargo build --release --bin rechess-backend

# ---- Stage 2: runtime — minimal, non-root ----------------------------------
# Debian-slim matches the rust:1.100 builder's libc family.
# glibc must match the builder image; on the first build verify the image
# boots (runbook §7 smoke) before tagging it for the platform.
FROM debian:trixie-slim AS runtime

# healthcheck tool (only for docker-run setups; orchestrators probe /health
# themselves — drop curl and the HEALTHCHECK below if you go distroless §6)
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 1001 --no-create-home rechess

COPY --from=builder /app/target/release/rechess-backend /usr/local/bin/rechess-backend

# Runtime configuration comes ONLY from the environment (§4).
ENV PORT=3000
EXPOSE 3000

USER rechess
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:${PORT}/health || exit 1

CMD ["rechess-backend"]
```

Why this shape (your brief §2: "non-root runtime user where practical and a
minimal runtime image compatible with the build"):

- **Pinned builder** (`rust:1.100`) matches `rust-toolchain.toml` — a rebuild
  next month produces the same binary family; `Cargo.lock` (committed) pins
  the crates.
- **No source, no cargo, no toolchain in the runtime image** — only the
  stripped ~2 MB binary plus a minimal Debian userspace.
- **Non-root**: `USER rechess` (uid 1001) — the process cannot write to the
  image or the host FS beyond its own memory.
- **Layer-caching variant** (optional, later): a `cargo-chef` stage
  (`lukemathwalker/cargo-chef:latest-rust-1.100`) restores compiled
  dependencies before `COPY backend/ .`, cutting rebuild times when only
  `src/` changes. Only worth it once rebuild frequency justifies it; the
  plain build above is ~1 minute.

## 3. `.dockerignore` (you create this file — at the repo root)

```
# .dockerignore — build context is the repo root (backend/tests/04-deployment.md)
target/
frontend/
.git/
```

- `target/` — never ship the (multi-GB) local build dir into the context;
  `COPY backend/ .` would copy it otherwise and bust the cache.
- `frontend/` — the backend image does not need the Expo app; the context
  stays small and the image cannot accidentally depend on frontend files.
- `.git/` — no history, no VCS metadata in the image.

## 4. Environment variables (exactly what `src/main.rs` reads)

Read from `src/main.rs` — these are the **actual** defaults:

| Variable | Default | Type / behaviour | Purpose |
|---|---|---|---|
| `RECHESS_HOST` | `0.0.0.0` | `String`; invalid host:port only fails at bind time | Bind interface. `0.0.0.0` listens on all interfaces (required in a container); use `127.0.0.1` for a local-only dev server |
| `PORT` | `3000` | parsed as `u16`; **unparsable or missing falls back to 3000** | Bind port. Platforms inject this (e.g. `PORT=8080`) — do not hardcode in the platform config |
| `RUST_LOG` | `info` | `tracing_subscriber` `EnvFilter` | Log verbosity. Examples: `info`, `debug`, `rechess_backend=debug,tower_http=info` |

Logs are structured (`tracing`), one line per event, e.g. verified on boot:
`2026-11-27T10:21:59.836660Z  INFO rechess_backend: Rechess backend started —
health endpoint at GET /health address=127.0.0.1:3777`.
Rules (binding, from the master plan): logs may contain room codes and
command IDs; **never** a `player_token` or any secret. Section §7 step 5 is
the check.

## 5. Health check

`GET /health` (mounted by `build_router()` in `src/lib.rs`, owner B4):

```bash
curl -i https://<host>/health
# HTTP/1.1 200 OK
# content-type: application/json
# content-length: 43
#
# {"service":"rechess-backend","status":"ok"}
```

- Exact body (43 bytes): `{"service":"rechess-backend","status":"ok"}` —
  keys sort alphabetically in the JSON serialization; treat the body as
  opaque "200 + `status: ok`" in probes, not string-equal.
- No auth, no rate limit, no room state in the response — safe for any
  external prober.
- It reports **process readiness** (bind + router up), not room-state health
  — with in-memory rooms there is no deeper state to probe.

## 6. WSS termination (TLS at the reverse proxy)

The backend serves plain HTTP/WS on `PORT`. **TLS is terminated in front** —
never expose the container directly to the internet; browsers/app stores need
`wss://` in production. The WebSocket endpoint is mounted by B3 at `/ws`
(per `src/network/README.md`), so the proxy must forward `/ws` with upgrade
headers and **timeouts longer than the server's heartbeat interval**.
B3's documented heartbeat (`src/network/README.md`): *server Ping every
30 s, close after 2 missed Pongs* — so a healthy idle connection frames at
least every 30 s, and the proxy read timeout must sit comfortably above it
(≥ 60 s floor; 120 s recommended, which tolerates one missed ping).

**Caddy** (automatic HTTPS via Let's Encrypt — preferred for simplicity):

```caddyfile
# /etc/caddy/Caddyfile
chess.example.com {
    reverse_proxy 127.0.0.1:3000
}
```

Caddy proxies WebSocket upgrades transparently — no special `/ws` block
needed. Two rules:

1. Do not shorten the transport timeouts below the heartbeat interval:
   keep the `reverse_proxy` transport defaults (no `read_timeout` override),
   or set them explicitly above the heartbeat interval.
2. If you add compression or buffering layers, flush events immediately —
   chess moves are tiny, latency-sensitive frames; the default streaming
   flush behaviour in Caddy is correct, do not enable aggressive buffering.

**nginx** (if the host already runs it) — the upgrade headers are mandatory:

```nginx
# /etc/nginx/sites-available/chess.example.com
server {
    listen 443 ssl;
    server_name chess.example.com;
    # ssl_certificate / ssl_certificate_key … (Let's Encrypt or platform-managed)

    # WebSocket endpoint: HTTP/1.1 upgrade + long read timeout
    location /ws {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        # nginx's default proxy_read_timeout (60s) kills idle WebSockets;
        # this MUST exceed the server heartbeat interval:
        proxy_read_timeout 120s;
        proxy_send_timeout 120s;
    }

    # health check passthrough
    location /health {
        proxy_pass http://127.0.0.1:3000;
    }
}
```

## 7. Runbook — deploy, verify, rollback

**Deploy (first time)**

1. Confirm green on `main`: the CI checks (03-ci-workflow.md) and your local
   merge-gate checklist (01-merge-gates.md) — clean checkout, committed
   `Cargo.lock`.
2. Build and tag the image from the **repo root**:
   ```bash
   docker build -f backend/Dockerfile -t rechess-backend:$(git rev-parse --short HEAD) .
   ```
3. Smoke the image locally **before** pushing:
   ```bash
   docker run --rm -p 3000:3000 rechess-backend:$(git rev-parse --short HEAD)
   curl -i http://localhost:3000/health          # 200 {"service":"rechess-backend","status":"ok"}
   ```
4. Push to the registry / deploy on the platform (§8) with env vars set
   (`PORT` from the platform; `RECHESS_HOST` default `0.0.0.0` is already
   container-correct; set `RUST_LOG=info`).
5. Point the TLS reverse proxy (§6) or the platform's managed TLS at the
   container port. Verify `wss://` reachability with the raw smoke:
   `websocat wss://<host>/ws` then send the `room.create` message from
   02-integration-suite.md §4 — expect the `room.created` reply.
6. Grep the first minutes of logs for secrets (must be empty):
   ```bash
   docker logs <container> 2>&1 | grep -i "player_token\|token"
   # only hits allowed: none. (Player IDs are fine; tokens never.)
   ```

**Verify after every deploy (the 2-minute checklist)**

| # | Check | Pass condition |
|---|---|---|
| 1 | `curl -i https://<host>/health` | `200` + `"status":"ok"` |
| 2 | WS smoke via `websocat wss://<host>/ws` + `room.create` | `room.created` with room code, own `player_token`, `"color":"white"` |
| 3 | Second client joins + both see `e2-e4` | 02-integration-suite.md §4 steps 2–3 |
| 4 | `docker logs` / platform logs | Structured lines; no tokens; no repeated `INTERNAL` errors |
| 5 | Heartbeat | Connection stays open ≥ 2 min idle (proxy timeout vs heartbeat configured in §6) |

**Rollback**

The image tag is the rollback unit: **redeploy the previous tag** (the tag
is the `git rev-parse --short HEAD` it was built from). Keep at least the
last two tags in the registry. Rollback does **not** restore rooms: they are
in-memory, so a rollback (like any restart) ends all ongoing games — players
see a reconnect failure and start fresh rooms. Announce that with the
rollback, do not pretend state survived.

**Shutdown / restart behaviour (documented limitation)**

- One process, in-memory rooms, no database: **every restart (deploy,
  rollback, crash) clears all rooms.** Clients keep `player_token`s in secure
  storage, but the tokens die with the process — resume after a restart
  fails with `ROOM_NOT_FOUND` / `INVALID_TOKEN` and the client returns to
  the home screen. This is an accepted MVP limitation; state it in every
  demo. Multi-instance scaling + shared state is explicitly out of scope
  (master plan §3.2).
- No graceful drain today (§1). `docker stop` sends SIGTERM; connections drop
  immediately. If a drain window is wanted, B3 adds
  `.with_graceful_shutdown(...)` on the `axum::serve` call in `main.rs`
  before the first production deploy.

**Client connection (how a phone points at the backend)**

- Local: set `EXPO_PUBLIC_API_URL=ws://<LAN-IP>:3000/ws` in the frontend env
  (`frontend/.env`) and load the app in Expo Go via the QR code — phone and
  backend must be on the same LAN.
- Production: `EXPO_PUBLIC_API_URL=wss://<host>/ws`, TLS terminated in front
  (§6) — never point a production client at plain `ws://`.
- The client side (connect, resume, heartbeat) is F3's:
  `frontend/src/services/02-websocket-service.md`.

## 8. Where to deploy (platform)

The brief does **not** name a hosting platform; the master plan (§14)
requires: *one Rust server process behind a TLS-terminating reverse proxy or
a platform that provides HTTPS/WSS*. Any platform that satisfies all of the
following is acceptable — choose as a team, then pin the choice in the root
`README.md`:

- [ ] Runs the Docker image from §2 as **one instance** (no multi-instance
      autoscaling — rooms are in-memory and not shareable).
- [ ] Terminates TLS itself and speaks **WSS** (`wss://<host>/ws`), with
      idle/WebSocket timeouts you can raise above the heartbeat interval.
- [ ] Injects `PORT` (§4) and keeps the container alive (not serverless /
      per-request — a WS connection must survive for the whole game).
- [ ] Exposes an HTTP health probe you can point at `GET /health` (§5).
- [ ] Lets you pull two previous image tags for rollback (§7).

A typical Docker-deploying platform (Render / Fly.io / Railway style) is
configured as: service = Docker image (or the repo with `dockerfilePath:
backend/Dockerfile`), health check path = `/health`, port = injected `PORT`,
instances = exactly 1. In-platform env vars: `RUST_LOG=info` only — everything
else has correct defaults. Before committing the choice, run the §7 verify
checklist against the platform's preview URL, especially the 2-minute idle
WebSocket check (catches aggressive platform-level proxies in front of the
WSS endpoint).

**Release sign-off (before final submission)**

- Coordinate a code freeze window (no merges except release fixes) with all
  owners.
- Re-run the full release acceptance checklist (§7 verify + the platform
  checks above).
- Record known issues in [05-performance.md](05-performance.md) §6.
