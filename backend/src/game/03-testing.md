# 03 — The test suite B1 ships

> **⚠️ Read this first: these tests cannot exist yet.** At the time this doc
> set was written there is **no engine implementation** — `src/game/` contains
> only the frozen contract, the testkit fake and these docs, and `cargo test`
> is green with 15 protocol/testkit tests. This document ships the test suite
> **you will write alongside your engine**. As soon as the skeleton from
> [`02-implementation-guide.md`](02-implementation-guide.md) §12 compiles,
> create `backend/tests/game_engine.rs` (and optionally the in-module tests of
> §3 below) and make them pass one by one. TDD works very well here: each
> fixture in this file corresponds to a rule you must implement anyway.

**All tests in this document must be green before your module merges.** This
is B4's merge gate for the engine (see [`04-acceptance.md`](04-acceptance.md)).

The code below assumes the layout from
[`02-implementation-guide.md`](02-implementation-guide.md) §1: `src/game/engine.rs`
defines `pub struct ShakmatyEngine` with inherent `new_game()` and
`from_fen()` constructors, wired via `pub mod engine;` in `mod.rs`. If you
named your type differently, adjust only the imports/helpers — every other
identifier comes from the frozen contract. Import style is modeled on
`backend/tests/testkit_demo.rs` (which compiles and passes today):

```rust
use rechess_backend::game::ChessEngine as _;   // trait methods in scope
use rechess_backend::game::engine::ShakmatyEngine;
use rechess_backend::game::{GameError, GameResult, GameResultReason, GameStatus,
                             MoveRequest, PlayerColor, Promotion};
```

---

## 1. Run commands

```bash
export PATH=/tmp/rust/cargo/bin:$PATH RUSTUP_HOME=/tmp/rust/rustup CARGO_HOME=/tmp/rust/cargo
cd /home/z/my-project/Rechess/backend

cargo test game::                 # unit tests inside src/game/ (e.g. engine.rs #[cfg(test)])
cargo test game::tests::promotion # a single unit test by name
cargo test --test game_engine     # the integration file below (backend/tests/game_engine.rs)
cargo test                        # EVERYTHING — must stay green (15 tests today + yours)
```

* `cargo test game::` is a name filter: it matches tests whose full path
  contains `game::`, i.e. the `#[cfg(test)] mod tests` inside
  `src/game/engine.rs` (their full names are `game::engine::tests::<case>`).
* `cargo test --test game_engine` runs exactly the integration file
  `backend/tests/game_engine.rs` (a separate crate linking `rechess_backend`).
* Keep the whole suite green at every commit — B2/B3/B4 build against `main`,
  and the baseline suite (protocol round-trips + testkit demos) is the
  contract canary: if YOUR change breaks `testkit_demo.rs`, you changed
  contract semantics by accident.

## 2. `backend/tests/game_engine.rs` — the full integration suite

Every expected value below (FENs, SANs, candidate lists) was verified against
shakmaty 0.30.3 by running real code — see the fixture table in
[`02-implementation-guide.md`](02-implementation-guide.md) §14.

