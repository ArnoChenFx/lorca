import { Stack } from "expo-router";
import { Platform, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { t, useLanguage } from "../../../../src/i18n";
import { ChatsScreen } from "../../../../src/ui/ChatsScreen";
import { useWide } from "../../../../src/ui/layout";
import { SidebarToggle } from "../../../../src/ui/SidebarToggle";
import { Symbol } from "../../../../src/ui/Symbol";
import { usePalette } from "../../../../src/ui/theme";

/// The chat list, or the empty pane beside the sidebar that holds the list in a wide window.
export default function IndexScreen() {
  useLanguage();
  const p = usePalette();
  const wide = useWide();
  const insets = useSafeAreaInsets();
  if (!wide) return <ChatsScreen bar={Platform.OS !== "ios"} />;
  return (
    <View style={styles.empty}>
      <Stack.Screen options={{ headerShown: false }} />
      <View style={[styles.toggle, { top: insets.top + 4, left: insets.left + 4 }]}>
        <SidebarToggle />
      </View>
      <Symbol name="bubble.left.and.bubble.right" size={40} color={p.tertiaryLabel} />
      <Text style={[styles.text, { color: p.secondaryLabel }]}>{t("Select a chat")}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  empty: { flex: 1, alignItems: "center", justifyContent: "center", gap: 12 },
  toggle: { position: "absolute" },
  text: { fontSize: 17 },
});
