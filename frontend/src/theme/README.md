# `src/theme/` — design tokens (shared, F1 curates changes via PR)

- There is **no theme provider and no theme-switching JS** — theming is
  CSS-first. All tokens live in `global.css` (frontend root), imported once
  by `src/app/layout.tsx`. Tailwind v4 reads them from `@theme`, so each
  `--color-<token>` becomes utilities: `bg-background`, `bg-surface`,
  `text-foreground`, `text-muted-foreground`, `bg-primary`,
  `border-border`, `bg-board-light`, …
- **Every theme defines every variable.** The light values sit in the
  `@theme { … }` block; the dark values override them in the
  `@media (prefers-color-scheme: dark)` block — keep both lists in sync
  (same tokens, same order). Add a new token = add it to BOTH blocks + a PR
  the team sees.
- Dark mode follows the **system** setting automatically (the
  `prefers-color-scheme` media query overrides the variables, and every
  utility references them). Preview it with the OS appearance switch or the
  browser devtools rendering emulation. Manual switching, if ever wanted, is
  a team PR (a `.dark` class variant would be added then).
- Components NEVER read or set colors themselves — they use semantic
  classNames. This folder is documentation; the config is `global.css`.

## Token reference

| Token | Use |
|---|---|
| `background` / `surface` / `surface-muted` | screen / card / inset backgrounds |
| `foreground` / `muted-foreground` | primary / secondary text |
| `primary` / `primary-foreground` | brand actions |
| `border` | dividers, outlines |
| `destructive` / `destructive-foreground` / `success` | resign / confirm semantics |
| `board-light` / `board-dark` / `move-target` / `last-move` / `check` / `piece-white` / `piece-black` | chess board (F2) |
