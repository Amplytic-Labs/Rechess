//! Module: rooms — owner **B2** (Room Management and Game Lifecycle).
//!
//! * `contract.rs` — the FROZEN public API (`RoomRegistry` + `RoomHandle`
//!   traits, commands, outcomes, events). Read it first: B3 builds against
//!   it, and changes go through a B2+B3 PR.
//! * `testkit.rs` — `FakeRegistry`/`FakeHandle` so B3/B4 can test against the
//!   contract before your real registry/actors land.
//!
//! Read `src/rooms/README.md` in this folder before writing any code here.

pub mod contract;
pub mod testkit;

pub use contract::*;
