# `src/screens/game/` — Owner: **F4** (Game Screen and Frontend Integration)

> **Read `frontend/RULES.md` first** — it is binding (no StyleSheet, no
> expo-status-bar, everything themed, ownership map). Then read this file
> completely before writing code.

---

## 1. Your mission

Deliver the in-game experience: the main GameScreen layout, player panels,
turn indicator, move history, draw/resign/rematch controls, connection
banner, game-over view — and assemble everyone's work into the complete
player flow. You own everything the user sees **after** both players are in
the room.

You compose F1's common components, F2's `ChessBoard` and F3's
state/hooks. You do **not** own the board component, the stores, or the
lobby screens.

## 2. What you own / what you don't touch

| You own | You do NOT touch |
|---|---|
| `src/screens/game/**` | `src/components/chess/` (F2) — embed, don't edit |
| `src/app/game.tsx` (thin route wrapper) | `src/services/`, `src/store/`, `src/types/` (F3) |
| Player panels, turn indicator, history list | `src/screens/lobby/**`, its routes (F1) |
| Draw/resign/rematch controls + confirmations | `src/components/common/` (F1 — request changes via PR) |
| Connection banner, game-over/result view | |

## 3. The base that already exists (don't rebuild it)

- Routes mount from `src/app/`: create `game.tsx` when your screen is ready
  (thin wrapper — see RULES.md §3; coordinate the route name with F1).
- All data comes from F3's hooks: `useGameState()`, `useRoomState()`,
  `useRoomActions()`, `useConnection()` (names frozen with F3 —
  see `src/services/README.md` §7).
- F2's `ChessBoard` props contract: `src/components/chess/README.md` §4.
- Board/theme tokens: `board-light`, `board-dark`, `move-target`,
  `last-move`, `check` — use them, don't redefine them.

## 4. Screen composition (your brief)

```
┌──────────────────────────┐
│  opponent panel (name,   │  nickname, color, connected dot
│  color, clock-less state) │
├──────────────────────────┤
│                          │
│      ChessBoard (F2)     │  FEN/legalMoves/orientation from F3;
│                          │  disabled when not your turn/pending/offline
├──────────────────────────┤
│  your panel + turn       │  "Your move" / "Waiting for opponent…"
├──────────────────────────┤
│  move history (scroll)   │  SAN from authoritative snapshots only
├──────────────────────────┤
│  actions: draw · resign  │  rematch appears after game finished
└──────────────────────────┘
```

- Layout must work on common phone aspect ratios (test small phones);
  board scales to width (F2 guarantees square squares — give it a width).
- Orientation: your color at the bottom (`orientation = yourColor`).
- The screen receives **only non-secret** room/session references (route
  param: room code at most — never the token).

## 5. Behaviour requirements (your brief)

- **Server-derived only:** render from F3's stores. Do not keep a second
  copy of board/turn state; do not patch the board on errors — the store
  applies the authoritative snapshot.
- Turn: clear "your move / opponent's move" indication; disable move
  submission when it's not your turn (server still enforces — this is UX).
- Move history: scrollable, readable, from `move_history` (SAN).
- Controls: offer/accept draw, resign (confirmation dialog — destructive!),
  rematch vote with "waiting for opponent" state.
- Connection banner: reconnecting / opponent disconnected — distinct from a
  finished game; board interaction disabled while disconnected.
- Rejected moves: show F3's safe error message (toast/inline), never patch
  the board.
- Game-over view: winner + reason (checkmate, resignation, stalemate,
  draw reason) from `result`, with rematch buttons.
- Duplicate submissions: respect F3's `pending` — disable buttons while
  pending.
- Leave/exit: navigate out cleanly without orphaning the socket (call
  F3's `leaveRoom` — don't disconnect the service from a screen).

## 6. How to test your module independently

```bash
cd frontend
npm run typecheck && npm run lint
npx jest game
```

- Build against a **mock snapshot** first (Day 1–2): render GameScreen with
  F3's hooks mocked, F2's board with a fixture FEN.
- Component tests: panels show nicknames/colors/connection; turn text
  switches with `turn`; resign needs confirmation; rematch shows
  waiting state; result view maps reasons to text; disabled states while
  pending/disconnected.
- Days 3–4: real F3 service, two devices/simulators, B3's server — record
  the usability defects. Days 7–8: end-to-end QA pass with the team.

## 7. Deliverables (definition of done)

- [ ] `src/app/game.tsx` route (thin) + complete GameScreen composition
- [ ] Player panels, turn indicator, move history, action controls
- [ ] Confirmation for resign; rematch both-vote UX; game-over view with reasons
- [ ] Connection banner + board disabled states
- [ ] Component tests with mocked stores + fixture snapshots
- [ ] Two-device end-to-end pass logged with the team

## 8. Key coordination

- **F3** daily: hook names, snapshot fields, pending/error semantics.
- **F2**: the ChessBoard prop contract (their README §4) — changes via PR.
- **F1**: navigation into/out of the game route, shared visual language.
- **B4**: end-to-end tests and the demo flow.
