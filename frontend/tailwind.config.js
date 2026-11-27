/** @type {import('tailwindcss').Config} */
//
// Rechess design tokens — the single source of truth for colors.
// Values are space-separated RGB so they work with opacity modifiers
// (e.g. `bg-primary/50`) and with the CSS-variable overrides in
// src/theme/ThemeProvider.tsx (light/dark).
// Semantic tokens only: components must never hardcode colors.
module.exports = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  presets: [require("nativewind/preset")],
  theme: {
    extend: {
      colors: {
        background: "rgb(var(--color-background) / <alpha-value>)",
        surface: "rgb(var(--color-surface) / <alpha-value>)",
        "surface-muted": "rgb(var(--color-surface-muted) / <alpha-value>)",
        foreground: "rgb(var(--color-foreground) / <alpha-value>)",
        "muted-foreground": "rgb(var(--color-muted-foreground) / <alpha-value>)",
        primary: "rgb(var(--color-primary) / <alpha-value>)",
        "primary-foreground": "rgb(var(--color-primary-foreground) / <alpha-value>)",
        border: "rgb(var(--color-border) / <alpha-value>)",
        destructive: "rgb(var(--color-destructive) / <alpha-value>)",
        success: "rgb(var(--color-success) / <alpha-value>)",
        // chessboard-specific tokens (F2 owns their usage)
        "board-light": "rgb(var(--color-board-light) / <alpha-value>)",
        "board-dark": "rgb(var(--color-board-dark) / <alpha-value>)",
        "move-target": "rgb(var(--color-move-target) / <alpha-value>)",
        "last-move": "rgb(var(--color-last-move) / <alpha-value>)",
        check: "rgb(var(--color-check) / <alpha-value>)",
      },
    },
  },
  plugins: [
    // Light-mode defaults for the CSS variables (dark overrides are applied
    // at runtime by src/theme/ThemeProvider.tsx via vars()).
    ({ addBase }) =>
      addBase({
        ":root": {
          "--color-background": "250 250 249",
          "--color-surface": "255 255 255",
          "--color-surface-muted": "240 240 238",
          "--color-foreground": "28 28 30",
          "--color-muted-foreground": "120 118 113",
          "--color-primary": "79 70 229",
          "--color-primary-foreground": "255 255 255",
          "--color-border": "225 226 224",
          "--color-destructive": "220 38 38",
          "--color-success": "22 163 74",
          "--color-board-light": "235 236 208",
          "--color-board-dark": "115 149 82",
          "--color-move-target": "63 175 80",
          "--color-last-move": "255 213 79",
          "--color-check": "235 87 87",
        },
      }),
  ],
};
