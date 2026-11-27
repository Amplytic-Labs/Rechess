# `src/theme/` — design tokens (shared, F1 curates changes via PR)

- There is **no ThemeProvider component** — uniwind theming is CSS-first.
  All tokens live in `global.css` (repo root) inside
  `@layer theme { :root { @variant light {…} @variant dark {…} } }`, named
  `--color-<token>` so they become Tailwind utilities:
  `bg-background`, `bg-surface`, `text-foreground`, `text-muted-foreground`,
  `bg-primary`, `border-border`, `bg-board-light`, …
- **Every theme defines every variable** (uniwind warns in dev otherwise).
  Add a new token = add it to BOTH `@variant` blocks + a PR the team sees.
- Dark mode: `light`/`dark` are built-in and the default theme is `system`
  (follows the device). Manual switching (if wanted later):
  `Uniwind.setTheme('dark' | 'light' | 'system')`; read the active theme with
  `useUniwind()` → `{ theme, hasAdaptiveThemes }`
  (see https://docs.uniwind.dev/theming/basics).
- Components NEVER read or set colors themselves — they use semantic
  classNames. This folder is documentation; the config is `global.css`.

## Token reference

| Token | Use |
|---|---|
| `background` / `surface` / `surface-muted` | screen / card / inset backgrounds |
| `foreground` / `muted-foreground` | primary / secondary text |
| `primary` / `primary-foreground` | brand actions |
| `border` | dividers, outlines |
| `destructive` / `success` | resign / confirm semantics |
| `board-light` / `board-dark` / `move-target` / `last-move` / `check` | chess board (F2) |
