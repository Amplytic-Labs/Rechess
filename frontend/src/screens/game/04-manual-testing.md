# 04 — Manual testing (browser-based — this REPLACES component tests)

**UI is manual-tested in the browser — by design** (RULES.md §5; the
original briefs' "component tests" are superseded). vitest exists for pure
logic only and F3 owns that suite — you add no test files here. Your
testing loop is exactly:

```bash
cd frontend
npm run typecheck   # tsc --noEmit — proves your code against the real types
npm run lint        # eslint — enforces RULES.md (tokens only, thin routes, …)
npm run test        # vitest — F3's pure-logic suite must stay green
npm run build       # production build — proves the dev harness never ships
```

…plus the manual checklists below, executed in the browser (desktop AND a
mobile viewport — no device emulators outside the browser devtools, no extra
tooling), and noted in your PR description.

## 1. Building against fixtures (backend not up yet)

You must be able to build and exercise the screen with NO socket and NO
backend, using the typed fixtures in `@/types/protocol.fixtures`
(`sampleSnapshot` — in-progress game, one move played, guest disconnected;
`sampleFinishedSnapshot` — black wins by checkmate; see the file for exact
values).

**Documented approach — dev-only fixture variant (recommended):**

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
- **It must not ship.** Gate the mount on the build-time NODE_ENV constant
  with a dynamic import, so the production client bundle never contains the
  dev module or the fixtures:

  ```tsx
  // game-screen.tsx — dev-only harness mount, top of module
  const IS_DEV = process.env.NODE_ENV === "development";
  const GameScreenDev = IS_DEV
    ? lazy(() => import("./game-screen.dev"))   // eliminated by `next build`
    : () => null;
  ```

  `next build` statically replaces `process.env.NODE_ENV`, so `IS_DEV` is
  `false` in production and the dev module is dropped from the bundle. Call
  the gating out in the PR so reviewers re-check it — an alternative is to
  temporarily point the thin route at the dev variant while developing and
  revert before merge; either way, the "not shipped" check below is
  mandatory.

  **Not-shipped check:** after `npm run build`, confirm the client chunks
  contain no fixture data or dev module — e.g.
  `grep -r "TESTCODE1" .next/static/` returns nothing (the fixture room
  code only exists in `protocol.fixtures.ts`).

```tsx
// src/screens/game/game-screen.dev.tsx — SKELETON (F4: implement; DEV ONLY)
// NOT reachable in production — see doc 04 §1 rules.
"use client";

import { useState } from "react";

import { sampleFinishedSnapshot, sampleSnapshot } from "@/types/protocol.fixtures";

export function GameScreenDev() {
  // TODO: local state seeded from sampleSnapshot
  // TODO: buttons: "finished (checkmate)", "toggle turn",
  //       "toggle opponent connected", "draw offer from opponent",
  //       "pending move" — each flips state that the real child components
  //       render, so you can exercise every screen state without a server.
  // TODO: render the same components the real screen renders, with
  //       fixture-driven values (board from snapshot.fen, history from
  //       snapshot.move_history, players from snapshot.players …).
  return <main className="flex flex-1 flex-col bg-background">{/* … */}</main>;
}
```

**Checklist A — fixture render (no server).** For each row: verify at
`http://localhost:3001` (`npm run dev`), in a desktop window AND a mobile
viewport (devtools device toolbar, ~375px wide, or a real phone), light +
dark mode.

| # | Step | Expected result |
|---|---|---|
| A1 | Render with `sampleSnapshot` | Board shows the position after 1. e4 (`fen` ends `4P3`), orientation = your seat, opponent bar "Guest" with a **disconnected** dot (fixture has `connected: false`), history shows "1. e4" |
| A2 | `turn` is `"black"` in the fixture | Turn indicator reads "Waiting for opponent…" when yourColor=white; board disabled (or "Your move" when yourColor=black) |
| A3 | Toggle to `sampleFinishedSnapshot` | Result view: "Checkmate" reason, winner=black ("You win" if black is your seat, "You lose" if white), rematch + leave controls, board inert |
| A4 | Fixture `draw_offer: null` | No draw banner; "Offer draw" button visible |
| A5 | `npm run typecheck && npm run lint` | Zero errors — fixtures prove the wiring compiles against real types |
| A6 | `npm run build`, then grep the client chunks for fixture data | Build passes; no `TESTCODE1` / dev module in the production bundle |

## 2. Live scenarios — dev backend, two browser clients

Start the backend (repo README): `cd backend && cargo run` (health check
`curl http://localhost:3000/health`). The WebSocket URL comes from
`NEXT_PUBLIC_API_URL` (F3's config module reads it; default
`ws://localhost:3000/ws` for local dev — RULES.md §5), so a browser on the
same machine needs no configuration. To test on a real phone over the LAN,
open the dev server's network URL (`http://<LAN-IP>:3001`, printed by
`npm run dev`) and point the env at `ws://<LAN-IP>:3000/ws`.

