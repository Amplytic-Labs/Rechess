import "../../global.css";

import { Stack } from "expo-router";
import * as SplashScreen from "expo-splash-screen";
import { useEffect } from "react";

import { HeroUINativeProvider } from "heroui-native/provider";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { StyledGestureHandlerRootView } from "@/components/common/styled";

SplashScreen.preventAutoHideAsync();

export default function RootLayout() {
  useEffect(() => {
    SplashScreen.hideAsync();
  }, []);

  return (
    // HeroUI Native requires GestureHandlerRootView as the outermost
    // wrapper, with HeroUINativeProvider as its direct child.
    <StyledGestureHandlerRootView className="flex-1">
      <SafeAreaProvider>
        <HeroUINativeProvider>
          {/* Screens are added by F1/F4 as src/app/*.tsx route files — see
              src/screens/lobby/README.md for the route plan. Theming needs
              no provider: tokens live in global.css and follow the system
              theme (see RULES.md §1.3). */}
          <Stack>
            <Stack.Screen name="index" options={{ headerShown: false }} />
          </Stack>
        </HeroUINativeProvider>
      </SafeAreaProvider>
    </StyledGestureHandlerRootView>
  );
}