```rust
//! Integration tests for B1's chess engine against the FROZEN contract
//! (src/game/contract.rs). Modeled on tests/testkit_demo.rs import style.
//!
//! Covers: snapshot basics, movement, special moves (castling, en passant,
//! promotion), all typed errors, terminal detection (mate, stalemate,
//! insufficient material, fifty-move, threefold), and the lifecycle actions
//! (draw offer/accept/auto-accept, resign, rematch reset, snapshot purity).

use rechess_backend::game::engine::ShakmatyEngine;
// (named import, not `ChessEngine as _` like testkit_demo.rs, because test 14
// writes the type `Box<dyn ChessEngine>` explicitly)
use rechess_backend::game::{
    ChessEngine, GameError, GameResult, GameResultReason, GameStatus, MoveRecord,
    MoveRequest, PlayerColor, Promotion,
};

const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn new() -> ShakmatyEngine {
    ShakmatyEngine::new_game()
}

fn from_fen(fen: &str) -> ShakmatyEngine {
    ShakmatyEngine::from_fen(fen).expect("test fixture FEN must be valid")
}

fn mv(from: &str, to: &str, promotion: Option<Promotion>) -> MoveRequest {
    MoveRequest { from: from.into(), to: to.into(), promotion }
}

/// Expected `result` field for finished games.
fn result(winner: Option<PlayerColor>, reason: GameResultReason) -> Option<GameResult> {
    Some(GameResult { winner, reason })
}

// --- 1. Starting position -------------------------------------------------

#[test]
fn starting_position_snapshot() {
    let engine = new();
    let snap = engine.snapshot();
    assert_eq!(snap.fen, START_FEN);
    assert_eq!(snap.turn, PlayerColor::White);
    assert_eq!(snap.status, GameStatus::InProgress);
    assert_eq!(snap.result, None);
    assert_eq!(snap.last_move, None);
    assert!(snap.history_san.is_empty(), "no moves played yet");
    assert_eq!(snap.legal_moves.len(), 20, "20 legal moves in the initial position");
    assert!(snap.legal_moves.contains(&"e2e4".to_string()));
    assert_eq!(snap.draw_offered_by, None);
}

// --- 2. Applying a move ---------------------------------------------------

#[test]
fn apply_e2e4_updates_snapshot() {
    let mut engine = new();
    let snap = engine.apply_move(mv("e2", "e4", None)).expect("e2e4 is legal");

    assert_eq!(snap.turn, PlayerColor::Black);
    assert_eq!(snap.status, GameStatus::InProgress);
    assert_eq!(snap.result, None);
    assert_eq!(snap.history_san, vec!["e4".to_string()]);
    assert_eq!(snap.last_move, Some(MoveRecord {
        from: "e2".into(), to: "e4".into(), san: "e4".into(),
    }));
    assert!(snap.legal_moves.contains(&"e7e5".to_string()));
    assert_eq!(snap.fen, "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
}

// --- 3. Fool's mate → checkmate --------------------------------------------

#[test]
fn fools_mate_ends_in_checkmate() {
    let mut engine = new();
    for m in [("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")] {
        engine.apply_move(mv(m.0, m.1, None)).expect("fool's mate sequence is legal");
    }
    let snap = engine.snapshot();

    assert_eq!(snap.status, GameStatus::Checkmate);
    assert_eq!(snap.result, result(Some(PlayerColor::Black), GameResultReason::Checkmate));
    assert_eq!(
        snap.history_san,
        vec!["f3", "e5", "g4", "Qh4#"].into_iter().map(String::from).collect::<Vec<_>>()
    );
    assert_eq!(snap.fen, "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
    assert!(snap.legal_moves.is_empty(), "no moves once mated");
    assert_eq!(snap.turn, PlayerColor::White, "mated side is still to move");
}

// --- 4. Castling ------------------------------------------------------------

#[test]
fn castling_is_legal_and_applies() {
    let mut engine = new();
    for m in [
        ("e2", "e4"), ("e7", "e5"),   // 1. e4 e5
        ("g1", "f3"), ("b8", "c6"),   // 2. Nf3 Nc6
        ("f1", "e2"), ("f8", "e7"),   // 3. Be2 Be7
        ("e1", "g1"),                  // 4. O-O
    ] {
        engine.apply_move(mv(m.0, m.1, None)).expect("castling setup is legal");
    }
    let snap = engine.snapshot();

    assert_eq!(snap.history_san.last().map(String::as_str), Some("O-O"));
    assert_eq!(snap.last_move.as_ref().map(|r| r.san.as_str()), Some("O-O"));
    assert_eq!(snap.last_move.as_ref().map(|r| (r.from.as_str(), r.to.as_str())),
               Some(("e1", "g1")), "castling record uses KING squares, not the rook square");
    assert_eq!(snap.fen, "r1bqk1nr/ppppbppp/2n5/4p3/4P3/5N2/PPPPBPPP/RNBQ1RK1 b kq - 5 4");
    assert_eq!(snap.turn, PlayerColor::Black);
    assert!(snap.legal_moves.contains(&"g8f6".to_string()));
}

#[test]
fn castling_blocked_cases() {
    // (a) Squares occupied: from the start, e1g1 is not possible (f1/g1 busy).
    let mut engine = new();
    assert_eq!(engine.apply_move(mv("e1", "g1", None)), Err(GameError::IllegalMove));

    // (b) Rights absent: legal squares, no rights.
    let mut engine = from_fen("4k3/8/8/8/8/8/8/R3K2R w - - 0 1");
    assert_eq!(engine.apply_move(mv("e1", "g1", None)), Err(GameError::IllegalMove));

    // (c) King passes through an attacked square (rook on f2 attacks f1).
    let mut engine = from_fen("4k3/8/8/8/8/8/5r2/R3K2R w KQ - 0 1");
    let snap = engine.snapshot();
    assert_eq!(snap.status, GameStatus::InProgress, "not in check — only the path is attacked");
    assert!(!snap.legal_moves.contains(&"e1g1".to_string()), "O-O through check");
    assert!(snap.legal_moves.contains(&"e1c1".to_string()), "O-O-O still allowed");
    assert_eq!(engine.apply_move(mv("e1", "g1", None)), Err(GameError::IllegalMove));

    // Sanity: with a clean path both flanks work and O-O applies.
    let mut engine = from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    let snap = engine.snapshot();
    assert!(snap.legal_moves.contains(&"e1g1".to_string()));
    assert!(snap.legal_moves.contains(&"e1c1".to_string()));
    let snap = engine.apply_move(mv("e1", "g1", None)).expect("O-O is legal here");
    assert_eq!(snap.history_san, vec!["O-O".to_string()]);
}

// --- 5. En passant ----------------------------------------------------------

#[test]
fn en_passant_capture_and_expiry() {
    // Capture: 1. e4 d5 2. e5 f5 3. exf6 e.p. — the black f5 pawn is removed.
    let mut engine = new();
    for m in [("e2", "e4"), ("d7", "d5"), ("e4", "e5"), ("f7", "f5"), ("e5", "f6")] {
        engine.apply_move(mv(m.0, m.1, None)).expect("en passant setup is legal");
    }
    let snap = engine.snapshot();
    assert_eq!(snap.history_san.last().map(String::as_str), Some("exf6"));
    assert_eq!(snap.fen, "rnbqkbnr/ppp1p1pp/5P2/3p4/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 3");

    // Expiry: the e.p. right lives exactly one ply. 3. Nf3 Nc6 → then e5f6
    // is an illegal diagonal push to an empty square.
    let mut engine = new();
    for m in [("e2", "e4"), ("d7", "d5"), ("e4", "e5"), ("f7", "f5"), ("g1", "f3"), ("b8", "c6")] {
        engine.apply_move(mv(m.0, m.1, None)).expect("setup is legal");
    }
    assert_eq!(engine.apply_move(mv("e5", "f6", None)), Err(GameError::IllegalMove));
}

// --- 6. Promotion -------------------------------------------------------------

#[test]
fn promotion_without_piece_is_rejected() {
    let mut engine = from_fen("7k/4P3/8/8/8/8/8/4K3 w - - 0 1");
    let before = engine.snapshot();
    let err = engine.apply_move(mv("e7", "e8", None)).expect_err("promotion piece required");
    assert_eq!(err, GameError::PromotionRequired);
    assert_eq!(engine.snapshot(), before, "a rejected move changes nothing");
}

#[test]
fn all_four_promotion_pieces_work() {
    for (piece, san_prefix) in [
        (Promotion::Queen, "e8=Q"),
        (Promotion::Rook, "e8=R"),
        (Promotion::Bishop, "e8=B"),
        (Promotion::Knight, "e8=N"),
    ] {
        let mut engine = from_fen("7k/4P3/8/8/8/8/8/4K3 w - - 0 1");
        let snap = engine
            .apply_move(mv("e7", "e8", Some(piece)))
            .expect("all four promotion choices are legal");
        assert!(snap.history_san.last().unwrap().starts_with(san_prefix));
    }
    // Queen promotion was verified end-to-end against shakmaty:
    let mut engine = from_fen("7k/4P3/8/8/8/8/8/4K3 w - - 0 1");
    let snap = engine.apply_move(mv("e7", "e8", Some(Promotion::Queen))).unwrap();
    assert_eq!(snap.history_san, vec!["e8=Q+".to_string()]);
    assert_eq!(snap.fen, "4Q2k/8/8/8/8/8/8/4K3 b - - 0 1");
}

#[test]
fn promotion_piece_on_non_promotion_move_is_rejected() {
    let mut engine = new();
    assert_eq!(
        engine.apply_move(mv("e2", "e4", Some(Promotion::Queen))),
        Err(GameError::UnexpectedPromotion)
    );
}

// --- 7. Typed move errors ------------------------------------------------------

#[test]
fn wrong_side_to_move_is_rejected() {
    let mut engine = new();
    assert_eq!(engine.apply_move(mv("e7", "e5", None)), Err(GameError::WrongTurn));
}

#[test]
fn illegal_move_is_rejected() {
    let mut engine = new();
    assert_eq!(engine.apply_move(mv("e2", "e5", None)), Err(GameError::IllegalMove));
}

#[test]
fn invalid_square_strings_are_rejected() {
    let mut engine = new();
    assert_eq!(
        engine.apply_move(mv("e9", "e4", None)),
        Err(GameError::InvalidSquare { value: "e9".into() })
    );
    assert_eq!(
        engine.apply_move(mv("e2", "z9", None)),
        Err(GameError::InvalidSquare { value: "z9".into() })
    );
    assert_eq!(
        engine.apply_move(mv("E2", "e4", None)),
        Err(GameError::InvalidSquare { value: "E2".into() })
    );
}

#[test]
fn move_after_game_over_is_rejected() {
    // Board-based ending: fool's mate.
    let mut engine = new();
    for m in [("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")] {
        engine.apply_move(mv(m.0, m.1, None)).unwrap();
    }
    // It is White's move and e2e4 passes the turn check — but the game-over
    // precondition fires FIRST (deterministic error whatever the payload).
    assert_eq!(engine.apply_move(mv("e2", "e4", None)), Err(GameError::GameAlreadyOver));
}

// --- 8. Terminal statuses from FEN fixtures -------------------------------------

#[test]
fn stalemate_from_fixture() {
    let mut engine = from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
    let snap = engine.snapshot();
    assert_eq!(snap.status, GameStatus::Stalemate);
    assert_eq!(snap.result, result(None, GameResultReason::Stalemate));
    assert!(snap.legal_moves.is_empty());
    // The engine reports the terminal status on the very first snapshot —
    // no move needed (that is why effective_result() is computed, not stored).
    assert_eq!(engine.apply_move(mv("h8", "h7", None)), Err(GameError::GameAlreadyOver));
}

#[test]
fn insufficient_material_from_fixture() {
    let engine = from_fen("k7/8/8/8/8/8/8/K7 w - - 0 1");
    let snap = engine.snapshot();
    assert_eq!(snap.status, GameStatus::Draw);
    assert_eq!(snap.result, result(None, GameResultReason::InsufficientMaterial));
    assert!(snap.legal_moves.is_empty(), "game over → no legal moves, even though the king could walk");
}

// --- 9. Automatic draw rules ------------------------------------------------------

#[test]
fn fifty_move_rule_auto_draws() {
    // Halfmove clock 99, White to move, one quiet king move available.
    let mut engine = from_fen("k7/8/8/8/8/8/8/K6R w - - 99 100");
    let before = engine.snapshot();
    assert_eq!(before.status, GameStatus::InProgress, "99 halfmoves is not yet a draw");

    let snap = engine.apply_move(mv("a1", "a2", None)).expect("quiet king move");
    assert_eq!(snap.status, GameStatus::Draw);
    assert_eq!(snap.result, result(None, GameResultReason::FiftyMoveRule));
    assert_eq!(snap.fen, "k7/8/8/8/8/8/K7/7R b - - 100 100");
    assert!(snap.legal_moves.is_empty());

    // And the game stays over:
    assert_eq!(engine.apply_move(mv("a8", "a7", None)), Err(GameError::GameAlreadyOver));
}

#[test]
fn threefold_repetition_auto_draws() {
    let mut engine = new();
    for (i, m) in [
        ("g1", "f3"), ("g8", "f6"), ("f3", "g1"), ("f6", "g8"), // 1. Nf3 Nf6 2. Ng1 Ng8
        ("g1", "f3"), ("g8", "f6"), ("f3", "g1"), ("f6", "g8"), // 3. Nf3 Nf6 4. Ng1 Ng8
    ].into_iter().enumerate() {
        let snap = engine.apply_move(mv(m.0, m.1, None)).expect("knight shuffle is legal");
        if i == 3 {
            assert_eq!(snap.status, GameStatus::InProgress,
                "after the first shuffle cycle the start position has occurred twice only");
        }
        if i == 7 {
            assert_eq!(snap.status, GameStatus::Draw);
            assert_eq!(snap.result, result(None, GameResultReason::ThreefoldRepetition));
            assert!(snap.legal_moves.is_empty());
        }
    }
    assert_eq!(
        engine.snapshot().history_san,
        vec!["Nf3", "Nf6", "Ng1", "Ng8", "Nf3", "Nf6", "Ng1", "Ng8"]
            .into_iter().map(String::from).collect::<Vec<_>>()
    );
}

// --- 10. Draw lifecycle ------------------------------------------------------------

#[test]
fn offer_then_accept_ends_game_in_draw() {
    let mut engine = new();
    let snap = engine.offer_draw(PlayerColor::White).expect("offer ok");
    assert_eq!(snap.draw_offered_by, Some(PlayerColor::White));
    assert_eq!(snap.status, GameStatus::InProgress, "an offer alone does not end the game");
    assert_eq!(snap.fen, START_FEN, "an offer does not touch the board");

    let snap = engine.accept_draw(PlayerColor::Black).expect("opponent accepts");
    assert_eq!(snap.status, GameStatus::Draw);
    assert_eq!(snap.result, result(None, GameResultReason::DrawAgreed));
    assert_eq!(snap.draw_offered_by, None, "offer consumed");
    assert_eq!(snap.fen, START_FEN, "agreement does not touch the board");
    assert!(snap.legal_moves.is_empty());
}

#[test]
fn counter_offer_auto_accepts() {
    let mut engine = new();
    engine.offer_draw(PlayerColor::White).expect("White offers");
    // If the OPPONENT already has an active offer, offer_draw ACCEPTS it.
    let snap = engine.offer_draw(PlayerColor::Black).expect("Black's counter-offer accepts");
    assert_eq!(snap.status, GameStatus::Draw);
    assert_eq!(snap.result, result(None, GameResultReason::DrawAgreed));
    assert_eq!(snap.draw_offered_by, None);
}

#[test]
fn lifecycle_actions_after_game_over_are_rejected() {
    let mut engine = new();
    engine.resign(PlayerColor::White).expect("resign");
    assert_eq!(engine.offer_draw(PlayerColor::Black), Err(GameError::GameAlreadyOver));
    assert_eq!(engine.accept_draw(PlayerColor::Black), Err(GameError::GameAlreadyOver));
    assert_eq!(engine.resign(PlayerColor::Black), Err(GameError::GameAlreadyOver));
}

// --- 11. Resignation ------------------------------------------------------------------

#[test]
fn resign_by_white_wins_for_black() {
    let mut engine = new();
    engine.apply_move(mv("e2", "e4", None)).expect("one move first");
    let snap = engine.resign(PlayerColor::White).expect("resign ok");

    assert_eq!(snap.status, GameStatus::Resigned);
    assert_eq!(snap.result, result(Some(PlayerColor::Black), GameResultReason::Resignation));
    assert_eq!(snap.turn, PlayerColor::Black, "resign does not flip the turn");
    assert_eq!(snap.history_san, vec!["e4".to_string()], "resign does not touch history");
    assert_eq!(snap.last_move.as_ref().map(|r| r.san.as_str()), Some("e4"));
    assert_eq!(snap.fen, "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    assert!(snap.legal_moves.is_empty());
}

// --- 12. Rematch reset -------------------------------------------------------------------

#[test]
fn reset_for_rematch_clears_everything() {
    let mut engine = new();
    engine.apply_move(mv("e2", "e4", None)).expect("some play");
    engine.apply_move(mv("e7", "e5", None)).expect("some play");
    engine.offer_draw(PlayerColor::Black).expect("a lingering offer");
    let before = engine.snapshot();
    assert_ne!(before.fen, START_FEN);

    engine.reset_for_rematch();
    let snap = engine.snapshot();

    let fresh = new();
    assert_eq!(snap, fresh.snapshot(), "rematch snapshot equals a brand-new game, exactly");
    assert_eq!(snap.fen, START_FEN);
    assert_eq!(snap.turn, PlayerColor::White);
    assert_eq!(snap.status, GameStatus::InProgress);
    assert_eq!(snap.result, None);
    assert_eq!(snap.last_move, None);
    assert!(snap.history_san.is_empty());
    assert_eq!(snap.legal_moves.len(), 20);
    assert_eq!(snap.draw_offered_by, None);
}

// --- 13. Purity ------------------------------------------------------------------------------

#[test]
fn snapshot_never_mutates_and_failures_leave_state_unchanged() {
    let mut engine = new();
    engine.apply_move(mv("e2", "e4", None)).expect("e2e4");
    let s1 = engine.snapshot();
    let s2 = engine.snapshot();
    assert_eq!(s1, s2, "snapshot() is pure");

    // A rejected move must not consume anything.
    assert_eq!(engine.apply_move(mv("e2", "e4", None)), Err(GameError::IllegalMove));
    assert_eq!(engine.apply_move(mv("e9", "e4", None)), Err(GameError::InvalidSquare { value: "e9".into() }));
    assert_eq!(engine.apply_move(mv("e7", "e5", None)), Err(GameError::WrongTurn));
    assert_eq!(engine.snapshot(), s1, "failed moves change nothing");
}

// --- 14. B2's exact usage pattern ---------------------------------------------------------------

#[test]
fn engine_works_behind_a_dyn_box() {
    // B2's room actor holds `Box<dyn ChessEngine>` — prove the trait is
    // usable exactly that way (dyn-compatibility + Send).
    let mut engine: Box<dyn ChessEngine> = Box::new(ShakmatyEngine::new_game());
    let snap = engine
        .apply_move(mv("e2", "e4", None))
        .expect("trait object call works");
    assert_eq!(snap.turn, PlayerColor::Black);
    let _ = engine.offer_draw(PlayerColor::White).expect("ok");
    engine.reset_for_rematch();
    assert_eq!(engine.snapshot().fen, START_FEN);
}
```

