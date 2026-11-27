# 01 — The frozen contract explained

This document explains [`contract.rs`](contract.rs) piece by piece: every type
with its semantics (what each field means, who reads it), every trait method's
exact behavior including edge cases, the full error model, how B2 consumes
your engine, and the constructor conventions. **Every signature here is copied
from the real file — if this doc and `contract.rs` ever disagree, `contract.rs`
wins and this doc must be fixed in the same PR.**

The types are DOMAIN types, deliberately separate from the wire types in
`crate::protocol`: B2 maps between them (see §8 below). Your module compiles
and is unit-tested with zero knowledge of protocol, rooms, sockets or Axum —
never add a `use crate::protocol::…` or `use crate::rooms::…` inside
`src/game/`.

---

## 1. Types at a glance

| Type | Kind | One-liner |
|---|---|---|
| `PlayerColor` | enum `White \| Black` | Seat color; mirrors wire `"white" \| "black"`. |
| `Promotion` | enum `Queen \| Rook \| Bishop \| Knight` | Pawn promotion choice; wire letters `"q" \| "r" \| "b" \| "n"`. |
| `GameStatus` | enum | Game phase: `InProgress`, `Check`, `Checkmate`, `Stalemate`, `Draw`, `Resigned`. |
| `GameResultReason` | enum | Why the game ended: `Checkmate`, `Stalemate`, `Resignation`, `DrawAgreed`, `InsufficientMaterial`, `ThreefoldRepetition`, `FiftyMoveRule`. |
| `GameResult` | struct | `winner: Option<PlayerColor>` + `reason: GameResultReason`. `winner` is `None` for draws. |
| `MoveRecord` | struct | One move for display: `from`/`to` squares + `san` string. |
| `MoveRequest` | struct | A move as submitted from the wire: `from`, `to`, `promotion: Option<Promotion>`. |
| `GameSnapshot` | struct | The authoritative snapshot B2 assembles the wire payload from. |
| `GameError` | enum | Stable, typed errors B2 maps onto protocol `ErrorCode`s. |
| `ChessEngine` | trait | The engine contract: `apply_move`, `offer_draw`, `accept_draw`, `resign`, `reset_for_rematch`, `snapshot`. |

All types derive `Debug, Clone` (plus `Copy, PartialEq, Eq` where shown in the
source file); none of them derive `Serialize` — serialization is B3's wire
layer, not yours. `GameSnapshot` deriving `PartialEq` is load-bearing for tests
(`assert_eq!(engine.snapshot(), engine.snapshot())`).

---

## 2. The value types

### `PlayerColor`

```rust
pub enum PlayerColor { White, Black }
```

* Who is allowed to move right now (`GameSnapshot::turn`) and who performed a
  lifecycle action (`offer_draw(&mut self, by: PlayerColor)` etc.).
* `PlayerColor::opposite(self) -> Self` is provided by the contract — use it,
  don't reimplement it.
* `Display` prints `"White"` / `"Black"` (capitalized, domain flavor — B2 maps
  to lowercase wire strings).
* Read by: B2 (seat assignment, turn checks on room commands), the frontend
  via the wire mapping.

### `Promotion`

```rust
pub enum Promotion { Queen, Rook, Bishop, Knight }
```

* Arrives already typed inside `MoveRequest::promotion`. The wire letters
  `"q" | "r" | "b" | "n"` are parsed by B3/B2 (`protocol` has its own
  `Promotion` mirror); you never see a raw `char`.
* Your validation job is only the **mismatch** cases (see `apply_move` below):
  a promotion piece missing where one is required, or supplied where none is
  allowed.

### `GameStatus`

```rust
pub enum GameStatus { InProgress, Check, Checkmate, Stalemate, Draw, Resigned }
```

The game *phase*, independent of the room status owned by B2 (`waiting`,
`active`, …). Exact semantics your engine must produce:

