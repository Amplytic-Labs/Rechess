import { Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

/**
 * Placeholder home screen — proves the base app + theming + Expo Router run.
 * F1 replaces this with the real lobby home screen
 * (see src/screens/lobby/README.md).
 */
export default function HomeScreen() {
  return (
    <SafeAreaView className="flex-1">
      <View className="flex-1 items-center justify-center gap-3 px-6">
        <Text className="text-4xl font-bold text-foreground">Rechess</Text>
        <Text className="text-center text-base text-muted-foreground">
          Team base app. F1 will replace this screen with the lobby.
        </Text>
        <Text className="text-sm text-muted-foreground">
          Frontend rules: see RULES.md · Your module: src/screens/lobby/README.md
        </Text>
      </View>
    </SafeAreaView>
  );
}
