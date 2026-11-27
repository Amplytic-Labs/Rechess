# Architecture Overview

## System

```
┌──────────────────────┐         WSS (protocol v1)         ┌──────────────────────┐
│  Next.js web client  │ ◀───────────────────────────────▶ │   Rust backend        │
│  (browser, React 19) │                                  │   axum 0.8 · tokio     │
│                      │                                  │   single process       │
│  F1 lobby screens    │                                  │  ┌──────────────────┐  │
│  F2 chessboard       │                                  │  │ B3 network       │  │
│  F3 ws client+store  │                                  │  │  (ws, protocol)  │  │
│  F4 game screen      │                                  │  └───────┬──────────┘  │
└──────────────────────┘                                  │  ┌───────▼──────────┐  │
                                                          │  │ B2 rooms         │  │
                                                          │  │  (actors, seats) │  │
                                                          │  └───────┬──────────┘  │
                                                          │  ┌───────▼──────────┐  │
                                                          │  │ B1 game          │  │
                                                          │  │  (shakmaty)      │  │
                                                          │  └──────────────────┘  │
                                                          └──────────────────────┘
```

## Key decisions (from the master plan, restated for daily use)

1. **Server-authoritative.** The client renders a tentative selection but
   never declares a move legal or a game finished. All validation happens on
   the server (B1 engine, called from B2's room actor).
2. **One room actor per room.** A Tokio task owns the room's state and
   processes commands sequentially — no concurrent writes to a board, no
   `Mutex` held across awaits. Registry locks are short-lived and never held
   across I/O.
3. **Bounded channels everywhere.** Every command and outbound queue has a
   bound; saturation has a defined safe outcome (disconnect the slow client),
   never unbounded memory growth.
4. **Split socket tasks.** Each connection is a reader task (validate → route
   into B2) plus a writer task draining a per-player outbound queue.
5. **Single process, in-memory state.** No database; rooms are lost on
   restart — an accepted MVP limitation, documented by B4.
6. **Anonymous identity.** Guest nicknames only; the private reconnect token
   is a credential stored hashed server-side and only in secure storage on
   the client. Never in route params, logs, or broadcasts.
7. **Protocol v1 with revisions.** Every accepted state change bumps a
   per-room revision; clients send `expected_revision` and reconcile from
   snapshots on `STALE_REVISION`. See `docs/protocol-v1.md`.

## Data flow — one move

```
tap → F2 board (onMove) → F3 sends game.move (id, expected_revision)
  → B3 validates envelope/auth → B2 room actor serializes command
  → B1 apply_move() → GameSnapshot
  → B2 bumps revision, dedups id → B3 broadcasts game.state
  → F3 store applies snapshot atomically → F4/F2 re-render
```

## Module boundaries (who talks to whom)

- B1 ↔ B2: engine API (moves, snapshots, lifecycle actions). B1 knows
  nothing about rooms or sockets.
- B2 ↔ B3: registry + `RoomHandle` + outbound events. B3 never touches
  `RoomState` directly.
- B3 ↔ F3: protocol v1 (JSON). Co-owned spec; type mirrors on both sides.
- F3 ↔ F1/F2/F4: typed hooks/actions + zustand stores. Screens never open
  sockets; F2 never implements rules.

## Out of MVP scope

Accounts/OAuth, matchmaking/rankings, chat/spectators, AI, persistent match
history, database, multi-instance scaling, chess clocks (unless the
assignment requires them).
