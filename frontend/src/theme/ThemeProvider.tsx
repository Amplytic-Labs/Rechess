/**
 * Theme provider — the ONLY place that maps light/dark to token values.
 *
 * Tokens are defined in tailwind.config.js as CSS variables (`--color-*`).
 * This provider overrides those variables per color scheme using vars(),
 * so every component below it just uses semantic classNames like
 * `bg-surface`, `text-foreground`, `bg-primary` — in both light and dark.
 *
 * RULES.md: components never read or set colors themselves; they use tokens.
 */
import { DarkTheme, DefaultTheme, ThemeProvider } from "expo-router";
import { useColorScheme, vars } from "nativewind";
import type { PropsWithChildren } from "react";
import { View } from "react-native";

const schemes = {
  light: vars({
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
  }),
  dark: vars({
    "--color-background": "24 24 27",
    "--color-surface": "39 39 42",
    "--color-surface-muted": "52 52 56",
    "--color-foreground": "244 244 245",
    "--color-muted-foreground": "161 161 170",
    "--color-primary": "129 140 248",
    "--color-primary-foreground": "24 24 27",
    "--color-border": "63 63 70",
    "--color-destructive": "248 113 113",
    "--color-success": "74 222 128",
    "--color-board-light": "112 136 106",
    "--color-board-dark": "56 72 56",
    "--color-move-target": "104 211 145",
    "--color-last-move": "250 204 21",
    "--color-check": "239 68 68",
  }),
} as const;

export function AppThemeProvider({ children }: PropsWithChildren) {
  // nativewind's useColorScheme also exposes setColorScheme/toggleColorScheme
  // if a manual in-app theme toggle is wanted later.
  const { colorScheme } = useColorScheme();
  const scheme = colorScheme === "dark" ? "dark" : "light";
  return (
    // The style prop is used ONLY for vars() — the documented NativeWind
    // theming pattern (variables inherit down the tree). Everything else
    // must use className (see RULES.md).
    <ThemeProvider value={colorScheme === "dark" ? DarkTheme : DefaultTheme}>
      <View style={schemes[scheme]} className="flex-1 bg-background">
        {children}
      </View>
    </ThemeProvider>
  );
}