| Variant | When `GameSnapshot::status` is this |
|---|---|
| `InProgress` | Game running, side to move NOT in check. |
| `Check` | Game running, side to move IS in check. |
| `Checkmate` | Side to move is in check and has no legal move. `result` is `Some` with `winner: Some(<the other color>)`, `reason: Checkmate`. |
| `Stalemate` | Side to move is NOT in check and has no legal move. `result` is `Some` with `winner: None`, `reason: Stalemate`. |
| `Draw` | Game ended by agreement or rule: `DrawAgreed`, `InsufficientMaterial`, `ThreefoldRepetition` or `FiftyMoveRule`. `winner: None`. |
| `Resigned` | A player resigned; `winner: Some(<opponent of the resigner>)`, `reason: Resignation`. |

Coherence rules (B2's mapping and F4's UI depend on them):

* `result` is `Some(_)` **iff** status is one of `Checkmate`, `Stalemate`,
  `Draw`, `Resigned` (i.e. the game has a final result). `InProgress` and
  `Check` always carry `result: None`.
* When the game is over — for ANY reason — `legal_moves` MUST be empty (no
  further moves exist; see the `foolsmate_snapshot()` fixture in
  [`testkit.rs`](testkit.rs) for the shape).

### `GameResultReason` and `GameResult`

```rust
pub enum GameResultReason { Checkmate, Stalemate, Resignation, DrawAgreed,
                            InsufficientMaterial, ThreefoldRepetition, FiftyMoveRule }

pub struct GameResult { pub winner: Option<PlayerColor>, pub reason: GameResultReason }
```

* Mirrors the wire `GameResultReason` values exactly (`checkmate`, `stalemate`,
  `resignation`, `draw_agreed`, `insufficient_material`, `threefold_repetition`,
  `fifty_move_rule` — see `docs/protocol-v1.md` §4.3).
* `winner` is `None` exactly for the draw reasons (`Stalemate`, `DrawAgreed`,
  `InsufficientMaterial`, `ThreefoldRepetition`, `FiftyMoveRule`) and
  `Some(color)` for `Checkmate` and `Resignation` (the opponent of the player
  who was mated / resigned).

### `MoveRecord`

```rust
pub struct MoveRecord {
    pub from: String,  // e.g. "e2"
    pub to: String,    // e.g. "e4"
    pub san: String,   // Standard algebraic notation, e.g. "e4", "Nf3", "exd5", "e8=Q"
}
```

* One move for display: last-move highlight (`last_move`) and the history list.
* `san` is standard algebraic notation **including check/mate suffixes** —
  see [`testkit.rs`](testkit.rs) `foolsmate_snapshot()`, where the last entry
  of `history_san` is `"Qh4#"`, not `"Qh4"`.
* Square encoding is the wire format (`"e1"`, `"g1"`), not shakmaty enums.
  **Castling pitfall:** for `O-O` the record must carry `from: "e1"`,
  `to: "g1"` (king squares, matching what the client submitted) — NOT the
  rook square `h1`, which is what `shakmaty::Move::to()` returns for castle
  moves. Details in [`02-implementation-guide.md`](02-implementation-guide.md)
  §6.

### `MoveRequest`

```rust
pub struct MoveRequest {
    pub from: String,
    pub to: String,
    pub promotion: Option<Promotion>,
}
```

* A move exactly as submitted from the wire (B2 passes it through after its
  own seat/revision checks — those are NOT yours).
* `from`/`to` are square strings like `"e2"`/`"e4"`: you validate the syntax
  yourself (see `GameError::InvalidSquare`). Shakmaty's square parser accepts
  exactly lowercase `a-h` + `1-8`, and rejects `"e9"`, `"E2"`, `"e2x"`, `"e"`.
* Never trust the strings — validate before any shakmaty call (shakmaty
  parses squares infallibly only after `parse()` succeeds; feeding garbage
  into a `Square` conversion without parsing must never happen).

---

## 3. `GameSnapshot` — field by field

