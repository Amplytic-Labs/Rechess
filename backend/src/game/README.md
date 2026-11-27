# Module `game` — Owner: **B1** (Chess Engine and Game Logic)

> **Read this file completely before writing any code.**
> It tells you what you are expected to do, how to do it, what the result must
> be, and exactly where the boundary to your teammates' modules is — so that
> nobody steps on anyone else's work.

---

## 1. Your mission

Deliver a small, well-tested Rust module that takes a move request (or a
lifecycle action) and either **rejects it with a typed error** or **returns the
updated authoritative game snapshot**. The server is the only authority on
rules — the frontend never decides whether a move is legal.

You are **not** building WebSockets, rooms, or anything network-related. Those
belong to B2 (rooms) and B3 (network). Your module must be usable by B2 with
no dependency on Axum, sockets, or frontend code.

## 2. What you own / what you don't touch

| You own (`src/game/`) | You do NOT touch |
|---|---|
| The chess rules wrapper around the `shakmaty` crate | `src/rooms/` (B2) |
| Move validation and application | `src/network/` (B3) |
| FEN / SAN / move history / snapshots | `src/main.rs` (only if B3 asks you to expose something) |
| Check, checkmate, stalemate, draw detection | `frontend/` (F-team) |
| Unit tests for chess rules | Wire-format types (`protocol` structs live with B3, see §7) |

If a change you need seems to require touching another module, talk to that
module's owner first — do not edit their files.

## 3. How your module fits the system

```
client move ──▶ B3 network ──▶ B2 room actor ──▶ B1 ChessEngine
                                                │ validates + applies
              broadcast snapshot ◀── B2 ◀────────┘ GameSnapshot (game:: types)
```

- B2's room actor calls your engine **sequentially** from a single task — you
  never need internal locking (no `Mutex` around the board).
- B3 maps your engine output into the wire JSON (see `docs/protocol-v1.md`).
  You never serialize anything yourself unless B3 asks for a helper.

## 4. Facts you need on Day 1 — the `shakmaty` crate

The crate is already pinned in `backend/Cargo.toml`: **`shakmaty = "0.30.3"`**
(released Nov 2026, actively maintained, it is the Lichess rules library).
Your first job is a **proof of concept** that confirms the facts below on your
machine — never trust a wrapper doc blindly, including this one.

> ⚠️ **License note:** `shakmaty` is **GPL-3.0-or-later**. For a university
> assignment this is normally fine (you distribute source). If the team needs a
> permissive license instead, raise it on Day 1 and swap to `cozy-chess` (MIT,
> but fewer features — you would write more SAN/FEN code yourself). Decision
> belongs to the whole team, before you build on it.

Key API surface (verify each with a tiny test):

```rust
use shakmaty::{Chess, Move, Square, fen::Fen, san::San, uci::Uci, CastlingMode};

let mut pos = Chess::default();                          // initial position
let from: Square = "e2".parse().unwrap();
let to: Square = "e4".parse().unwrap();

// Legal moves (20 in the initial position). Move is an enum:
//   Normal { role, from, capture, to, promotion } | EnPassant { from, to } | Castle { king, rook }
let moves = pos.legal_moves();
let m = moves.iter().find(|m| m.from() == Some(from) && m.to() == Some(to)).unwrap();

// SAN — generate BEFORE playing the move (needs the pre-move position for disambiguation)
let san = San::from_move(&pos, m).to_string();            // "e4"

// FEN
let fen = Fen::from_position(&pos, shakmaty::EnPassantMode::Legal).to_string();

// State transitions — play() takes the position BY VALUE
let next = pos.play(m).unwrap();                         // Result<Chess, PlayError>

// Game status
pos.is_check(); pos.is_checkmate(); pos.is_stalemate();
pos.is_insufficient_material(); pos.halfmoves();          // fifty-move counter

// Parse a FEN back into a position (useful for tests)
let pos2: Chess = "rnbqkbnr/... b KQkq - 0 1".parse::<Fen>().unwrap()
    .into_position(CastlingMode::Standard).unwrap();
```

**What `shakmaty` does NOT do** (you must build this yourself in the app layer):

- Threefold-repetition tracking → keep a `Vec<`zobrist hash`>` history and count
  occurrences (`pos.zobrist_hash()`).
- The fifty-move draw rule → check `pos.halfmoves() >= 100`.
- Draw offers / acceptance, resignation, rematch reset — these are
  multiplayer lifecycle operations, not board moves.

## 5. Your interface contract with B2

This is the **proposed** public API. Finalize it with B2 **on Day 1** (post your
structs in the team chat / a PR). Once agreed, treat it as frozen: B2 builds
against it.

