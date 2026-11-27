const { getDefaultConfig } = require("expo/metro-config");
const { withUniwindConfig } = require("uniwind/metro");

const config = getDefaultConfig(__dirname);

// uniwind + Tailwind CSS v4. `cssEntryFile` must be a relative path string.
// `dtsFile` adds `className` types for RN core components — generated when
// Metro runs (the first `npm start` / `expo export` creates it; commit it).
module.exports = withUniwindConfig(config, {
  cssEntryFile: "./global.css",
  dtsFile: "./src/uniwind-types.d.ts",
});