```rust
pub struct GameSnapshot {
    pub fen: String,
    pub turn: PlayerColor,
    pub status: GameStatus,
    pub result: Option<GameResult>,       // Present only when the game is finished.
    pub last_move: Option<MoveRecord>,
    pub history_san: Vec<String>,          // SAN move list, oldest first.
    pub legal_moves: Vec<String>,          // UCI-style: "e2e4", "e7e8q".
    pub draw_offered_by: Option<PlayerColor>, // Color with an active draw offer, if any.
}
```

| Field | Semantics | Who reads it |
|---|---|---|
| `fen` | Full FEN of the current position (6 fields), with the **legal** en-passant convention (`EnPassantMode::Legal`), e.g. `"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"` after 1. e4. Never taken from the client. | B2 → wire `fen`; F2 renders the board from it. |
| `turn` | Side to move **now**. Mirrors the position; it only advances via applied moves — resign/draw do NOT flip it (see `testkit.rs` `foolsmate_snapshot()`: mated White is still `turn: PlayerColor::White`). | B2 turn checks; F4 turn indicator. |
| `status` | See `GameStatus` above. | B2 → wire `game_status`; F4 result banner. |
| `result` | `Some` iff finished. | B2 → wire `result`; `game.finished` event. |
| `last_move` | The most recent applied move, or `None` at game start / after `reset_for_rematch`. Lifecycle actions (draw/resign) do NOT change it. | F2 last-move highlight. |
| `history_san` | Every applied move in SAN, oldest first, including `+`/`#` suffixes. | F4 move list. |
| `legal_moves` | Legal moves for the side to move in UCI-style lowercase strings (`"e2e4"`, `"e7e8q"`, castling as king travel `"e1g1"`/`"e1c1"`). Empty when the game is over. `snapshot()` after a **rejected** move reports the unchanged previous list. | F2 target highlighting. |
| `draw_offered_by` | `Some(color)` while that color's draw offer is active and unanswered; `None` otherwise, including after the offer was consumed (accepted / auto-accepted / offerer's game over) and after `reset_for_rematch`. | F4 draw button state. |

The starting-position shape is exactly what
[`testkit.rs`](testkit.rs) `starting_snapshot()` builds — use it as the
reference: 20 `legal_moves`, empty `history_san`, `result: None`,
`draw_offered_by: None`, `turn: PlayerColor::White`,
`fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`.

---

## 4. `GameError` — full list, when each occurs

```rust
pub enum GameError {
    InvalidSquare { value: String },
    IllegalMove,
    WrongTurn,
    PromotionRequired,
    UnexpectedPromotion,
    GameAlreadyOver,
}
```

| Variant | Occurs when | Note |
|---|---|---|
| `InvalidSquare { value }` | `from` or `to` is not a valid square string (`"e9"`, `"E2"`, `"e2x"`, `"e"`, empty…). `value` carries the offending string. | Validate `from` first, then `to`; report the first offender. |
| `IllegalMove` | Both squares parse and it IS the mover's turn, but no legal move matches `from`/`to` (+ promotion): sliding into check, empty `from` square, blocked path, wrong geometry, expired en-passant, missing castling rights… | The catch-all after the typed pre-checks. |
| `WrongTurn` | The piece on `from` belongs to the opponent of the side to move (e.g. moving a black pawn while `turn` is White). | Detected via piece color on the `from` square, BEFORE legality. |
| `PromotionRequired` | A pawn is moving to its last rank (legal promotion moves from→to exist), but `promotion` is `None`. | Example: white pawn `e7`→`e8` with `promotion: None`. |
| `UnexpectedPromotion` | `promotion` is `Some(_)` but the matched legal move is not a promotion move (e.g. `e2`→`e4` with `promotion: Some(Promotion::Queen)`). | Also covers a promotion piece given for a non-pawn move. |
| `GameAlreadyOver` | Any move or lifecycle action (`apply_move`, `offer_draw`, `accept_draw`, `resign`) is attempted while the game already has a result. | Check this FIRST — before square parsing — so the error is deterministic regardless of the submitted move. |