## 3. Optional: in-module unit tests (`src/game/engine.rs`)

The integration suite above already covers the merge gate. Add in-module
tests when you want fast, focused checks while developing (they see private
items, so they can test `effective_result` or the repetition map directly):

```rust
// inside src/game/engine.rs
#[cfg(test)]
mod tests {
    use super::*; // engine internals + contract via the file's use statements

    #[test]
    fn initial_position_has_twenty_legal_moves() {
        let engine = ShakmatyEngine::new_game();
        assert_eq!(engine.legal_moves_uci().len(), 20);
        assert_eq!(engine.position.turn(), shakmaty::Color::White); // internals visible
    }
}
```

Run them with `cargo test game::`. Do not duplicate whole tests in both
places — pick the natural home (board mechanics: integration file; internal
helpers: unit module).

## 4. What is already covered elsewhere (do not re-test)

* `FakeEngine` / fixtures (`starting_snapshot`, `foolsmate_snapshot`) and the
  contract itself are exercised by `backend/tests/testkit_demo.rs` (B4's
  demo, green today) — you do not re-test them, and you never edit
  `testkit.rs`.
* Room-level behavior (revisions, tokens, seats, rematch AGREEMENT between
  two players) is B2's suite — your `reset_for_rematch` test only proves the
  engine resets.

