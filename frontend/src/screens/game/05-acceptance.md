# 05 — Acceptance, handoffs, flagged gaps

## 1. Definition of done — mapped line-by-line to F4's brief

Brief: `scripts/extracted_docs/09_F4_Game_Screen_Integration.md`.

| Brief | Requirement (paraphrased) | How it's met / where it's specified | Verify with |
|---|---|---|---|
| §2 | Agree route/screen contract with F1/F3; screen receives only non-secret refs | Route `src/app/game.tsx` takes room code only; token stays in F3's secure storage | Doc 01 §1; checklist C5 |
| §2 | GameScreen layout for common phone ratios, responsive spacing | Composition tree + layout rules; board full-width, bounded history | Doc 01 §2–§3; checklist §3 |
| §2 | Embed F2's board with authoritative FEN/legal moves/last move/orientation/disabled/check props | Snapshot → `ChessBoardProps` mapping table (all 7 props) | Doc 01 §4 |
| §2 | Subscribe to F3's state; render server-derived values; no second copy of board/turn state | Hooks-only wiring; single derivation point; narrow selectors | Doc 01 §6 |
| §2 | Player nicknames, colors, connected status, whose turn | `PlayerInfo` bars + presence dots + active-turn highlight | Doc 01 §5 |
| §2 | Move history in SAN, scrollable, readable | `move_history` SAN pair rows, newest visible, performance note | Doc 01 §5 |
| §2 | Draw offer/accept, resign, rematch via F3 actions; confirm destructive ones | Draw states table; resign Dialog; rematch vote flow | Doc 02 §1–§3 |
| §2 | Show check, checkmate, stalemate, draw, resignation, disconnect states; distinguish disconnect from finished | checkSquare derivation; result mapping; separate presence/connection banners vs result view | Doc 01 §4.2, Doc 02 §4, Doc 03 §3–§4/§6 |
| §2 | Game-over/result view with winner/reason and rematch status | Full `GameResultReason` mapping table + rematch status UX | Doc 02 §3–§4 |
| §2 | Connection banner, reconnect/loading states; board disabled while disconnected or not current player | Banner states from `useConnection()`; one disabled expression | Doc 03 §4–§5 |
| §2 | Handle rejected moves without manually patching the board | Rejection → mapped toast + snapshot reconcile | Doc 03 §2, Doc 02 §5 |
| §2 | Two-client end-to-end tests; record usability defects | Full manual checklists B + C; defects recorded in PR | Doc 04 §2–§3 |
| §4 | Suggested screen composition (header, opponent, turn, board, own, history, actions, banners, result) | Component tree mirrors it | Doc 01 §2 |
| §5 | State/action contract with F3 — shared state, not raw WS in the screen | Exact hook/action names wired; raw socket banned | Doc 01 §6 |
| §6 | Label White/Black; identify local player | Color chip + "You" marker on own bar | Doc 01 §5 |
| §6 | Show current turn; disable submission when not your turn (server still enforces) | TurnIndicator + disabled expression | Doc 03 §5 |
| §6 | Disconnect = reconnecting/waiting state, not defeat | Opponent-disconnect banner + grace-window copy | Doc 03 §3 |
| §6 | History updates only from accepted snapshots | Store-driven history, never insert pending move | Doc 01 §5 |
| §6 | Draw offer identifies offerer; accept/decline per backend | `draw_offer` color + accept flow (+ offer-doubles-as-accept subtlety) | Doc 02 §1 |
| §6 | Resignation requires confirmation | HeroUI Dialog, destructive tokens | Doc 02 §2 |
| §6 | Rematch requires both; waiting-for-opponent state | Vote flow keyed to own vote + fresh-snapshot signal | Doc 02 §3 |
| §6 | Result explains winner and reason | Mapping tables for all reasons | Doc 02 §4 |
| §6 | Safe exit/leave without orphaning socket state | `leaveRoom()` + navigation with F1; never disconnect the service from a screen | Doc 01 §1, Doc 02 §4 |
| §6 | Consistent loading/disabled/error states; no duplicate submissions | `pending` gates on every control | Doc 02–03 |
| §8–§9 | Deliverables + acceptance (full match through the app, backend-confirmed state, disconnect/finish handled, usable on target phones) | Doc set as a whole | Doc 04 all checklists |

