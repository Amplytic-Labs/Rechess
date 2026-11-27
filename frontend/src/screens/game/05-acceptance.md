# 05 — Acceptance, handoffs, flagged gaps

## 1. Definition of done — mapped line-by-line to F4's brief

Brief: `scripts/extracted_docs/09_F4_Game_Screen_Integration.md`.

| Brief | Requirement (paraphrased) | How it's met / where it's specified | Verify with |
|---|---|---|---|
| §2 | Agree route/screen contract with F1/F3; screen receives only non-secret refs | Thin route `src/app/game/[roomCode]/page.tsx`; the screen reads the room code from the URL (`useParams`); token stays in F3's storage, never in a URL | Doc 01 §1; checklist C5 |
| §2 | GameScreen layout, responsive spacing | Mobile-portrait-first composition tree; board + side panel on wide screens; no fixed page widths | Doc 01 §2–§3; checklist §3 |
| §2 | Embed F2's board with authoritative FEN/legal moves/last move/orientation/disabled/check props | Snapshot-based `ChessBoardProps` mapping table (4 props) | Doc 01 §4 |
| §2 | Subscribe to F3's state; render server-derived values; no second copy of board/turn state | Hooks-only wiring; single derivation point; narrow selectors | Doc 01 §6 |
| §2 | Player nicknames, colors, connected status, whose turn | `PlayerInfo` bars + presence dots + active-turn highlight | Doc 01 §5 |
| §2 | Move history in SAN, scrollable, readable | `move_history` SAN pair rows, newest visible, performance note | Doc 01 §5 |
| §2 | Draw offer/accept, resign, rematch via F3 actions; confirm destructive ones | Draw states table; resign dialog (F1's Dialog primitive); rematch vote flow | Doc 02 §1–§3 |
| §2 | Show check, checkmate, stalemate, draw, resignation, disconnect states; distinguish disconnect from finished | check highlight internal to F2's board (from `game_status`); result mapping; separate presence/connection banners vs result view | Doc 01 §4.2, Doc 02 §4, Doc 03 §3–§4/§6 |
| §2 | Game-over/result view with winner/reason and rematch status | Full `GameResultReason` mapping table + rematch status UX | Doc 02 §3–§4 |
| §2 | Connection banner, reconnect/loading states; board disabled while disconnected or not current player | Banner states from `useConnection()`; one disabled expression | Doc 03 §4–§5 |
| §2 | Handle rejected moves without manually patching the board | Rejection → mapped toast + snapshot reconcile | Doc 03 §2, Doc 02 §5 |
| §2 | Two-client end-to-end tests; record usability defects | Full manual checklists B + C (two browser contexts); defects recorded in PR | Doc 04 §2–§3 |
| §4 | Suggested screen composition (header, opponent, turn, board, own, history, actions, banners, result) | Component tree mirrors it (+ the route's own waiting state) | Doc 01 §2 |
| §5 | State/action contract with F3 — shared state, not raw WS in the screen | Exact hook/action names wired; raw socket, API routes and `fetch` all banned | Doc 01 §6 |
| §6 | Label White/Black; identify local player | Color chip + "You" marker on own bar | Doc 01 §5 |
| §6 | Show current turn; disable submission when not your turn (server still enforces) | TurnIndicator + disabled expression | Doc 03 §5 |
| §6 | Disconnect = reconnecting/waiting state, not defeat | Opponent-disconnect banner + grace-window copy | Doc 03 §3 |
| §6 | History updates only from accepted snapshots | Store-driven history, never insert pending move | Doc 01 §5 |
| §6 | Draw offer identifies offerer; accept/decline per backend | `draw_offer` color + accept flow (+ offer-doubles-as-accept subtlety) | Doc 02 §1 |
| §6 | Resignation requires confirmation | Shared Dialog primitive, destructive tokens | Doc 02 §2 |
| §6 | Rematch requires both; waiting-for-opponent state | Vote flow keyed to own vote + fresh-snapshot signal | Doc 02 §3 |
| §6 | Result explains winner and reason | Mapping tables for all reasons | Doc 02 §4 |
| §6 | Safe exit/leave without orphaning socket state | `leaveRoom()` + navigation with F1; never disconnect the service from a screen | Doc 01 §1, Doc 02 §4 |
| §6 | Consistent loading/disabled/error states; no duplicate submissions | `pending` gates on every control | Doc 02–03 |
| §8–§9 | Deliverables + acceptance (full match through the app, backend-confirmed state, disconnect/finish handled, usable on target phones) | Doc set as a whole | Doc 04 all checklists (incl. mobile viewport) |

## 2. Handoff notes — what F4 depends on

### F2 — `src/components/chess/README.md`

- Consumed exactly: component `ChessBoard`, props `snapshot: GameSnapshot`,
  `orientation`, `disabled?`, `onMoveSubmit({from,to,promotion})`
  (`promotion` always present: `Promotion | null`).
- Open item to confirm with F2: the actual component name and file path of
  the board component (`chess-board.tsx` is their suggestion, not final).

### F3 — `src/services/README.md` (+ future `src/hooks/`)

- Consumed exactly: `useRoomActions()` (`submitMove`, `offerDraw`,
  `acceptDraw`, `resign`, `rematch`, `leaveRoom`), `useRoomState()`
  (`roomCode`, `players`, `status`, `yourColor`, `connection`),
  `useGameState()` (`snapshot` + `pending`), `useConnection()`
  (slice: `status`, `attempts`, `lastError` with
  `"connecting"|"connected"|"reconnecting"|"disconnected"`).
- **Resume is service-internal** — F3 sends `session.resume` on
  connect/reconnect from its token storage; screens never call it and never
  see the token.
- Brief-vs-repo name mismatches resolved in favor of F3's doc 03 §5:
  brief's `makeMove` → `submitMove`; brief's `requestRematch` → `rematch`;
  planning notes' `useGameSnapshot`/`useRoom`/`useConnectionStatus`/
  `useSendCommand` → `useGameState`/`useRoomState`/`useConnection`/
  `useRoomActions`.
- `src/hooks/` does not exist yet — until it lands, build via the fixture
  dev variant (doc 04 §1), not by stubbing F3's files yourself.

### F1 — lobby + shared primitives

- Navigation in/out: the lobby navigates into the game route
  (`/game/[roomCode]`, replace-style so Back doesn't return to a stale
  lobby screen); the exact call and the leave target after `leaveRoom()`
  need one conversation (their doc set owns the lobby routes; the binding
  navigation rules are RULES.md §3 — App Router, `next/link`,
  `useRouter`/`useParams`, no other router).
- Waiting-state split — RESOLVED: `/game/[roomCode]` mounts F1's
  `WaitingRoom` (`@/screens/lobby/waiting-room`) while
  `status === "waiting"` (doc 01 §1) — one waiting screen, owned by F1,
  mounted by you; the user never sees two different "waiting" screens.
- Shared visual language: buttons, dialogs and toasts come from F1's
  `src/components/common/` — confirm the exact names/paths/variants against
  F1's current doc set before wiring (see gap §3.7).

### Backend (B2/B3)

- Backend quick start: `cd backend && cargo run` (health on
  `http://localhost:3000/health`); the WS URL comes from
  `NEXT_PUBLIC_API_URL` (default `ws://localhost:3000/ws` — RULES.md §5).
- Grace window (~60s default) and finished-room retention (~10 min) are
  B2 config constants — the UI must not hardcode them, only set
  expectations (doc 03 §3).

## 3. Open questions / gaps F4 should raise (don't work around silently)

1. **`GameSnapshot` has no rematch-vote field** — the opponent's vote is
   invisible until both voted and the fresh snapshot arrives; "waiting"
   state is keyed only to your own vote. Ask F3 (and B2/B3) to surface
   rematch votes in room state/protocol. (Doc 02 §3.)
2. **`destructive-foreground` token — resolved (added to `global.css`):**
   solid destructive buttons use `bg-destructive` +
   `text-destructive-foreground`. (Doc 02 §2.)
3. **Check-square derivation — resolved with the snapshot contract:** F2's
   board derives the checked king's square from the FEN internally when
   `game_status === "check"` (their doc 02 §3); F4 passes no `checkSquare`
   prop. (Doc 01 §4.2.)
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
7. **F1's shared primitives are the single control source** —
   `src/components/common/` currently contains no components; the exact
   names, paths and variants (Button primary/secondary/destructive with
   disabled + loading, Dialog, Toast) are defined by F1's doc set. Confirm
   them before wiring (doc 02 preamble), and PR — never hand-roll — for
   anything missing.
8. **Browser Back / accidental navigation mid-game** — reload is safe
   (F3 resumes from storage), but the browser Back button leaving an active
   game does not call `leaveRoom()`. Decide with F1 whether to guard it
   (an unload guard) or accept it; the explicit Leave control is the
   supported exit either way. (Doc 01 §1.)
