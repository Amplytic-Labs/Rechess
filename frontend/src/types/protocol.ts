/**
 * WebSocket protocol v1 — TypeScript mirror of the wire contract.
 *
 * SOURCE OF TRUTH: `docs/protocol-v1.md`. This file mirrors the Rust types
 * in `backend/src/protocol/` exactly — every field name, every wire value.
 *
 * OWNER: F3. Change rule (binding): any protocol change must update, in the
 * SAME reviewed PR: (1) `backend/src/protocol/`, (2) this file,
 * (3) `docs/protocol-v1.md`. Never rename a field silently — the other side
 * compiles and breaks at runtime otherwise.
 *
 * The matching typed fixtures live in `protocol.fixtures.ts`.
 */

export const PROTOCOL_VERSION = 1;

// ---------------------------------------------------------------------------
// Shared enums (wire values are lowercase / snake_case / SCREAMING_SNAKE)
// ---------------------------------------------------------------------------

/** Seat color. */
export type PlayerColor = "white" | "black";

/** Room lifecycle status. */
export type RoomStatus = "waiting" | "active" | "finished" | "expired";

/** Game phase (independent of room status). */
export type GameStatus =
  | "in_progress"
  | "check"
  | "checkmate"
  | "stalemate"
  | "draw"
  | "resigned";

/** Why the game ended. */
export type GameResultReason =
  | "checkmate"
  | "stalemate"
  | "resignation"
  | "draw_agreed"
  | "insufficient_material"
  | "threefold_repetition"
  | "fifty_move_rule";

/** Pawn promotion choice. */
export type Promotion = "q" | "r" | "b" | "n";

/** Stable error codes. Map each to a friendly themed message (RULES.md §4). */
export type ErrorCode =
  | "ROOM_NOT_FOUND"
  | "ROOM_FULL"
  | "ROOM_EXPIRED"
  | "INVALID_TOKEN"
  | "NOT_YOUR_TURN"
  | "ILLEGAL_MOVE"
  | "STALE_REVISION"
  | "GAME_FINISHED"
  | "INVALID_PAYLOAD"
  | "UNSUPPORTED_VERSION"
  | "RATE_LIMITED"
  | "INTERNAL";

// ---------------------------------------------------------------------------
// Shared payload objects
// ---------------------------------------------------------------------------

/** One move for display (last-move highlight + history list). */
export interface MoveRecord {
  from: string;
  to: string;
  /** Standard algebraic notation, e.g. "Nf3", "exd5", "e8=Q". */
  san: string;
}

/** Public player data — safe to broadcast. NEVER contains player_token. */
export interface PlayerPublic {
  id: string;
  nickname: string;
  color: PlayerColor;
  connected: boolean;
}

/** Public room state for the waiting room / lobby views. */
export interface RoomPublicState {
  room_code: string;
  status: RoomStatus;
  players: PlayerPublic[];
}

/** Final result. `winner` is null for draws. */
export interface GameResult {
  winner: PlayerColor | null;
  reason: GameResultReason;
}

/**
 * The full authoritative game state — the ONLY source of truth for the UI.
 * Built by the server (B2) from B1's engine; never assembled client-side.
 */
export interface GameSnapshot {
  room_code: string;
  status: RoomStatus;
  game_status: GameStatus;
  /** Present only when the game is finished. */
  result: GameResult | null;
  /** Full FEN (with legal en-passant square). */
  fen: string;
  turn: PlayerColor;
  last_move: MoveRecord | null;
  /** SAN move list, oldest first. */
  move_history: string[];
  /** UCI-style legal moves for the side to move: "e2e4", "e7e8q". Server only. */
  legal_moves: string[];
  players: PlayerPublic[];
  /** Color of the player with an active draw offer, if any. */
  draw_offer: PlayerColor | null;
}

// ---------------------------------------------------------------------------
// Client → server
// ---------------------------------------------------------------------------

/** Empty payload (room.leave). */
export type EmptyPayload = Record<string, never>;

/**
 * One client command. `type` + `payload` are flattened into the envelope
 * (exactly like serde flatten on the Rust side).
 */
export type ClientCommand =
  | { type: "room.create"; payload: { nickname: string } }
  | { type: "room.join"; payload: { room_code: string; nickname: string } }
  | { type: "session.resume"; payload: { room_code: string; player_token: string } }
  | {
      type: "game.move";
      payload: {
        from: string;
        to: string;
        /** "q" | "r" | "b" | "n", or null when not a promotion (always present). */
        promotion: Promotion | null;
        expected_revision: number;
      };
    }
  | { type: "game.offer_draw"; payload: { expected_revision: number } }
  | { type: "game.accept_draw"; payload: { expected_revision: number } }
  | { type: "game.resign"; payload: { expected_revision: number } }
  | { type: "game.rematch"; payload: { expected_revision: number } }
  | { type: "room.leave"; payload: EmptyPayload };

/**
 * Client envelope: `{ "v": 1, "id": "<uuid>", "type": "…", "payload": {…} }`.
 * `v` must be PROTOCOL_VERSION; `id` is a unique command UUID (expo-crypto
 * `randomUUID()`) used for dedup on the server.
 */
export type ClientEnvelope = { v: number; id: string } & ClientCommand;

// ---------------------------------------------------------------------------
// Server → client
// ---------------------------------------------------------------------------

/** PRIVATE reply to `room.create` — the ONLY event on the wire with a token. */
export interface RoomCreatedPayload {
  room_code: string;
  player_id: string;
  player_token: string;
  color: PlayerColor;
  room: RoomPublicState;
}

/** PRIVATE reply to `room.join`. */
export interface RoomJoinedPayload {
  player_id: string;
  player_token: string;
  color: PlayerColor;
  room: RoomPublicState;
}

/** PRIVATE reply to `session.resume` — full snapshot at the current revision. */
export interface SessionReadyPayload {
  player_id: string;
  color: PlayerColor;
  snapshot: GameSnapshot;
}

export interface PlayerPresencePayload {
  player_id: string;
  connected: boolean;
}

/** Targeted at the mover whose move failed. */
export interface MoveRejectedPayload {
  error: ErrorCode;
  message: string;
  snapshot: GameSnapshot | null;
}

export interface GameFinishedPayload {
  result: GameResult;
  snapshot: GameSnapshot;
}

export interface ServerErrorPayload {
  error: ErrorCode;
  message: string;
}

/** One server event. `game.state` payload IS the snapshot (flattened). */
export type ServerEvent =
  | { type: "room.created"; payload: RoomCreatedPayload }
  | { type: "room.joined"; payload: RoomJoinedPayload }
  | { type: "session.ready"; payload: SessionReadyPayload }
  | { type: "game.state"; payload: GameSnapshot }
  | { type: "player.presence"; payload: PlayerPresencePayload }
  | { type: "game.move_rejected"; payload: MoveRejectedPayload }
  | { type: "game.finished"; payload: GameFinishedPayload }
  | { type: "server.error"; payload: ServerErrorPayload };

/**
 * Server envelope: `{ "v": 1, "type": "…", "revision"?: n, "payload": {…} }`.
 * `revision` is present ONLY on stateful events (`game.state`,
 * `game.move_rejected`, `game.finished`) — never inside the payload.
 */
export type ServerEnvelope = { v: number; revision?: number } & ServerEvent;
