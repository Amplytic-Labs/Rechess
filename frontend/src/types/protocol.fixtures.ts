/**
 * Typed wire fixtures — TypeScript mirror of the backend test fixtures
 * (`backend/src/protocol/mod.rs` → `sample_snapshot`, and
 * `backend/src/rooms/testkit.rs` → `sample_protocol_snapshot`).
 *
 * How the protocol contract is verified (see RULES.md §5):
 *
 *  1. `npm run typecheck` proves these fixtures satisfy the types in
 *     `protocol.ts` — the TS wire contract compiles against realistic data.
 *  2. `npm run test` pins the envelope shapes against these fixtures
 *     (`src/types/protocol.test.ts`).
 *  3. `cargo test` on the backend proves the IDENTICAL JSON round-trips
 *     through the Rust wire types (`backend/tests/protocol_contract.rs`).
 *
 * F1 / F2 / F4: import these fixtures to render screens and components
 * with realistic server data while F3's WebSocket service is still in
 * development (e.g. feed `sampleSnapshot` to F2's board adapter).
 */
import {
  ClientEnvelope,
  GameSnapshot,
  PROTOCOL_VERSION,
  RoomPublicState,
  ServerEvent,
} from "./protocol";

/** Full in-progress game, one move played — mirrors the backend fixture. */
export const sampleSnapshot: GameSnapshot = {
  room_code: "TESTCODE1",
  status: "active",
  game_status: "in_progress",
  result: null,
  fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
  turn: "black",
  last_move: { from: "e2", to: "e4", san: "e4" },
  move_history: ["e4"],
  legal_moves: ["e7e5", "e7e6"],
  players: [
    { id: "host", nickname: "Host", color: "white", connected: true },
    { id: "guest", nickname: "Guest", color: "black", connected: false },
  ],
  draw_offer: null,
};

/** Waiting room with only the host seated — for lobby UI (F1). */
export const sampleRoom: RoomPublicState = {
  room_code: "TESTCODE1",
  status: "waiting",
  players: [{ id: "host", nickname: "Host", color: "white", connected: true }],
};

/** Finished game (black wins by checkmate) — for result UI (F4). */
export const sampleFinishedSnapshot: GameSnapshot = {
  ...sampleSnapshot,
  game_status: "checkmate",
  result: { winner: "black", reason: "checkmate" },
  turn: "white",
  legal_moves: [],
};

/** The private reply to room.create — the ONLY event carrying player_token. */
export const sampleRoomCreatedEvent: ServerEvent = {
  type: "room.created",
  payload: {
    room_code: "TESTCODE1",
    player_id: "host",
    player_token: "secret-token",
    color: "white",
    room: sampleRoom,
  },
};

/** A submit-move envelope exactly as the client sends it (doc §2.1 example). */
export const sampleMoveCommand: ClientEnvelope = {
  v: PROTOCOL_VERSION,
  id: "cmd-unique-001",
  type: "game.move",
  payload: {
    from: "e2",
    to: "e4",
    promotion: null,
    expected_revision: 12,
  },
};
