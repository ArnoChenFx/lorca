import { Stack } from "expo-router";
import { Platform } from "react-native";
import { t, useLanguage } from "../../src/i18n";
import { useWide, WideMainPanes } from "../../src/ui/layout";
import { useStackScreenOptions } from "../../src/ui/navigation";
import { usePalette } from "../../src/ui/theme";

// A chat opened from a link or a notification still has the list under it.
export const unstable_settings = { initialRouteName: "(tabs)" };

/// The tabs and the open chat. A narrow window stacks the chat over the tabs. A wide one keeps
/// the list in a sidebar, and this stack is the pane beside it: its first screen is the empty
/// pane (the tabs with their bar hidden) and a chat takes the place of the one before it.
export default function MainLayout() {
  useLanguage();
  const p = usePalette();
  const wide = useWide();
  const screenOptions = useStackScreenOptions();

  return (
    <WideMainPanes borderColor={p.separator}>
      <Stack screenOptions={screenOptions}>
        <Stack.Screen name="(tabs)" options={{ title: t("Chats"), headerShown: false }} />
        <Stack.Screen
          name="chat/[id]"
          options={{
            ...(Platform.OS === "android" ? { headerShown: false } : { headerTransparent: true, headerShadowVisible: false, headerTitleAlign: "center" as const }),
            // Beside the sidebar there is nothing to go back to.
            ...(wide ? { headerBackVisible: false, gestureEnabled: false, animation: "none" as const } : null),
          }}
        />
      </Stack>
    </WideMainPanes>
  );
}
