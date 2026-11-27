//! Module: game — owner **B1** (Chess Engine and Game Logic).
//!
//! * `contract.rs` — the FROZEN public API (domain types + `ChessEngine`
//!   trait). Read it first: B2 builds against it, and changes go through a
//!   B1+B2 PR.
//! * `testkit.rs` — scriptable `FakeEngine` so B2/B3/B4 can test against the
//!   contract before your real engine lands.
//!
//! Read `src/game/README.md` in this folder before writing any code here.

pub mod contract;
pub mod testkit;

pub use contract::*;
