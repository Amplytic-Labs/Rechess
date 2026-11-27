// ESLint flat config — Expo base + the binding rules from RULES.md.
// These rules are enforced automatically; RULES.md explains the why.
// Run: npm run lint
const reactNative = require("eslint-plugin-react-native");

module.exports = [
  ...require("eslint-config-expo/flat"),
  {
    files: ["src/**/*.{js,jsx,ts,tsx}"],
    plugins: { "react-native": reactNative },
    rules: {
      // RULES.md §2: style with NativeWind className only.
      "react-native/no-inline-styles": "error",
      "react-native/no-color-literals": "error",
    },
  },
  {
    files: ["src/**/*.{js,jsx,ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "react-native",
              importNames: ["StyleSheet"],
              message: "RULES.md: no StyleSheet objects — style with NativeWind className.",
            },
            {
              name: "expo-status-bar",
              message: "RULES.md: do not mount StatusBar in code — status bar style is configured once in app.json.",
            },
          ],
          patterns: [
            {
              group: ["react-navigation", "react-navigation/*", "@react-navigation/*"],
              message: "RULES.md: navigation goes through expo-router only.",
            },
          ],
        },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector: "CallExpression[callee.object.name='StyleSheet'][callee.property.name='create']",
          message: "RULES.md: no StyleSheet.create — style with NativeWind className.",
        },
      ],
    },
  },
  {
    ignores: [".expo/**", "node_modules/**", "dist/**"],
  },
];
