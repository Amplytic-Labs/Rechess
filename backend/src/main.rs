//! Rechess backend entry point.
//!
//! The binary only wires configuration, tracing and the router together —
//! all real logic lives in the library modules (see `src/lib.rs`).

use rechess_backend::build_router;

#[tokio::main]
async fn main() {
    // Structured logs; control verbosity with RUST_LOG
    // (e.g. `RUST_LOG=rechess_backend=debug,tower_http=info`).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Runtime configuration comes from the environment, never from code.
    // B4 documents every variable in backend/README.md.
    let host = std::env::var("RECHESS_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr: std::net::SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("valid bind address");

    let app = build_router();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listen address");
    tracing::info!(address = %addr, "Rechess backend started — health endpoint at GET /health");

    axum::serve(listener, app).await.expect("server error");
}