`GameError` implements `Display` and `std::error::Error`; the messages are
stable strings (`"invalid square: \"e9\""`, `"illegal move"`,
`"not this side's turn"`, `"promotion piece required"`, `"promotion given on a
non-promotion move"`, `"game already over"`). B2 maps the variants onto
protocol `ErrorCode`s (`ILLEGAL_MOVE`, `NOT_YOUR_TURN`, `GAME_FINISHED`, …)
— the mapping table lives with B2/B3, see `docs/protocol-v1.md` §6. You
never build those wire codes yourself.

---

## 5. The `ChessEngine` trait — exact method contracts

```rust
pub trait ChessEngine: Send {
    fn apply_move(&mut self, request: MoveRequest) -> Result<GameSnapshot, GameError>;
    fn offer_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;
    fn accept_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;
    fn resign(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;
    fn reset_for_rematch(&mut self);
    fn snapshot(&self) -> GameSnapshot;
}
```

The engine is **pure, synchronous game logic**: no async, no I/O, no channels,
no clocks, no locks (see §7). **Must never panic on bad input** — every bad
input is a typed `GameError`.

### 5.1 `apply_move` — validation order

Run the checks in this order and return the FIRST error; only a move that
passes everything is applied:

1. **Game over precondition** — if the game already has a result:
   `Err(GameError::GameAlreadyOver)`. *(Checked first so the error is
   deterministic whatever the move strings contain — this is the documented
   resolution of an ordering ambiguity in the brief; confirm with B2, see
   §7.)*
2. **Square validity** — parse `from`, then `to` (`InvalidSquare { value }`
   with the first offending string).
3. **Wrong turn** — if the piece standing on `from` belongs to the opponent
   of the side to move: `Err(GameError::WrongTurn)`. An empty `from` square
   is NOT wrong-turn; it fails later as `IllegalMove`.
4. **Promotion mismatch** — match the legal moves between `from` and `to`:
   * pawn to last rank with `promotion: None` → `Err(GameError::PromotionRequired)`,
   * `promotion: Some(_)` on a non-promotion move → `Err(GameError::UnexpectedPromotion)`.
5. **Legality** — no legal move matches `from`/`to` (+promotion) →
   `Err(GameError::IllegalMove)`.
6. **Apply, then game-over detection** — play the move, update history /
   repetition table / draw-offer state, then compute the new status:
   `Checkmate` → `Stalemate` → `InsufficientMaterial` → `FiftyMoveRule` →
   `ThreefoldRepetition` → `Check` → `InProgress`, in that precedence (mate
   beats the 100th halfmove; the exact precedence list to implement is in
   [`02-implementation-guide.md`](02-implementation-guide.md) §8).

On success: `Ok(updated_snapshot)`. On error: **the engine state must be
completely unchanged** (validate everything before mutating — a rejected move
must not consume history, clocks or repetition counts).

Edge cases to honor:

* En passant is only legal in the ply immediately after the double push;
  later attempts are plain `IllegalMove`.
* Castling is legal only with rights intact, path empty, and the king not in,
  through, or into check.
* All four promotion choices must work (`Queen`, `Rook`, `Bishop`, `Knight`),
  e.g. `"e7e8q"`, `"e7e8r"`, `"e7e8b"`, `"e7e8n"`.
* Applying a move while a draw offer is active: the offer is implicitly
  declined — the engine clears `draw_offered_by` (recommended default,
  confirm with B2, see §7).

### 5.2 `offer_draw` — accept-if-opponent-offered semantics

```rust
fn offer_draw(&mut self, by: PlayerColor) -> Result<GameSnapshot, GameError>;
```

