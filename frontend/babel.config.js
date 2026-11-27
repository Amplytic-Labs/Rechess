module.exports = function (api) {
  api.cache(true);
  return {
    presets: [
      // NativeWind: className support + jsxImportSource. Reanimated's plugin
      // is included by babel-preset-expo automatically (SDK 58) — do not add
      // it again.
      ["babel-preset-expo", { jsxImportSource: "nativewind" }],
      "nativewind/babel",
    ],
  };
};
