import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
  ]),
  // RULES.md §1.2 — no hardcoded colors in classNames: use the design
  // tokens (bg-background, text-foreground, bg-board-light, …) from
  // global.css. Arbitrary color literals like bg-[#fff] are banned.
  {
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector: "Literal[value=/-[#]/]",
          message:
            "Arbitrary color literal — use a design token from global.css (bg-background, text-foreground, bg-board-light, …) instead (RULES.md §1.2).",
        },
      ],
    },
  },
]);

export default eslintConfig;
