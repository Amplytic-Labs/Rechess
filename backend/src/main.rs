//! Rechess backend entry point.
//!
//! This is the minimal starting skeleton: it only wires the modules together
//! and serves a health endpoint. Everything else is built by the module owners
//! following the README in their module folder:
//!
//! * `src/game/`    — B1 (chess rules wrapper)
//! * `src/rooms/`   — B2 (room registry, actors, lifecycle)
//! * `src/network/` — B3 (WebSocket transport + protocol)
//!
//! B4 owns integration tests (`tests/`), CI and deployment (see `tests/README.md`).

mod game;
mod network;
mod rooms;

use axum::{routing::get, Json, Router};

#[tokio::main]
async fn main() {
    // Structured logs; control verbosity with RUST_LOG
    // (e.g. `RUST_LOG=rechess_backend=debug,tower_http=info`).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Runtime configuration comes from the environment, never from code.
    // B4 documents every variable in backend/README.md.
    let host = std::env::var("RECHESS_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr: std::net::SocketAddr = format!("{host}:{port}").parse().expect("valid bind address");

    let app: Router = build_router();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listen address");
    tracing::info!(address = %addr, "Rechess backend started — health endpoint at GET /health");

    axum::serve(listener, app).await.expect("server error");
}

fn build_router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "rechess-backend" }))
}
