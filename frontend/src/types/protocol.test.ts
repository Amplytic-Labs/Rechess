/**
 * Wire-contract unit tests — the TS mirror of the backend's
 * `backend/tests/protocol_contract.rs` (see RULES.md §5). This file is the
 * pattern for F3's suite: pure logic, node environment, no DOM. F3 extends
 * it (or adds siblings) as codec, stores and reconnect logic land.
 */
import { describe, expect, it } from "vitest";

import {
  sampleFinishedSnapshot,
  sampleMoveCommand,
  sampleRoom,
  sampleRoomCreatedEvent,
  sampleSnapshot,
} from "./protocol.fixtures";

describe("typed fixtures satisfy the wire types", () => {
  it("sampleSnapshot is a full in-progress GameSnapshot", () => {
    expect(sampleSnapshot.status).toBe("active");
    expect(sampleSnapshot.game_status).toBe("in_progress");
    expect(sampleSnapshot.result).toBeNull();
    expect(sampleSnapshot.legal_moves.length).toBeGreaterThan(0);
    expect(sampleSnapshot.players).toHaveLength(2);
  });

  it("the finished snapshot carries a result and no legal moves", () => {
    expect(sampleFinishedSnapshot.game_status).toBe("checkmate");
    expect(sampleFinishedSnapshot.result).toEqual({
      winner: "black",
      reason: "checkmate",
    });
    expect(sampleFinishedSnapshot.legal_moves).toEqual([]);
  });

  it("sampleRoom is a waiting room with the host seated", () => {
    expect(sampleRoom.status).toBe("waiting");
    expect(sampleRoom.players).toHaveLength(1);
    expect(sampleRoom.players[0]?.connected).toBe(true);
  });
});

describe("envelope shapes on the wire (docs/protocol-v1.md §2)", () => {
  it("game.move always carries promotion (null when not a promotion)", () => {
    const json = JSON.parse(JSON.stringify(sampleMoveCommand)) as {
      type: string;
      payload: { promotion: unknown; expected_revision: number };
    };
    expect(json.type).toBe("game.move");
    expect("promotion" in json.payload).toBe(true);
    expect(json.payload.promotion).toBeNull();
    expect(json.payload.expected_revision).toBe(12);
  });

  it("the client envelope flattens v + id + type + payload exactly like serde", () => {
    expect(Object.keys(sampleMoveCommand).sort()).toEqual([
      "id",
      "payload",
      "type",
      "v",
    ]);
  });

  it("room.created carries no revision (only stateful events do)", () => {
    const json = JSON.parse(
      JSON.stringify(sampleRoomCreatedEvent),
    ) as Record<string, unknown> & { payload: Record<string, unknown> };
    expect("revision" in json).toBe(false);
    expect("revision" in json.payload).toBe(false);
  });

  it("player_token appears only in room.created/room.joined payloads", () => {
    const json = JSON.parse(
      JSON.stringify(sampleRoomCreatedEvent),
    ) as { type: string; payload: Record<string, unknown> };
    expect(json.type).toBe("room.created");
    expect("player_token" in json.payload).toBe(true);
    // The other 7 event types carry no token — pinned by the ServerEvent
    // discriminated union at compile time (see protocol.ts).
  });
});