* If the game is over → `Err(GameError::GameAlreadyOver)`.
* If the **opponent** already has an active offer → this call **accepts** it:
  the game ends `Draw` with `GameResult { winner: None, reason:
  GameResultReason::DrawAgreed }`, `draw_offered_by` cleared. (This is B2's
  "accept by counter-offering" UX; the doc comment in `contract.rs` fixes
  this semantics.)
* Otherwise record/keep the offer by `by` (a repeat offer by the same color is
  idempotent) and return `Ok(snapshot)` with `draw_offered_by: Some(by)`.
* A draw offer NEVER changes the board, `turn`, `history_san` or `last_move` —
  only `draw_offered_by` (and, on auto-accept, `status`/`result`).

### 5.3 `accept_draw`

* If the game is over → `Err(GameError::GameAlreadyOver)`.
* If the opponent of `by` has an active offer → the game ends `Draw`,
  `GameResult { winner: None, reason: GameResultReason::DrawAgreed }`,
  `draw_offered_by: None`.
* If there is NO active opponent offer, the frozen contract does not define
  an error variant. **Recommended documented behavior:** return `Ok` with the
  unchanged snapshot (B2's room layer is expected to guard this — see §7,
  open point). Board untouched.

### 5.4 `resign`

* If the game is over → `Err(GameError::GameAlreadyOver)`.
* Otherwise the game ends immediately: `GameStatus::Resigned`,
  `GameResult { winner: Some(by.opposite()), reason:
  GameResultReason::Resignation }` — **the opponent of the resigning color
  wins**, never the resigner.
* The board, FEN, `turn`, `history_san` and `last_move` are untouched
  (resignation is a lifecycle action, not a board move). `legal_moves` becomes
  empty because the game is over.

### 5.5 `reset_for_rematch`

* Resets EVERYTHING to a fresh game: starting position (FEN
  `"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"`), `turn:
  PlayerColor::White`, `GameStatus::InProgress`, `result: None`,
  `last_move: None`, empty `history_san`, the 20 starting `legal_moves`,
  `draw_offered_by: None` — and all internal state a fresh game would have
  (repetition table, clocks), so the reset snapshot equals a brand-new game's
  snapshot exactly.
* Returns `()` (cannot fail). The engine does NOT police when a rematch is
  allowed — mutual agreement is B2's room-level concern.
* Allowed from any state (active or finished).

### 5.6 `snapshot` — purity

* Returns the current authoritative `GameSnapshot` and **never mutates the
  engine** — calling it twice in a row yields equal snapshots (`GameSnapshot`
  derives `PartialEq`), and calling it after a failed `apply_move` shows the
  unchanged pre-move state. Cheap enough to call on every command: B2 sends a
  full snapshot after every accepted state change.
* Implementation note: build the whole `GameSnapshot` fresh on each call
  (clone the FEN/history strings) — do not cache a stale copy.

---

## 6. Constructors — convention and why they are not in the trait

The trait deliberately has **no constructor**. Adding `fn new_game() -> Self`
to the trait would require `Self: Sized` and break dyn-compatibility
(`Box<dyn ChessEngine>`), which is exactly how B2 stores the engine inside a
room actor. This is stated in the doc comment on `trait ChessEngine`.

What you provide instead (inherent items on your concrete type, e.g.
`ShakmatyEngine`):

```rust
// src/game/engine.rs — inherent constructors, OUTSIDE the trait:
impl ShakmatyEngine {
    /// Fresh game from the initial position. THE constructor B2 needs.
    pub fn new_game() -> Self { /* ... */ }

    /// Optional convenience for tests/debugging: start from a FEN string.
    /// NEVER used by B2 in production. Returns Err(message) on a bad FEN.
    pub fn from_fen(fen: &str) -> Result<Self, String> { /* ... */ }
}
```

* **`new_game()`** is required — B2 cannot construct your engine any other
  way (they never name your concrete type except through this call).