```rust
// src/game/engine.rs — proposed shape (implement it yourself)

pub struct ChessEngine { /* position, history, draw-offer state, ... */ }

impl ChessEngine {
    pub fn new_game() -> Self;
    /// Apply a move: from/to squares like "e2"/"e4", optional promotion 'q'|'r'|'b'|'n'.
    pub fn apply_move(&mut self, from: &str, to: &str, promotion: Option<char>) -> Result<GameSnapshot, GameError>;
    pub fn offer_draw(&mut self, by: Side) -> Result<GameSnapshot, GameError>;
    pub fn accept_draw(&mut self, by: Side) -> Result<GameSnapshot, GameError>;
    pub fn resign(&mut self, by: Side) -> Result<GameSnapshot, GameError>;
    pub fn reset_for_rematch(&mut self);                  // new board, cleared history
    pub fn snapshot(&self) -> GameSnapshot;               // current state, no mutation
}

pub struct GameSnapshot { /* your design: fen, turn, status, last_move, san_history, legal_moves, ... */ }
pub enum GameError { IllegalMove, WrongTurn, NotPromotionSquare, UnknownPromotion, InvalidSquare(String), GameAlreadyOver, /* ... */ }
```

Rules for this contract:

- Inputs are **strings from the wire** (`from`, `to`, promotion letters) — you
  validate square syntax and promotion values and return typed errors; never
  panic on bad input.
- `legal_moves` uses UCI-style encoding (`"e2e4"`, `"e7e8q"`) so the frontend
  can highlight destinations — coordinate with B3/F2 on the encoding (see
  `docs/protocol-v1.md` §4.2).
- Keep `shakmaty` types **inside** this module. B2 must be able to compile
  against your public API without knowing shakmaty exists (only re-export
  simple types like a `Side` enum you define).
- Room revision, player credentials, connection state: **not yours** — never
  add them to the engine.

## 6. Suggested module layout (yours to adjust)

```
src/game/
├── mod.rs        // pub mod engine; pub mod snapshot; ... — re-exports the public API
├── engine.rs     // ChessEngine: validation, apply, lifecycle actions
├── snapshot.rs   // GameSnapshot + status/result types
├── errors.rs     // GameError (typed, stable)
└── tests.rs      // or #[cfg(test)] mod tests in each file
```

## 7. How to test your module independently

You do **not** need the server, rooms, or sockets. Unit tests only:

```bash
cd backend
cargo test game::            # run only your module's tests
cargo test game::tests::promotion    # a single test
```

Your test suite must cover (from your brief):

1. Initial position: 20 legal moves; first move `e2-e4` yields the expected FEN.
2. Special moves: castling (both sides, incl. castling-through-check denial),
   en passant (incl. the one-move expiry window), all four promotion choices.
3. Check, checkmate (e.g. fool's mate), stalemate, insufficient material.
4. Wrong-turn moves, illegal moves, moves after game over, invalid square
   strings, promotion given on a non-promotion move → each returns the right
   `GameError` variant.
5. Draw lifecycle: offer → accept ends the game; resign ends it; rematch
   reset returns an initial snapshot.

Use FEN fixtures as the source of truth for positions (write them as consts
in the test module). Keep tests deterministic; B4 will turn some of them into
cross-module regression tests later.

## 8. Integration sequence (from the master plan)

| Day | Milestone |
|---|---|
| 1 | PoC that `shakmaty` parses the initial position + enumerates/applies legal moves; publish your proposed public structs and error cases to B2 |
| 2 | Engine skeleton + basic move tests merged |
| 3–4 | First vertical slice: new game, apply e2-e4, return snapshot |
| 5–6 | Edge-case rules, terminal states, draw/resign behaviour |
| 7–8 | Fix integration defects found in room/WS tests |
| 9–10 | Support B4's regression suite; record crate version + limitations in this README (§9) |

The vertical slice on Day 3–4 is the key coordination point: B2 needs only
`new_game()`, `apply_move()` and `snapshot()` to demonstrate an end-to-end
move with B3 and the frontend.

## 9. Deliverables (definition of done)

- [ ] Compiling `game` module with a small, stable public API usable by B2
- [ ] Unit tests for standard movement and all special rules (§7)
- [ ] Typed error model (`GameError`) — no panics, no stringly-typed errors
- [ ] Snapshot containing: FEN, side to move, game status, last move, SAN history, legal moves
- [ ] A short usage example in this README's place (update §5 with the final signatures)
- [ ] A note recording the exact `shakmaty` version used and which rules you
      implemented yourself (repetition, fifty-move, draw/resign lifecycle)

## 10. Key coordination

- **B2** is your daily contact — the engine/room boundary.
- **B3** owns the wire JSON; when your snapshot fields are mapped to protocol
  messages, agree names with B3, but B3 owns `docs/protocol-v1.md`.
- Raise any limitation you find in `shakmaty` **immediately** — never silently
  approximate a chess rule.
