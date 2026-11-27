import "../global.css";

import { Stack } from "expo-router";
import * as SplashScreen from "expo-splash-screen";
import { cssInterop } from "nativewind";
import { useEffect } from "react";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { AppThemeProvider } from "@/theme/ThemeProvider";

// Third-party components accept className only after a one-time cssInterop
// mapping at the app root (see frontend/RULES.md §Styling). This is the
// documented NativeWind pattern for non-core components.
cssInterop(GestureHandlerRootView, { className: "style" });

SplashScreen.preventAutoHideAsync();

export default function RootLayout() {
  useEffect(() => {
    SplashScreen.hideAsync();
  }, []);

  return (
    <GestureHandlerRootView className="flex-1">
      <SafeAreaProvider>
        <AppThemeProvider>
          {/* Screens are added by F1/F4 as src/app/*.tsx route files — see
              src/screens/lobby/README.md for the route plan. */}
          <Stack>
            <Stack.Screen name="index" options={{ headerShown: false }} />
          </Stack>
        </AppThemeProvider>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}
