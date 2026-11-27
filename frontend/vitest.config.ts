import { fileURLToPath } from "node:url";

import { defineConfig } from "vitest/config";

/**
 * Vitest — pure-logic unit tests only (protocol codec, stores, revision
 * guards — see RULES.md §5). UI is verified manually (each module's
 * manual-testing doc). Needs a DOM later? Propose jsdom via a team PR.
 */
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}"],
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
});
