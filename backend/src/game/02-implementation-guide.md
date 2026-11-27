# 02 — Implementation guide (shakmaty 0.30.3)

HOW to implement the frozen contract with shakmaty 0.30.3 (already pinned in
`backend/Cargo.toml`; the version was verified against the actual crate source
when this doc was written — every API call in this guide was compiled and run
against shakmaty 0.30.3, and every expected FEN/SAN was printed by it, not
hand-written).

> ⚠️ **License reminder** (from the master plan): shakmaty is
> GPL-3.0-or-later. Fine for an assignment where you distribute source; if the
> team needs permissive, raise it on Day 1 — do not build on it silently.

First run the crate PoC from your brief (Day 1 task): confirm on your machine
that shakmaty parses the initial position and enumerates/applies legal moves.
Use `02-implementation-guide.md` §12's verified table as expected values.

---

## 1. File layout inside `src/game/`

Keep it small — the contract already centralizes the types:

```
src/game/
├── mod.rs           # add: pub mod engine; pub use engine::ShakmatyEngine;
├── contract.rs     # FROZEN — do not touch without a B1+B2 PR
├── testkit.rs       # shared — do not touch without a PR
├── engine.rs        # ShakmatyEngine: the whole implementation + #[cfg(test)]
└── history.rs       # optional: repetition table (only if engine.rs gets long)
```

* One file (`engine.rs`) is genuinely enough — the engine is ~200 lines. Split
  only if it becomes unwieldy.
* `mod.rs` wiring: add `pub mod engine;` and (recommended) re-export the
  concrete type so tests and B2 can write `rechess_backend::game::ShakmatyEngine`:

  ```rust
  pub mod contract;
  pub mod engine;     // ← your addition
  pub mod testkit;

  pub use contract::*;
  pub use engine::ShakmatyEngine;   // ← optional re-export
  ```

* The brief's original proposal (engine.rs / game_state.rs / moves.rs /
  history.rs / result.rs / errors.rs) predates the frozen contract — the types
  now live in `contract.rs`, so most of those files would be empty. Do NOT
  recreate them.

## 2. Holding the position

Hold a **position**, not a bare board: `shakmaty::Chess` (the standard-chess
position type, re-exported at the crate root — the `variant` feature is NOT
needed and NOT enabled). A `Board` alone cannot answer turn/check/legality
questions; `Chess` carries side-to-move, castling rights, ep square, clocks —
everything `Position` needs.

```rust
use shakmaty::{Chess, EnPassantMode, CastlingMode, Move, Position, Role, Square};
use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::uci::UciMove;
use shakmaty::zobrist::Zobrist64;
```

`Chess::default()` is the initial position; `pos.turn()` is shakmaty's
`Color` (`White`/`Black`) — map to the contract's `PlayerColor` (a two-line
match or `From` impl).

## 3. Parsing and validating squares

```rust
let from: Square = match request.from.parse() {
    Ok(sq) => sq,
    Err(_) => return Err(GameError::InvalidSquare { value: request.from.clone() }),
};
let to: Square = match request.to.parse() { /* same, value: request.to.clone() */ };
```

* `Square`'s `FromStr` accepts exactly lowercase `a-h` + `1-8`, two
  characters. Verified rejections: `"e9"`, `"E2"`, `"e2x"`, `"e"` (→
  `InvalidSquare`). Do not uppercase-normalize — the wire sends lowercase;
  be strict.
* Parse `from` first, `to` second; report the FIRST offender in
  `InvalidSquare { value }`.
* Never feed a raw string into a square conversion without parsing — validate
  first, then let shakmaty see only valid squares.

## 4. Wrong-turn check, then finding the legal move