## 5. Coverage checklist (brief → test)

| Brief area | Test(s) above |
|---|---|
| Initial position: 20 legal moves, FEN after e2e4 | `starting_position_snapshot`, `apply_e2e4_updates_snapshot` |
| Normal movement, captures | `apply_e2e4_updates_snapshot`, `en_passant_capture_and_expiry` (plus the SAN history of every sequence) |
| Turn enforcement | `wrong_side_to_move_is_rejected` |
| Check / checkmate | `fools_mate_ends_in_checkmate`, `castling_blocked_cases` (c) asserts InProgress-not-check |
| Stalemate / insufficient material | `stalemate_from_fixture`, `insufficient_material_from_fixture` |
| Castling (legal, occupied, no rights, through check) | `castling_is_legal_and_applies`, `castling_blocked_cases` |
| En passant (capture + one-ply expiry) | `en_passant_capture_and_expiry` |
| Promotion (all four + both mismatch errors) | `promotion_without_piece_is_rejected`, `all_four_promotion_pieces_work`, `promotion_piece_on_non_promotion_move_is_rejected` |
| History/FEN correctness | every sequence's `history_san`/`fen` assertions |
| Errors: invalid square, illegal, wrong turn, game over | `invalid_square_strings_are_rejected`, `illegal_move_is_rejected`, `wrong_side_to_move_is_rejected`, `move_after_game_over_is_rejected`, `lifecycle_actions_after_game_over_are_rejected` |
| Draw offer → accept; auto-accept; resign; rematch | §10/§11/§12 tests |
| Fifty-move / threefold tracking | `fifty_move_rule_auto_draws`, `threefold_repetition_auto_draws` |
| Snapshot purity / no side effects on failure | `snapshot_never_mutates_and_failures_leave_state_unchanged`, `promotion_without_piece_is_rejected` |
| B2 integration shape (dyn box) | `engine_works_behind_a_dyn_box` |

If a test here fails but the engine "seems right", trust the test fixture
over your intuition: every FEN/SAN expectation was machine-verified against
shakmaty 0.30.3. If you believe a fixture is wrong, verify it with a scratch
program first, then fix the doc + test in the same commit and tell B2 (a
changed expectation is a semantic decision, not a typo).
