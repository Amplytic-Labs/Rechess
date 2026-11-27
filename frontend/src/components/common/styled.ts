/**
 * One place for `withUniwind` interop wrappers: third-party components that
 * don't natively accept `className` get it here, wrapped ONCE at module
 * level (see uniwind docs — never wrap inside a render function).
 *
 * Extend this file when a third-party component needs className; RN core
 * components (View, Text, Pressable, FlatList…) support className already.
 */
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { SafeAreaView } from "react-native-safe-area-context";
import { withUniwind } from "uniwind";

// Root wrapper for the app tree (used in src/app/_layout.tsx).
export const StyledGestureHandlerRootView = withUniwind(GestureHandlerRootView);

// SafeAreaView from react-native-safe-area-context (core SafeAreaView is
// deprecated). Use this instead of importing SafeAreaView directly.
export const StyledSafeAreaView = withUniwind(SafeAreaView);
