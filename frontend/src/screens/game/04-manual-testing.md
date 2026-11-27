# 04 — Manual testing (this REPLACES unit tests)

**There is no jest on this frontend — by design** (RULES.md §5; the original
briefs' "component tests" are superseded). Your testing loop is exactly:

```bash
cd frontend
npm run typecheck   # tsc --noEmit — proves your code against the real types
npm run lint        # eslint — enforces RULES.md (no StyleSheet, tokens only, …)
```

…plus the manual checklists below, executed on a phone/simulator, and noted
in your PR description.

## 1. Building against fixtures (backend not up yet)

You must be able to build and exercise the screen with NO socket and NO
backend, using the typed fixtures in `@/types/protocol.fixtures`
(`sampleSnapshot` — in-progress game, one move played, guest disconnected;
`sampleFinishedSnapshot` — black wins by checkmate; see the file for exact
values).

**Documented approach — props-injected dev variant (recommended):**

- The real `game-screen.tsx` takes its data from F3's hooks ONLY — no props
  injection in the shipping path.
- Add a separate **dev-only** wrapper `game-screen.dev.tsx` that feeds the
  screen from props: the screen component accepts an optional
  `initialSnapshot`-style override ONLY via this dev wrapper's state, or the
  dev wrapper renders the same child components with fixture data directly.
  Keep it dead simple: local React state seeded from `sampleSnapshot`, a
  couple of buttons to swap to `sampleFinishedSnapshot` / toggle
  `turn` / toggle opponent `connected`, so you can drive every UI state by
  hand.
- **It must not ship enabled:** gate it with `if (__DEV__)` or leave it
  reachable only from a dev-only route/entry that the production navigator
  never registers — and call this out in the PR so reviewers check it. The
  production game path is `src/app/game.tsx` → `game-screen.tsx` with live
  hooks, period.

```tsx
// src/screens/game/game-screen.dev.tsx — SKELETON (F4: implement; DEV ONLY)
// NOT reachable in production — see doc 04 §1 rules.
import { useState } from "react";

import { sampleFinishedSnapshot, sampleSnapshot } from "@/types/protocol.fixtures";

import { StyledSafeAreaView } from "@/components/common/styled";

export function GameScreenDev() {
  // TODO: local state seeded from sampleSnapshot
  // TODO: buttons: "finished (checkmate)", "toggle turn",
  //       "toggle opponent connected", "draw offer from opponent",
  //       "pending move" — each flips state that the real child components
  //       render, so you can exercise every screen state without a server.
  // TODO: render the same components the real screen renders, with
  //       fixture-driven values (board from snapshot.fen, history from
  //       snapshot.move_history, players from snapshot.players …).
  return <StyledSafeAreaView className="flex-1 bg-background">{/* … */}</StyledSafeAreaView>;
}
```

**Checklist A — fixture render (no server).** For each row: verify on a
phone/simulator (small phone included), light + dark mode.

| # | Step | Expected result |
|---|---|---|
| A1 | Render with `sampleSnapshot` | Board shows the position after 1. e4 (`fen` ends `4P3`), orientation = your seat, opponent bar "Guest" with a **disconnected** dot (fixture has `connected: false`), history shows "1. e4" |
| A2 | `turn` is `"black"` in the fixture | Turn indicator reads "Waiting for opponent…" when yourColor=white; board disabled (or "Your move" when yourColor=black) |
| A3 | Toggle to `sampleFinishedSnapshot` | Result view: "Checkmate" reason, winner=black ("You win" if black is your seat, "You lose" if white), rematch + leave controls, board inert |
| A4 | Fixture `draw_offer: null` | No draw banner; "Offer draw" button visible |
| A5 | `npm run typecheck && npm run lint` | Zero errors — fixtures prove the wiring compiles against real types |

## 2. Live scenarios — dev backend, two clients

Start the backend (repo README): `cd backend && cargo run` (health check
`curl http://localhost:3000/health`). F3's service README documents the
connection URL — for a phone, the server's LAN IP. Run two clients: two
simulators, or simulator + Expo Go on a physical phone.

**Checklist B — the create → join → move loop (your brief's Day-4
milestone):**

| # | Step | Expected result |
|---|---|---|
| B1 | Client A: create room (nickname) | A gets White, waiting-room state; room code 8 chars |
| B2 | Client B: join with the code | Both clients show both players connected; both navigate to the game screen |
| B3 | A (White) plays e2e4 | A: pending spinner, board NOT updated optimistically; then `game.state` revision N+1 applies: e4 shows, history "1. e4", turn passes to Black on BOTH clients |
| B4 | Board state on B | Same FEN, same revision, same history — no drift |
| B5 | B (Black) plays a legal reply | Same loop mirrored; A's board updates without A touching anything |
| B6 | Not-your-turn: A taps a piece & square while it's Black's move | Board disabled / no `game.move` sent; even if a move got through, server rejects with `NOT_YOUR_TURN` → friendly toast, board unchanged |
| B7 | Illegal move path (e.g. try moving through a blocked square via any UI path that lets you) | `ILLEGAL_MOVE` toast ("That move isn't legal in this position."), board unchanged |
| B8 | Stale revision (two moves submitted near-simultaneously from the same client if the UI allows) | Exactly one lands; the other gets `STALE_REVISION` toast + board reconciles to the fresh snapshot |
| B9 | Complete a fast checkmate (scholar's mate) | Both clients show `game.finished` result view: winner + "Checkmate" reason; rematch controls appear |
| B10 | Draw offer: A taps "Offer draw" | A: "waiting for opponent"; B: draw-offer banner + "Accept draw" button |
| B11 | B accepts | Both show draw result view: "Draw" + "draw_agreed" reason |
| B12 | Resign: A taps Resign → confirm dialog → confirm | Result view: B wins by "resignation" on both clients; cancel path does NOT resign |
| B13 | Rematch after a result: A votes, then B votes | After A: "Waiting for opponent…"; after B: fresh `game.state` — reset board, empty history, status active on BOTH clients |

**Checklist C — connection error paths:**

| # | Step | Expected result |
|---|---|---|
| C1 | Kill B's network (airplane mode / simulator offline) | B: "Reconnecting…" banner, board disabled; A: "Opponent disconnected" banner (NOT a result view), game continues |
| C2 | Restore B's network within the grace window | B resumes its seat (F3's resume); banners clear on both; both show the same current snapshot |
| C3 | Background the app, return | Same as C1→C2 via app lifecycle |
| C4 | Kill the server | Both: reconnecting/disconnected banner; the board still shows the last authoritative snapshot (read-only) |
| C5 | Expired/invalid resume token (restart server so rooms are gone — in-memory state) | Friendly `ROOM_NOT_FOUND`/`INVALID_TOKEN` handling, routed home — no crash, no raw JSON |

## 3. Before-done checklist

- [ ] `npm run typecheck` — zero errors.
- [ ] `npm run lint` — zero errors/warnings.
- [ ] No hardcoded colors: search your diff for `#`, `rgb(`, `hsl(` — every
      color is a token utility (`bg-surface`, `text-muted-foreground`,
      `border-border`, `bg-primary`, `bg-destructive`, `bg-success`, …).
- [ ] No `StyleSheet`, no `style` prop (except a documented dynamic value),
      no `expo-status-bar` import.
- [ ] Dark mode verified: run checklist A with the simulator appearance
      switched — no unreadable text, no white-on-white.
- [ ] Small phone verified (e.g. a 375pt-wide device): board fully visible,
      history scrolls, controls reachable.
- [ ] Dev variant (`game-screen.dev.tsx`) NOT reachable in production.
- [ ] No `player_token` anywhere in your screens/logs (you should not even
      import anything token-related).
- [ ] Two-device pass (checklist B+C) recorded — defects noted in the PR
      description / team channel (your brief: "record usability defects").