* **`from_fen(&str)`** is optional but strongly recommended: several required
  tests (stalemate, fifty-move rule, castling-blocked fixtures) need crafted
  positions; see [`03-testing.md`](03-testing.md). Keep it public (integration
  tests in `backend/tests/` are a separate crate and cannot see
  `#[cfg(test)]` helpers).
* **Factory helper for B2:** if B2 wants to construct engines without naming
  your concrete type (their room actor holds `Box<dyn ChessEngine>`), agree on
  one free function and add it in a small B1+B2 PR:

  ```rust
  /// Agreed factory so B2 never imports the concrete type. Lives in src/game/.
  pub fn new_engine() -> Box<dyn ChessEngine> {
      Box::new(ShakmatyEngine::new_game())
  }
  ```

  Until that PR exists, B2 constructs `ShakmatyEngine::new_game()` and coerces
  it to `Box<dyn ChessEngine>` themselves.

---

## 7. How B2 consumes your engine (the boundary)

Reading `src/rooms/contract.rs` you can see the pattern; you implement the
engine side only:

* **One engine per room, owned by ONE room-actor task.** B2's actor processes
  `RoomCommand`s (`Move`, `OfferDraw`, `AcceptDraw`, `Resign`, `Rematch`, …)
  **sequentially**, translating them 1:1 into your trait calls
  (`RoomCommand::Move` → `apply_move(MoveRequest { from, to, promotion })`,
  etc.). The actor holds `Box<dyn ChessEngine>` — that is why the trait is
  `Send` (moves the engine between tasks) and why constructors stayed out of
  it (dyn-compatibility).
* **No locking needed, do not add any.** Because calls are serialized by the
  actor, `&mut self` methods are always exclusive. No `Mutex`/`RwLock` inside
  your engine — it would be dead weight and a foot-gun.
* **Revision, seats, tokens, connection state are NOT yours.** `expected_revision`,
  dedup, seat authorization: B2, before your method is even called. Never add
  them to the engine or the snapshot.
* **Wire mapping is B2/B3's, not yours.** B2 assembles the protocol
  `GameSnapshot` from your domain snapshot plus room data. Field-by-field:
  `fen → fen`, `turn → turn`, `status → game_status`, `result → result`,
  `last_move → last_move`, `history_san → move_history`,
  `legal_moves → legal_moves`, `draw_offered_by → draw_offer`; B2 adds
  `room_code`, `status` (room status) and `players` (see
  `docs/protocol-v1.md` §4.3 and `src/protocol/server_message.rs` — read
  only, never import).

**Open points to resolve with B2 in a PR (do not silently decide):**

1. **`GameAlreadyOver` precedence** — this doc puts the game-over check before
   square parsing (deterministic error whatever the payload). The brief listed
   it last; if B2 relies on a different order, adjust this doc + tests
   together.
2. **`accept_draw` with no active offer** — no `GameError` variant fits
   (`"no draw offer pending"`); recommended no-op-with-snapshot until B2
   confirms a guard in the room layer.
3. **Does an applied move clear an active draw offer?** Recommended: yes
   (implicit decline, lichess-style).
4. **Auto-draw vs claimable draw** for `FiftyMoveRule` and
   `ThreefoldRepetition`: this doc set implements them as **automatic**
   (engine ends the game the moment the condition is met), because the
   contract has reason variants B2 expects the engine to set. If the team
   wants claimable-only draws, that is a B1+B2 contract PR (a claim entry
   point would be needed).

---

## 8. Quick reference: who calls what

```
client move ──▶ B3 network ──▶ B2 room actor ──▶ B1 ChessEngine
                       (checks revision/seat/token FIRST)
                                        │ apply_move / offer_draw / accept_draw
                                        │ / resign / reset_for_rematch / snapshot
                                        ▼
              broadcast wire snapshot ◀── B2 maps domain GameSnapshot
                                      (game::types)   to protocol GameSnapshot
```

You implement the box on the right. Everything left of it is someone else's —
and all of it talks to you exclusively through the six trait methods and the
`new_game()` constructor.