**Wrong turn** (before any legality work — the piece's color is all you need):

```rust
// shakmaty::Color implements std::ops::Not — !turn() is the opponent:
match self.position.board().piece_at(from) {
    Some(piece) if piece.color == !self.position.turn() =>
        return Err(GameError::WrongTurn),
    _ => {}
}
```

**Finding the legal move(s) between `from`/`to`.** Two equivalent routes —
pick one, use it for both `apply_move` and the `legal_moves` snapshot field so
the encodings can never diverge:

*Route A (recommended): construct a `UciMove` and let shakmaty resolve it.*

```rust
let uci = UciMove::Normal {
    from,
    to,
    promotion: request.promotion.map(promotion_to_role), // Queen→Role::Queen, ...
};
match uci.to_move(&self.position) {       // Result<Move, IllegalUciMoveError>
    Ok(m) => m,
    Err(_) => return Err(GameError::IllegalMove),
}
```

`UciMove::to_move` understands castling (`e1g1` → `Move::Castle { king: e1,
rook: h1 }`) and en passant (diagonal pawn push to an empty square →
`Move::EnPassant`), and it ends with a full `Position::is_legal` check.

*Route B: match against the generated legal-move list:*

```rust
let candidates: Vec<Move> = self.position.legal_moves().into_iter()
    .filter(|m| m.from() == Some(from) && m.to() == to)
    .collect();
```

Note `Move::from() -> Option<Square>` (the `Option` is for Crazyhouse drops)
and `Move::to() -> Square` — **but for `Move::Castle`, `to()` is the ROOK
square** (verified: `e1g1` gives `from() == Some(E1)`, `to() == H1`). See §6.

**Promotion mismatch, from the candidates** (do this BEFORE deciding
legality, per the contract's validation order):

```rust
// candidates = moves matching from/to (from either route):
//   - if any candidate has m.promotion() == Some(_):
//        request.promotion == None        → Err(GameError::PromotionRequired)
//        request.promotion == Some(p)     → select the candidate whose
//                                           promotion() == Some(role_of(p))
//   - else if request.promotion == Some(_) → Err(GameError::UnexpectedPromotion)
//   - else                                 → the single candidate is the move
```

`Move::promotion() -> Option<Role>` (verified) exists on `Move`. A pawn push
to the last rank generates FOUR candidates (`e7e8q` `e7e8r` `e7e8b` `e7e8n` —
verified), so from/to alone never identifies the move when a promotion is
involved; that is exactly why the mismatch cases have their own errors.

## 5. THE pitfall: SAN must be computed BEFORE `play()` — and `play()` consumes the position by value

`Position::play` takes the position **by value**:

```rust
fn play(self, m: Move) -> Result<Self, PlayError<Self>>   // self by value!
```

So the naive code does not compile, or worse, you reorder it wrongly:

```rust
// BROKEN — does not compile: pos was moved:
let san = San::from_move(&pos, m);   // needs the PRE-move position for disambiguation
let next = pos.play(m)?;             // pos consumed HERE
```

**The SAN of a move requires the position BEFORE the move** (for
disambiguation like `Nbd2` and for the `+`/`#` suffix which needs the
position AFTER the move). Two clean solutions:

* **Recommended — one call, no clone.** You validated legality already, so
  "unchecked" play is safe, and this computes SAN from the pre-move position,
  plays the move in place, and suffixes `+`/`#` from the post-move position:

  ```rust
  // &mut self.position, consumed in place:
  let san: String = SanPlus::from_move_and_play_unchecked(&mut self.position, m).to_string();
  // e.g. "e4", "Nf3", "exf6", "O-O", "e8=Q+", "Qh4#"   (verified strings)
  ```

  You want `SanPlus` (not plain `San`) because plain `San::from_move` does NOT
  include the `+`/`#` suffix — and the contract's fixtures require `"Qh4#"`
  (see `testkit.rs` `foolsmate_snapshot()`).

* **Alternative — clone first** (if you dislike the `_unchecked` name; costs
  one position clone per move):

  ```rust
  let san_plus = SanPlus::from_move(pos.clone(), m); // takes pos BY VALUE, plays internally
  let next = pos.play(m).map_err(|_| GameError::IllegalMove)?;
  ```

  (`SanPlus::from_move` also consumes its argument — hence the clone.)

Either way, the rule to remember: **SAN from the pre-move position, suffix
from the post-move position, and `play()` eats `self` — decide the order
before you write the first line of `apply_move`.**

## 6. Zobrist hashing for threefold repetition

shakmaty gives you `Position::zobrist_hash`, but it does NOT track repetition
histories — that is yours (the master plan says so explicitly). The 0.30.3
signature is generic over `ZobristValue`, and the implementations are the
newtype wrappers `Zobrist64(pub u64)` / `Zobrist32` / `Zobrist16` — a raw
`u64` does NOT implement it (type inference will fail with `cannot infer
type of V` if you try). Verified usage:

```rust
use shakmaty::zobrist::Zobrist64;

let hash: Zobrist64 = self.position.zobrist_hash(EnPassantMode::Legal);
let hash: u64 = hash.0;   // pub tuple field; u64::from(hash) also works
```

State and policy (the documented choice — confirm with B2 if they care):

```rust
struct ShakmatyEngine {
    position: Chess,
    history_san: Vec<String>,
    last_move: Option<MoveRecord>,
    repetition: std::collections::HashMap<u64, u8>, // hash → occurrence count
    draw_offered_by: Option<PlayerColor>,
    result: Option<GameResult>,   // None while running; Some once finished
}

// at new_game()/from_fen()/reset_for_rematch():
//   insert the initial hash with count 1 (the starting position counts!)
// after EVERY applied move:
//   hash the NEW position, increment its count
//   if count reaches 3 → game over: Draw / ThreefoldRepetition (winner None)
//   if the move was irreversible → clear the map first, then insert count 1
```

* **Why `EnPassantMode::Legal`**: it excludes ep squares that are set but
  produce no legal capture — positions differing only by such a dead ep
  square hash equal, matching FIDE repetition rules. `EnPassantMode::Always`
  would under-detect legitimate repetitions.
* **Why clearing on irreversible moves is correct**: irreversible means pawn
  move or capture (`Move::is_zeroing()` — verified; castling is NOT zeroing
  but castling rights are part of the hash, so pre/post-castle positions
  never collide anyway). Positions before the last irreversible move can
  never reoccur, so dropping their counts cannot miss a repetition — it only
  bounds the map's size. With full-position zobrist hashes, keeping every
  count is ALSO correct; clearing is the documented choice (memory), not a
  correctness requirement. Pick one and keep the comment.
* The full hash already covers piece placement, side to move, castling rights
  and (legal-mode) ep — you do not need to mix anything in yourself.

## 7. The fifty-move rule — `halfmoves()` (and why `is_game_over()` is NOT enough)

The clock is already inside the position (it is part of the FEN): after every
applied move check

```rust
if self.position.halfmoves() >= 100 {   // 100 halfmoves = 50 moves per side
    // game over: Draw / FiftyMoveRule (winner None)
}
```

**Critical verified fact:** shakmaty's `Position::is_game_over()` does NOT
cover the fifty-move rule or repetition (it only checks variant end, no
legal moves, insufficient material). Proven with
`k7/8/8/8/8/8/8/K6R w - - 99 100` → after `a1a2`, `halfmoves() == 100` and
`is_game_over() == false`. YOU own both checks — never rely on
`is_game_over()` for them.

## 8. Check / checkmate / stalemate / status mapping

All on `Position` (all verified on 0.30.3):

| Question | API |
|---|---|
| Is the side to move in check? | `position.is_check()` / `position.checkers().is_empty()` for the negation |
| Checkmate? | `position.is_checkmate()` (check + no legal moves) |
| Stalemate? | `position.is_stalemate()` (no check + no legal moves) |
| No legal moves at all? | `position.legal_moves().is_empty()` |
| Dead position? | `position.is_insufficient_material()` |

Status computation, in this exact precedence (higher wins; a mating move that
also reaches 100 halfmoves is a CHECKMATE, FIDE 5.1.1):

```text
if result already set (resign / draw-agreed)          → that status (Resigned / Draw)
else if position.is_checkmate()                        → Checkmate  + result(winner = !turn, Checkmate)
else if position.is_stalemate()                        → Stalemate  + result(winner = None,  Stalemate)
else if position.is_insufficient_material()           → Draw       + result(winner = None,  InsufficientMaterial)
else if position.halfmoves() >= 100                    → Draw       + result(winner = None,  FiftyMoveRule)
else if repetition[hash] >= 3                          → Draw       + result(winner = None,  ThreefoldRepetition)
else if position.is_check()                            → Check      (result None)
else                                                   → InProgress(result None)
```

**Where to compute it:** recommended as a helper `fn effective_result(&self) -> Option<GameResult>` that returns the stored lifecycle `result` if set, otherwise evaluates the board-based rows of the table on the fly. `apply_move`'s game-over precondition, `snapshot_status()` and the `legal_moves`-empty rule all consult `effective_result()` — one source of truth. Lazy evaluation also makes `from_fen()` fixtures report the correct status on the very first `snapshot()` (a stalemate FEN is already over before any move — the tests in 03-testing.md rely on this). (Storing the computed result after each move instead also works, as long as `from_fen` runs the same detection at construction; pick one, don't mix.)

## 9. Assembling the `GameSnapshot`

```rust
fn snapshot(&self) -> GameSnapshot {
    GameSnapshot {
        fen: Fen::from_position(&self.position, EnPassantMode::Legal).to_string(),
        turn: to_player_color(self.position.turn()),
        status: /* §8 precedence via effective_result() */,
        result: self.effective_result(),
        last_move: self.last_move.clone(),
        history_san: self.history_san.clone(),
        legal_moves: if self.effective_result().is_some() {
            Vec::new()   // game over (ANY reason) → empty
        } else {
            self.legal_moves_uci()
        },
        draw_offered_by: self.draw_offered_by,
    }
}

fn legal_moves_uci(&self) -> Vec<String> {
    self.position.legal_moves().iter()
        .map(|m| UciMove::from_standard(*m).to_string())
        .collect()
}
```

* `Fen::from_position(&pos, EnPassantMode::Legal).to_string()` — verified
  output matches the fixtures (e.g. after 1. e4:
  `"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"`). `Fen`
  implements `Display`.
* **Use `UciMove::from_standard(m)`** (which assumes `CastlingMode::Standard`)
  for the UCI strings — verified: castling renders as the king's travel
  `"e1g1"` / `"e1c1"`, promotions as `"e7e8q"` lowercase, en passant as the
  pawn's travel `"e5f6"`.
* **`last_move` squares**: echo the (validated) `request.from` / `request.to`
  strings, and take only `san` from shakmaty. Reason: `Move::to()` on a
  castle returns the ROOK square (`e1g1` → `to() == H1`, verified), which
  would put `"h1"` into `MoveRecord.to` while `legal_moves` says `"e1g1"` —
  an inconsistent snapshot that would confuse F2's highlight. If you prefer
  deriving from the `Move`, use the `UciMove::Normal { from, to, .. }`
  fields of `UciMove::from_standard(m)`, which are the king squares for
  castling.
* `legal_moves` MUST be `Vec::new()` once any result is set (see the
  `foolsmate_snapshot()` fixture).

## 10. Lifecycle actions: only status/result (and the offer flag) change

`offer_draw`, `accept_draw`, `resign` (exact semantics in
[`01-contract.md`](01-contract.md) §5.2–5.4):

* They never touch `position`, `history_san`, `last_move` or the repetition
  map — a resign snapshot's FEN equals the pre-resign FEN byte for byte.
* `resign(by)` → `status: Resigned`, `result: Some(GameResult { winner:
  Some(by.opposite()), reason: Resignation })`.
* Draw agreement (accept or cross-offer auto-accept) → `status: Draw`,
  `result: Some(GameResult { winner: None, reason: DrawAgreed })`,
  `draw_offered_by: None`.
* All three return `Err(GameError::GameAlreadyOver)` when a result is already
  set.
* A successful `apply_move` clears an active draw offer (implicit decline —
  the documented default, see `01-contract.md` §7 point 3).

## 11. `reset_for_rematch`

```rust
fn reset_for_rematch(&mut self) {
    self.position = Chess::default();
    self.history_san.clear();
    self.last_move = None;
    self.draw_offered_by = None;
    self.result = None;
    self.repetition.clear();
    // then insert Chess::default()'s hash with count 1 (§6)
}
```

The resulting snapshot must EQUAL a fresh `new_game()` snapshot (`GameSnapshot`
derives `PartialEq` — the rematch test relies on it), including the FEN
counters `"… w KQkq - 0 1"`.

## 12. Skeleton (struct + one method sketched — the rest is yours)

```rust
//! src/game/engine.rs — B1's shakmaty-backed ChessEngine.

use std::collections::HashMap;

use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::uci::UciMove;
use shakmaty::zobrist::Zobrist64;
use shakmaty::{CastlingMode, Chess, EnPassantMode, Move, Position, Role, Square};

use super::contract::{
    ChessEngine, GameError, GameResult, GameResultReason, GameSnapshot, GameStatus,
    MoveRecord, MoveRequest, PlayerColor, Promotion,
};

/// shakmaty-backed engine. One instance = one game = one room (B2 owns it
/// inside the room actor; calls are sequential, no locking).
pub struct ShakmatyEngine {
    position: Chess,
    history_san: Vec<String>,
    last_move: Option<MoveRecord>,
    /// zobrist hash → occurrence count (threefold repetition).
    repetition: HashMap<u64, u8>,
    draw_offered_by: Option<PlayerColor>,
    /// Set when the game ended by a LIFECYCLE action (resign / draw agreed).
    /// Board-based endings are computed on the fly — see §8.
    result: Option<GameResult>,
}

impl ShakmatyEngine {
    /// THE constructor B2 uses.
    pub fn new_game() -> Self {
        Self {
            position: Chess::default(),
            history_san: Vec::new(),
            last_move: None,
            repetition: HashMap::new(), // TODO: insert initial hash with count 1
            draw_offered_by: None,
            result: None,
        }
    }

    /// Optional convenience for tests/debugging (see 03-testing.md). Not
    /// used by B2 in production.
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let position: Chess = fen
            .parse::<Fen>()
            .map_err(|e| e.to_string())?
            .into_position(CastlingMode::Standard)
            .map_err(|e| e.to_string())?;
        let _ = position; // TODO: build Self (remember: insert initial hash)
        unimplemented!("TODO")
    }

    fn snapshot_status(&self) -> GameStatus {
        // TODO: the §8 precedence table, via effective_result()
        unimplemented!("TODO")
    }

    fn effective_result(&self) -> Option<GameResult> {
        // TODO: stored lifecycle result, else compute the board-based rows
        // of the §8 table (checkmate / stalemate / insufficient material /
        // fifty-move / threefold) — one source of truth for "game over".
        unimplemented!("TODO")
    }

    fn legal_moves_uci(&self) -> Vec<String> {
        // TODO: UciMove::from_standard(m).to_string() for each legal move
        unimplemented!("TODO")
    }
}

impl ChessEngine for ShakmatyEngine {
    fn apply_move(&mut self, request: MoveRequest) -> Result<GameSnapshot, GameError> {
        // 1. TODO: effective_result().is_some() → Err(GameError::GameAlreadyOver)
        // 2. TODO: parse request.from / request.to (§3) → InvalidSquare
        // 3. TODO: piece on `from` is opponent's → Err(GameError::WrongTurn) (§4)
        // 4. TODO: candidates from/to; promotion mismatch →
        //          PromotionRequired / UnexpectedPromotion (§4)
        // 5. TODO: no candidate → Err(GameError::IllegalMove) (§4)
        // 6. TODO: SanPlus::from_move_and_play_unchecked(&mut self.position, m)
        //          (§5 — SAN BEFORE the position changes)
        //    TODO: update history_san / last_move / repetition (§6)
        //    TODO: clear draw_offered_by (implicit decline)
        //    (board-based endings need no storage: effective_result()
        //     computes them from the position + repetition map, §8)
        // 7. TODO: Ok(self.snapshot())
        unimplemented!("TODO")
    }

    fn offer_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError> {
        // TODO: GameAlreadyOver; opponent offered → auto-accept (DrawAgreed);
        //       else set draw_offered_by = Some(by), Ok(self.snapshot())
        unimplemented!("TODO")
    }

    fn accept_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError> {
        // TODO: GameAlreadyOver; opponent offer active → DrawAgreed;
        //       else Ok(self.snapshot()) unchanged (open point 01 §7)
        unimplemented!("TODO")
    }

    fn resign(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError> {
        // TODO: GameAlreadyOver; else Resigned + winner Some(by.opposite())
        unimplemented!("TODO")
    }

    fn reset_for_rematch(&mut self) {
        // TODO: §11 — everything back to a fresh game
        unimplemented!("TODO")
    }

    fn snapshot(&self) -> GameSnapshot {
        // TODO: §9 — build fresh, never mutate
        unimplemented!("TODO")
    }
}
```

Also remember `to_player_color(shakmaty::Color)` and
`promotion_to_role(Promotion) -> Role` helpers (two-line matches).

## 13. Pitfalls checklist (all of these have a test in 03-testing.md)

1. **`CastlingMode::Standard` everywhere.** Always parse FENs with
   `into_position(CastlingMode::Standard)` (or
   `Chess::from_setup(setup, CastlingMode::Standard)`). `Chess960` mode would
   make `e1g1` a rook-square-based castling notation and break the UCI
   round-trip.
2. **En-passant legality is one ply only**: shakmaty enforces it (verified:
   after 1. e4 d5 2. e5 f5 3. Nf3 Nc6, white's `e5f6` is illegal), but make
   sure you resolve the request through `to_move`/candidate matching so the
   rule is actually exercised — and test the expiry explicitly.
3. **En-passant mode in FENs you EMIT**: use `EnPassantMode::Legal`; a
   naive `Always` would print `-`-less FENs with dead ep squares that some
   clients reject.
4. **SAN-before-play ordering** (§5) and **`SanPlus`, not `San`** — you need
   the `#` in `"Qh4#"`.
5. **`Move::to()` on castling is the rook square** (§9) — echo the request
   strings for `MoveRecord` instead.
6. **`zobrist_hash` takes `EnPassantMode` and returns a `Zobrist64` newtype**,
   not a plain `u64` (§6).
7. **`is_game_over()` misses fifty-move and repetition** (§7).
8. **A rejected move must leave the engine byte-for-byte unchanged** — run
   ALL validations before the first mutation (§5.1 of 01-contract.md).
9. **`legal_moves` empty once any result is set** — including
   `Resigned`/`Draw`, where the board still has moves.
10. **Insert the starting position into the repetition map** — the threefold
    test triggers on the 8th ply of a knight shuffle only because the
    initial hash was counted at construction.

## 14. Verified fixture table (print-shakmaty-verified — use them in tests)

Expected values verified against shakmaty 0.30.3 by running real code:

| Scenario | Verified output |
|---|---|
| `Chess::default()` SAN of `e2e4` / after-FEN | `"e4"` / `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1` |
| Fool's mate `f2f3 e7e5 g2g4 d8h4` | SANs `["f3", "e5", "g4", "Qh4#"]`; FEN `rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3`; `is_checkmate() == true` (identical to `testkit::foolsmate_snapshot()`) |
| Castling game `e2e4 e7e5 g1f3 b8c6 f1e2 f8e7 e1g1` | SANs `["e4", "e5", "Nf3", "Nc6", "Be2", "Be7", "O-O"]`; FEN `r1bqk1nr/ppppbppp/2n5/4p3/4P3/5N2/PPPPBPPP/RNBQ1RK1 b kq - 5 4`; black `g8f6` legal |
| Crafted castle FEN `4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1` | both `e1g1` and `e1c1` legal; after `e1g1`: SAN `"O-O"`, FEN `4k3/8/8/8/8/8/8/R4RK1 b - - 1 1` |
| Rights-stripped `4k3/8/8/8/8/8/8/R3K2R w - - 0 1` | no castling at all |
| Castle-through-check `4k3/8/8/8/8/8/5r2/R3K2R w KQ - 0 1` | not in check; `e1g1` illegal, `e1c1` legal |
| En passant `e2e4 d7d5 e4e5 f7f5 e5f6` | SANs `["e4", "d5", "e5", "f5", "exf6"]`; FEN `rnbqkbnr/ppp1p1pp/5P2/3p4/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 3` |
| En-passant expiry `e2e4 d7d5 e4e5 f7f5 g1f3 b8c6` then `e5f6` | illegal (`IllegalUciMoveError`) |
| Promotion FEN `7k/4P3/8/8/8/8/8/4K3 w - - 0 1` | candidates `["e7e8q", "e7e8r", "e7e8b", "e7e8n", …king moves]`; after `e7e8q`: SAN `"e8=Q+"`, FEN `4Q2k/8/8/8/8/8/8/4K3 b - - 0 1` |
| Stalemate FEN `7k/5Q2/6K1/8/8/8/8/8 b - - 0 1` | `is_check() == false`, `is_stalemate() == true`, 0 legal moves |
| Bare kings `k7/8/8/8/8/8/8/K7 w - - 0 1` | `is_insufficient_material() == true` |
| Fifty-move FEN `k7/8/8/8/8/8/8/K6R w - - 99 100` | `halfmoves() == 99`, `is_game_over() == false`, `a1a2` legal; after `a1a2`: `halfmoves() == 100`, `is_game_over() == false`, black has 3 legal moves (not stalemate) |
| Threefold shuffle `g1f3 g8f6 f3g1 f6g8 g1f3 g8f6 f3g1 f6g8` | SANs `["Nf3", "Nf6", "Ng1", "Ng8", "Nf3", "Nf6", "Ng1", "Ng8"]`; initial-position hash count reaches 3 on ply 8; `is_game_over() == false`, `halfmoves() == 8` (only YOUR repetition logic ends this game) |
| Square parsing | `"e9"`, `"E2"`, `"e2x"`, `"e"` all fail to parse; `"e2"` succeeds |
| Wrong-turn probe | on the start position, `piece_at(e7) == Some(Black pawn)` while `turn() == White` |