**Two clients = two separate browser contexts**, e.g. two different browsers,
or one normal + one private/incognito window. **Do NOT use two tabs of the
same browser window**: tabs share F3's token storage for the origin, so the
second tab would resume the SAME seat instead of joining as the opponent.

**Checklist B — the create → join → move loop (your brief's Day-4
milestone):**

| # | Step | Expected result |
|---|---|---|
| B1 | Client A: create room (nickname) via the lobby | A gets White, waiting-room state; room code 8 chars |
| B2 | Client B: join with the code | Both clients show both players connected; both are on the game route |
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
| C1 | Cut B's network (devtools network offline emulation, or OS Wi-Fi off) | B: "Reconnecting…" banner, board disabled; A: "Opponent disconnected" banner (NOT a result view), game continues |
| C2 | Restore B's network within the grace window | B resumes its seat (F3's resume); banners clear on both; both show the same current snapshot |
| C3 | Switch B to another tab / minimize the window for a while, return | Same as C1→C2 if the socket dropped; tab visibility is no reason to lose the seat |
| C4 | Hard-reload B's tab mid-game (`Ctrl/Cmd+R`) | B remounts, F3 resumes from storage; same seat, same color, current snapshot + history restored — no duplicate moves, no crash |
| C5 | Kill the server | Both: reconnecting/disconnected banner; the board still shows the last authoritative snapshot (read-only) |
| C6 | Expired/invalid resume token (restart server so rooms are gone — in-memory state) | Friendly `ROOM_NOT_FOUND`/`INVALID_TOKEN` handling, routed home — no crash, no raw JSON |

## 3. Before-done checklist

- [ ] `npm run typecheck` — zero errors.
- [ ] `npm run lint` — zero errors/warnings.
- [ ] `npm run test` — F3's vitest suite still green (you did not break the
      pure-logic layer).
- [ ] `npm run build` — production build passes; dev harness + fixtures
      verified absent from the client bundle (checklist A6).
- [ ] No hardcoded colors: search your diff for `#`, `rgb(`, `hsl(` — every
      color is a token utility (`bg-surface`, `text-muted-foreground`,
      `border-border`, `bg-primary`, `bg-destructive`, `bg-success`, …).
- [ ] No `style` prop (except a documented dynamic value), no CSS files or
      CSS modules of your own — Tailwind `className` only.
- [ ] Dark mode verified: run checklist A with the OS appearance switched
      (or the browser devtools `prefers-color-scheme` emulation) — no
      unreadable text, no white-on-white.
- [ ] Narrow viewport verified (~375px wide, devtools device toolbar or a
      real phone): board fully visible and above the fold, history scrolls,
      controls reachable, no horizontal page scroll.
- [ ] Wide viewport verified (laptop width): board + side panel layout, no
      fixed page width, no page scroll caused by the board.
- [ ] Keyboard: every control reachable and operable by keyboard with
      visible focus (tab through header, actions, dialogs).
- [ ] Dev variant (`game-screen.dev.tsx`) NOT reachable in production
      (verified via the build, checklist A6).
- [ ] No `player_token` anywhere in your screens/logs (you should not even
      import anything token-related).
- [ ] Two-browser pass (checklist B+C) recorded — defects noted in the PR
      description / team channel (your brief: "record usability defects").
