# 05 — Performance benchmark plan

> **B4 doc set, part 5.** What we measure, how, and where the results are
> recorded. Owner: B4.
>
> **Guiding rule (your brief, §5):** *"Do not describe Rust as
> memory-efficient based only on language choice. Measure this implementation
> and report the test conditions. The initial target is a repeatable
> benchmark workload, not a predeclared performance promise."* Nothing in
> this document claims a latency or memory number; every number comes from a
> recorded run of the harness below.

---

## 1. The benchmark workload (the brief's numbers)

**50 simultaneous rooms / up to 100 connected players** — the initial
assignment benchmark workload (brief §2 and scenario table; master-plan
acceptance: *"At least 50 simultaneous rooms (up to 100 connected players)
are exercised; RAM, CPU, connection count and move latency are recorded"*).

Concretely, one benchmark run:

| Parameter | Value |
|---|---|
| Rooms | 50 (all `active`, one game each) |
| Players | 100 connected WebSockets (2 per room, White + Black) |
| Game script | deterministic move sequence per room (see §3) |
| Duration | fixed wall-clock: until every room finishes its scripted game, then a 5-minute idle-connected soak |
| Reconnects | 10 % of clients (10) disconnect and resume mid-game during the churn phase |
| Server | `cargo run --release` binary (release profile: thin LTO, stripped) |

## 2. What we measure

Everything in the brief's measurement table is recorded per phase (idle
baseline, connected, 10 rooms, 50 rooms, under burst, after cleanup):

| Measure | Metric | How captured |
|---|---|---|
| Memory | RSS at: process startup · idle (no clients) · 100 connected players in 50 rooms · 10 rooms · **after room cleanup** | `VmRSS` from `/proc/$(pidof rechess-backend)/status` sampled every 5 s; `docker stats` when containerized |
| CPU | idle CPU · CPU under move bursts · CPU under connection churn | `/proc/<pid>/stat` utime+stime deltas over the sampling interval (or `docker stats`) |
| Connections | active WebSockets · successful reconnects | client-side counter + server log connect/disconnect lines |
| Latency | **client command send → authoritative state received** (a `game.move` sent until the matching `game.state` frame is received by that client), **median and p95** | driver timestamps each send/recv (client-side clock, monotonic) |
| Errors | rejected commands · socket drops · queue-full events · server errors (`INTERNAL`) | driver-side counters + `docker logs` / server log grep |
| Environment | CPU model · RAM · OS · container limits · Rust version · release/debug build · test tool + version · run duration | recorded in the run header (§5 template) |

Sampling cadence: every 5 s from process start until shutdown; keep the raw
series (CSV) and the phase summary (table).

## 3. How to measure — the load harness

Three tiers, use (a) for the recorded benchmark, (c) for quick manual checks:

**(a) The load driver (recommended: a small Rust binary, `tests/`-adjacent
harness).** Reuses the crate's own fixtures (exact JSON from
`docs/protocol-v1.md`) and needs no extra runtime. Sketch — this is the
driver design, you write it as a dev tool when B3's `/ws` lands:

```text
load_driver <ws_url> [--rooms 50] [--reconnects 10] [--sample-every 5s]
├── phase 1  idle baseline          connect nothing, sample RSS/CPU 60 s
├── phase 2  connect 100 players    create 50 rooms (host WS), join 50 (guest WS)
│                                  → sample "idle connected" 60 s
├── phase 3  10-room step           hold 10 rooms live, sample
├── phase 4  50-room move burst     every room plays its scripted game:
│                                  driver sends e.g. 10 moves/s/room,
│                                  timestamps send→game.state per client
├── phase 5  churn/reconnects       10 clients drop + resume via token
│                                  (successful_reconnects counter)
└── phase 6  cleanup                leave all rooms, wait for expiry,
                                   sample until RSS stabilizes ("after cleanup")
```

Driver implementation notes:

- One tokio task per WebSocket (`tokio-tungstenite`), bounded mpsc for stats;
  each client plays a **fixed, deterministic** move script (e.g. a repeated
  40-move opening sequence; the fool's-mate fixture from
  `game::testkit::foolsmate_snapshot()` is the end-of-game check). No RNG in
  the move order, so runs are comparable.
