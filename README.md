# Rechess

Real-time 1v1 multiplayer chess — **Rust (axum/tokio) authoritative backend ·
Expo (React Native) mobile client · WebSocket protocol v1**.

No accounts, no sign-up: create a room, share the code, play, reconnect,
rematch. The server is the only authority on game state.

> **Team note (B4):** This repository is the team base. Backend logic and
> frontend screens are intentionally **not implemented** — every module ships
> with precise instructions, contracts and code skeletons in its README so
> each owner implements their part independently, then links without
> conflicts.

## Repository layout & ownership

```
Rechess/
├── backend/                 Rust server (single crate, 3 modules)
│   ├── src/game/            B1 — chess rules wrapper (shakmaty), snapshots
│   ├── src/rooms/           B2 — registry, room actors, seats, tokens, lifecycle
│   ├── src/network/         B3 — axum WebSocket transport (+ src/protocol/, wire types)
│   ├── tests/               B4 — integration tests, CI, Docker, deployment
│   └── src/main.rs          minimal skeleton: health endpoint (runs today)
├── frontend/                Expo SDK 58 mobile app (TypeScript, Expo Router)
│   ├── src/screens/lobby/   F1 — app foundation, home, create/join, waiting room
│   ├── src/components/chess/ F2 — chessboard, pieces, highlights, promotion
│   ├── src/services/        F3 — WebSocket client, stores, types, secure storage
│   ├── src/screens/game/    F4 — game screen, panels, history, results
│   ├── RULES.md             ⚠️ every frontend contributor reads this first
│   └── src/app/             Expo Router routes (thin wrappers over screens)
├── docs/
│   ├── architecture.md      system overview & key decisions
│   └── protocol-v1.md       WebSocket protocol spec (co-owned by B3 + F3)
└── README.md                this file
```

**Roles:** B1 Chess Engine · B2 Rooms · B3 WebSocket/Protocol · B4 QA/Deploy ·
F1 Lobby · F2 Board · F3 Client State · F4 Game Screen. The role briefs from
the project plan apply; each module README translates them into concrete
steps, contracts and test plans.

## Quick start

```bash
# Backend (Rust 1.100 pinned; see backend/rust-toolchain.toml)
cd backend
cargo run               # → http://localhost:3000/health

# Frontend (Node ≥ 22)
cd frontend
npm install             # already done in this base
npm start               # Expo dev server — scan QR with Expo Go

# Point the app at your backend: see frontend/src/services/README.md
```

## Ground rules (whole team)

1. **Work inside your module.** Cross-module or shared-file changes
   (`backend/src/main.rs`, `Cargo.toml`, `package.json`, configs) happen via a
   short PR that the affected owners approve.
2. **Interfaces are contracts.** The engine API (B1→B2), registry/handle API
   (B2→B3), protocol v1 (B3+F3) and the frontend hooks (F3→F1/F2/F4) are
   frozen on Day 1; changes are explicit PRs updating **both sides** plus the
   docs in the same commit.
3. **Frontend:** read [`frontend/RULES.md`](frontend/RULES.md) before writing
   a line of UI — it is binding for humans and AI assistants alike.
4. **Commit lockfiles** (`Cargo.lock`, `package-lock.json`). Never commit
   secrets, tokens or credentials.
5. **CI is green or tracked:** failing tests are never silently disabled;
   known issues live on the B4 list.

## Version policy (pinned on Day 1)

- Rust **1.100.0** (`backend/rust-toolchain.toml`), crates pinned in
  `backend/Cargo.toml` + `Cargo.lock` (axum 0.8.9, tokio 1.54.1, shakmaty
  0.30.3, …)
- Expo **SDK 58** (React Native 0.88, React 19.3), NativeWind **4.2.7**
  (Tailwind 3), Reanimated 4.7.1, zustand 5 — exact versions in
  `frontend/package.json` + `package-lock.json`
- MVP scope, architecture and milestones: `docs/architecture.md`
