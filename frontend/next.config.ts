import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // The app is a client-side real-time UI over F3's WebSocket service: no
  // server data fetching, nothing worth caching — keep route semantics
  // simple (no cacheComponents / partialPrefetching constraints for the
  // team). The app is served by `next start` (see B4's deployment doc).
  cacheComponents: false,
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
