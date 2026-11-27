//! FROZEN public contract of the `game` module (draft for Day-1 freeze).
//!
//! **Owner: B1** implements the [`ChessEngine`] trait (wrapping `shakmaty`)
//! inside this module. **B2** consumes it through these types only.
//!
//! These are DOMAIN types, deliberately separate from the wire types in
//! `crate::protocol` (B2 maps between them — see `src/rooms/README.md`).
//! Keeping them separate means B1's module can be compiled and unit-tested
//! with zero knowledge of protocol, rooms, sockets or Axum.
//!
//! **Change rule:** signature or semantic changes here are a PR agreed by
//! B1 + B2 (B3 only if the wire mirror is affected).

use std::fmt;

/// Seat color. Mirrored on the wire as `"white" | "black"` (protocol types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerColor {
    White,
    Black,
}

impl PlayerColor {
    pub fn opposite(self) -> Self {
        match self {
            PlayerColor::White => PlayerColor::Black,
            PlayerColor::Black => PlayerColor::White,
        }
    }
}

impl fmt::Display for PlayerColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerColor::White => write!(f, "White"),
            PlayerColor::Black => write!(f, "Black"),
        }
    }
}

/// Pawn promotion choice. Wire values: `"q" | "r" | "b" | "n"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Promotion {
    Queen,
    Rook,
    Bishop,
    Knight,
}

/// Game phase (independent of the room status owned by B2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    InProgress,
    Check,
    Checkmate,
    Stalemate,
    Draw,
    Resigned,
}

/// Why the game ended. Mirrors the wire `GameResultReason` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResultReason {
    Checkmate,
    Stalemate,
    Resignation,
    DrawAgreed,
    InsufficientMaterial,
    ThreefoldRepetition,
    FiftyMoveRule,
}

/// Final result. `winner` is `None` for draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameResult {
    pub winner: Option<PlayerColor>,
    pub reason: GameResultReason,
}

/// One move for display (last-move highlight + history list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveRecord {
    /// e.g. `"e2"`.
    pub from: String,
    /// e.g. `"e4"`.
    pub to: String,
    /// Standard algebraic notation, e.g. `"e4"`, `"Nf3"`, `"exd5"`, `"e8=Q"`.
    pub san: String,
}

/// A move request as submitted from the wire (B2 passes it through).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveRequest {
    pub from: String,
    pub to: String,
    pub promotion: Option<Promotion>,
}

/// The authoritative engine snapshot. B2 assembles the wire `GameSnapshot`
/// (protocol) from this plus room data (players, room code, room status).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSnapshot {
    pub fen: String,
    pub turn: PlayerColor,
    pub status: GameStatus,
    /// Present only when the game is finished.
    pub result: Option<GameResult>,
    pub last_move: Option<MoveRecord>,
    /// SAN move list, oldest first.
    pub history_san: Vec<String>,
    /// UCI-style legal moves for the side to move: `"e2e4"`, `"e7e8q"`.
    pub legal_moves: Vec<String>,
    /// Color of the player with an active draw offer, if any.
    pub draw_offered_by: Option<PlayerColor>,
}

/// Typed, stable errors. B2 maps these onto protocol `ErrorCode`s
/// (see `src/rooms/README.md` for the mapping table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    /// `from`/`to` is not a valid square string.
    InvalidSquare { value: String },
    /// The move is not legal in the current position.
    IllegalMove,
    /// It is not this side's turn.
    WrongTurn,
    /// A pawn reached the last rank but no promotion piece was given.
    PromotionRequired,
    /// A promotion piece was given for a non-promotion move.
    UnexpectedPromotion,
    /// The game already has a result; no further moves/actions.
    GameAlreadyOver,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::InvalidSquare { value } => write!(f, "invalid square: {value:?}"),
            GameError::IllegalMove => write!(f, "illegal move"),
            GameError::WrongTurn => write!(f, "not this side's turn"),
            GameError::PromotionRequired => write!(f, "promotion piece required"),
            GameError::UnexpectedPromotion => write!(f, "promotion given on a non-promotion move"),
            GameError::GameAlreadyOver => write!(f, "game already over"),
        }
    }
}

impl std::error::Error for GameError {}

/// The engine contract B2 builds against.
///
/// B1 implements this for their shakmaty-backed engine. The engine is pure,
/// synchronous game logic: it is owned by ONE room actor task (B2), which
/// calls it sequentially — no interior locking is required.
///
/// Constructors are deliberately NOT part of the trait (they would break
/// dyn-compatibility): each engine type provides its own `new_game()`
/// constructor, and B2 injects engines via its own factory.
pub trait ChessEngine: Send {
    /// Validate and apply one move; returns the updated snapshot or a
    /// typed error. Must never panic on bad input.
    fn apply_move(&mut self, request: MoveRequest) -> Result<GameSnapshot, GameError>;

    /// Offer a draw. If the OPPONENT already has an active offer, this call
    /// ACCEPTS it and the game ends with `DrawAgreed`.
    fn offer_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;

    /// Accept the opponent's active draw offer.
    fn accept_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;

    /// Resign; the opponent wins by `Resignation`.
    fn resign(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;

    /// Reset to a fresh game for a rematch (clears board, history, draw state).
    fn reset_for_rematch(&mut self);

    /// Current authoritative snapshot (never mutates).
    fn snapshot(&self) -> GameSnapshot;
}
