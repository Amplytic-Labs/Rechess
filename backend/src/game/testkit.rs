//! Test kit for the `game` module: a scriptable fake engine.
//!
//! This is NOT a chess implementation — it exists so B2 (rooms), B3 (network)
//! and B4 (integration tests) can test their modules against the
//! [`ChessEngine`] contract WITHOUT B1's real engine. B1's real
//! implementation must pass the same contract tests (see
//! `src/game/README.md` §Testing).

use super::contract::{
    ChessEngine, GameError, GameResult, GameResultReason, GameSnapshot, GameStatus, MoveRecord,
    MoveRequest, PlayerColor, Promotion,
};

/// Deterministic, scriptable fake engine for tests.
///
/// Behaviour:
/// * `apply_move`/`offer_draw`/`accept_draw`/`resign` return the snapshot set
///   in [`Self::snapshot`], or the queued error if one was pushed.
/// * Every call is recorded (see [`Self::calls`], [`Self::moves`]).
pub struct FakeEngine {
    pub snapshot: GameSnapshot,
    /// Queued errors: each call that can fail pops one error if present.
    pub errors: Vec<GameError>,
    /// Every move request received, in order.
    pub moves: Vec<MoveRequest>,
    /// Lifecycle calls in order ("draw:White", "accept:Black", "resign:White").
    pub calls: Vec<String>,
    /// How many times `reset_for_rematch` was called.
    pub reset_count: usize,
}

impl FakeEngine {
    pub fn new(snapshot: GameSnapshot) -> Self {
        Self {
            snapshot,
            errors: Vec::new(),
            moves: Vec::new(),
            calls: Vec::new(),
            reset_count: 0,
        }
    }

    /// Queue an error for the next fallible call.
    pub fn fail_next_with(&mut self, error: GameError) {
        self.errors.push(error);
    }

    fn next_result(&mut self) -> Result<GameSnapshot, GameError> {
        match self.errors.pop() {
            Some(error) => Err(error),
            None => Ok(self.snapshot.clone()),
        }
    }
}

impl ChessEngine for FakeEngine {
    fn apply_move(&mut self, request: MoveRequest) -> Result<GameSnapshot, GameError> {
        self.moves.push(request);
        self.next_result()
    }

    fn offer_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError> {
        self.calls.push(format!("draw:{by}"));
        self.next_result()
    }

    fn accept_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError> {
        self.calls.push(format!("accept:{by}"));
        self.next_result()
    }

    fn resign(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError> {
        self.calls.push(format!("resign:{by}"));
        self.next_result()
    }

    fn reset_for_rematch(&mut self) {
        self.reset_count += 1;
    }

    fn snapshot(&self) -> GameSnapshot {
        self.snapshot.clone()
    }
}

/// A valid starting-position snapshot for tests (domain flavour).
pub fn starting_snapshot() -> GameSnapshot {
    GameSnapshot {
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
        turn: PlayerColor::White,
        status: GameStatus::InProgress,
        result: None,
        last_move: None,
        history_san: Vec::new(),
        legal_moves: vec![
            "a2a3".into(),
            "a2a4".into(),
            "b2b3".into(),
            "b2b4".into(),
            "c2c3".into(),
            "c2c4".into(),
            "d2d3".into(),
            "d2d4".into(),
            "e2e3".into(),
            "e2e4".into(),
            "f2f3".into(),
            "f2f4".into(),
            "g2g3".into(),
            "g2g4".into(),
            "h2h3".into(),
            "h2h4".into(),
        ],
        draw_offered_by: None,
    }
}

/// A finished snapshot for tests (black won by checkmate after 1. f3? e5 2. g4?? Qh4#).
pub fn foolsmate_snapshot() -> GameSnapshot {
    GameSnapshot {
        fen: "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3".to_string(),
        turn: PlayerColor::White,
        status: GameStatus::Checkmate,
        result: Some(GameResult {
            winner: Some(PlayerColor::Black),
            reason: GameResultReason::Checkmate,
        }),
        last_move: Some(MoveRecord {
            from: "h4".into(),
            to: "h1".into(),
            san: "Qh4#".into(),
        }),
        history_san: vec!["f3".into(), "e5".into(), "g4".into(), "Qh4#".into()],
        legal_moves: Vec::new(),
        draw_offered_by: None,
    }
}

/// Convenience re-export so test code can build a promotion move quickly.
pub fn promotion_move() -> MoveRequest {
    MoveRequest {
        from: "e7".into(),
        to: "e8".into(),
        promotion: Some(Promotion::Queen),
    }
}
