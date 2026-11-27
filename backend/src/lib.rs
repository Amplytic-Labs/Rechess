//! Rechess backend — module map and ownership.
//!
//! | Module        | Owner | Purpose                                                            |
//! |---------------|-------|--------------------------------------------------------------------|
//! | [`protocol`]  | B3 (+ F3 on the frontend) | WebSocket wire types (JSON envelope, commands, events, error codes). |
//! | [`game`]      | B1    | `ChessEngine` contract + (later) the shakmaty-backed implementation. |
//! | [`rooms`]     | B2    | `RoomRegistry`/`RoomHandle` contracts + (later) registry & actors. |
//! | [`network`]   | B3    | Axum routes, WebSocket upgrade, connection tasks, dispatching.    |
//!
//! **Contracts vs implementations** — the contract files below are shared
//! interfaces, deliberately created so modules can be developed and tested
//! INDEPENDENTLY, then linked without type drift:
//!
//! * `protocol/` — the full wire types + round-trip tests (runs green today).
//! * `game::contract` — the engine trait; `game::testkit::FakeEngine` lets
//!   B2/B3/B4 test against it before B1's engine exists.
//! * `rooms::contract` — registry/handle traits; `rooms::testkit` fakes them
//!   for B3/B4.
//!
//! Every module folder contains a README with full instructions for its
//! owner. Integration tests live in `backend/tests/` (owner: B4).

pub mod game;
pub mod network;
pub mod protocol;
pub mod rooms;

use axum::{routing::get, Json, Router};

/// Builds the application router.
///
/// `GET /health` is the deployment health endpoint (owner: B4).
/// The WebSocket endpoint will be mounted here by B3 — see `src/network/README.md`.
pub fn build_router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "rechess-backend" }))
}