- Latency = per-client `Instant::now()` from sending the `game.move` envelope
  to receiving the `game.state` with the matching revision; aggregate
  median/p95 at the end. Report p95 per phase, not per client.
- Watch `expected_revision` discipline: each client tracks its last seen
  revision, so the driver exercises the same stale-revision/retry path a
  real client has.

**(b) k6 alternative** (if the team prefers a standard tool): k6 ≥ 0.5x with
the WebSocket extension (`k6/x/ws` or jslib ws). Same phases as SCENARIOS;
thresholds stay **informational** (`thresholds: { ws_connecting: [...] }`
for trend data only) until the first baseline is recorded. Note: k6 is not
required by the brief — the Rust driver (a) is preferred because it shares
the protocol fixtures with the backend's own tests.

**(c) websocat manual smoke** (not a benchmark): quick per-room check against
a running server —

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend
cargo run --release                 # verified: builds in ~1m 07s
websocat ws://127.0.0.1:3000/ws     # [after B3 lands] send room.create, time the reply by eye
```

**While the run is in progress**, from a second shell:

```bash
PID=$(pidof rechess-backend)
watch -n 5 "grep VmRSS /proc/$PID/status; ps -o %cpu= -p $PID"   # memory + CPU trend
```

## 4. Acceptance thresholds

Faithful to the brief: the **workload and its recording** are the acceptance
criteria, not predeclared performance numbers (master plan §13.3: the 50-room
target is a benchmark workload, not a claim of a latency/memory threshold).

The benchmark run is **PASS** when:

1. All 50 rooms reach `game.finished` with their scripted game — no room
   stuck, no client crash, server process alive the whole run.
2. Zero `INTERNAL` errors and zero queue-full events at this workload
   (rejected commands from *intentional* stale/illegal test moves are
   expected and counted separately, not errors).
3. RSS after cleanup returns to the idle-connected ballpark — no monotonic
   growth across two consecutive runs (that is the leak check; record both
   runs).
4. Median and p95 move latency for the 50-room phase are computed and
   recorded, and the full environment (CPU, RAM, OS, container limits, Rust
   version, build profile, tool, duration) is written into the results table.
5. The run is repeatable: the same command produces the report with the same
   workload shape (deterministic scripts).

**Numeric targets are set AFTER the first baseline run**, as a team decision,
and then recorded here (e.g. "p95 move latency at 50 rooms: baseline X ms,
agreed ceiling Y ms"). Until then this section deliberately contains no
invented threshold.

## 5. Results template (record every run here)

Copy the block below per run; one row per phase in the second table. Never
edit a past run — a rerun gets a new block.

```markdown
### Run <n> — <date>
- commit: <git rev-parse --short HEAD> · branch: <name>
- hardware: <CPU model · cores · RAM> · OS: <distro + kernel> (host) / <image> (container)
- limits: <docker --memory/--cpus if any> · Rust: 1.100.0 · profile: release (thin LTO, stripped)
- tool: <load_driver | k6 + version> · duration: <mm:ss> · rooms: 50 · players: 100
```

| Phase | Active WS | Rooms | RSS (MB) | CPU % (avg) | Move latency median (ms) | p95 (ms) | Errors (rejected / drops / queue-full / INTERNAL) |
|---|---|---|---|---|---|---|---|
| Startup idle | 0 | 0 | | | — | — | |
| Idle connected | 100 | 50 | | | — | — | |
| 10 rooms | 20 | 10 | | | | | |
| 50 rooms burst | 100 | 50 | | | | | |
| Churn/reconnects | 100±10 | 50 | | | | | |
| After cleanup | 0 | 0 | | | — | — | |

Notes: <anomalies, reconnects succeeded/failed, anything that needs follow-up>

## 6. Known-issues list (maintained from the first run)

- <date> — <issue> — <owner routed to> — <status>
- Server restart wipes rooms (in-memory limitation, 04-deployment.md §8) —
  affects any benchmark phase that restarts the process; never count a
  post-restart room as survived.