## 2. Handoff notes — what F4 depends on

### F2 — `src/components/chess/README.md`

- Consumed exactly: component `ChessBoard`, props `fen`, `orientation`,
  `legalMoves`, `lastMove: {from,to} \| null`, `checkSquare`, `disabled`,
  `onMove({from,to,promotion?})`.
- F2's README §3 mentions `tailwind.config.js` / `ThemeProvider.tsx` for
  tokens — **stale**; tokens actually live in `global.css`, no config, no
  provider (RULES.md §1.3). Does not affect the props; ignore those lines.
- F2's README §6 mentions `npx jest` — **superseded** by RULES.md §5 (no
  jest). Testing for F2 is also typecheck/lint + manual.
- Open item to confirm with F2: the actual export name and file path of the
  board component (`chess-board.tsx` is their suggestion, not final).

### F3 — `src/services/README.md` (+ future `src/hooks/`)

- Consumed exactly: `useRoomActions()` (`sendMove`, `offerDraw`,
  `acceptDraw`, `resign`, `rematch`, `leaveRoom`), `useRoomState()`
  (`roomCode`, `players`, `status`, `drawOffer`, `yourColor`),
  `useGameState()` (snapshot fields + `pending`), `useConnection()`
  (`"connecting"|"connected"|"reconnecting"|"disconnected"`).
- Brief-vs-repo name mismatches resolved in favor of F3's README §7:
  brief's `makeMove` → `sendMove`; brief's `requestRematch` → `rematch`;
  planning notes' `useGameSnapshot`/`useRoom`/`useConnectionStatus`/
  `useSendCommand` → `useGameState`/`useRoomState`/`useConnection`/
  `useRoomActions`.
- `src/hooks/` does not exist yet — until it lands, build via the fixture
  dev variant (doc 04 §1), not by stubbing F3's files yourself.

### F1 — lobby (navigation contract)

- F1's waiting room navigates to the game route when both players are
  present; exact param name (`roomCode`) + back-navigation after
  `leaveRoom()`/`room.leave` need one conversation (their README §4 says
  coordinate with F4).
- Shared visual language: prefer F1's `app-button` variants when they exist;
  otherwise HeroUI directly (RULES.md §2 preference order).

### Backend (B2/B3)

- Backend quick start: `cd backend && cargo run` (health on
  `http://localhost:3000/health`); connection URL is documented by F3.
- Grace window (~60s default) and finished-room retention (~10 min) are
  B2 config constants — the UI must not hardcode them, only set
  expectations (doc 03 §3).

## 3. Open questions / gaps F4 should raise (don't work around silently)

1. **`GameSnapshot` has no rematch-vote field** — the opponent's vote is
   invisible until both voted and the fresh snapshot arrives; "waiting"
   state is keyed only to your own vote. Ask F3 (and B2/B3) to expose
   rematch votes in room state/protocol. (Doc 02 §3.)
2. **No `destructive-foreground` token** — solid destructive buttons need
   `text-primary-foreground` or the outline variant. If that reads badly,
   propose the token via a PR to `global.css`. (Doc 02 §2.)
3. **`checkSquare` is not in the snapshot** — F4 derives the checked king's
   square from FEN (rendering-only parse). If F2 would rather own this
   (their `fen-board.ts` already parses the FEN), coordinate — it's their
   contract field. (Doc 01 §4.2.)
4. **Presence freshness** — between `player.presence` events the snapshot's
   `players[].connected` can be stale; the screen reads players through
   F3's hooks. Confirm F3 merges presence into room state exactly once.
   (Doc 03 §3.)
5. **Draw "decline"** — protocol v1 has no decline command; declining =
   keep playing (or resign). Confirm the team is happy with that UX copy.
   (Doc 02 §1.)
6. **Resume failure navigation** — on `INVALID_TOKEN` after resume, F3/F1
   own the return-home flow; confirm the game screen has nothing extra to
   render. (Doc 02 §5 table; doc 03 §4.)
7. **HeroUI Native named exports** — granular module paths are verified
   (`heroui-native/dialog`, `/button`, `/toast`, `/spinner`, …) but the
   exact named exports inside each module were not audited here; check the
   installed package (`node_modules/heroui-native/lib/typescript/src/…`)
   before importing. The ROOT import is banned either way.
