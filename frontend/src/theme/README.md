# `src/theme/` — design tokens (shared, F1 curates changes via PR)

- `ThemeProvider.tsx` is the ONLY place that maps light/dark to values. It
  wraps the app in `src/app/_layout.tsx` and overrides the `--color-*` CSS
  variables per color scheme with NativeWind's `vars()`.
- Token names + light defaults live in `tailwind.config.js` (the
  `colors`/`plugins` sections). Both places must stay in sync.
- Components NEVER read colors from here directly — they use classNames
  (`bg-surface`, `text-muted-foreground`, …). This folder is configuration,
  not an API.

Adding/changing a token = PR (whole team sees it) + update RULES.md's token
table if the meaning is new.
