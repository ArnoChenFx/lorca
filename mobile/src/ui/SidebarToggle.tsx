// Wide window: show or hide the chat-list sidebar beside the open chat.

import { Platform } from "react-native";
import { t, useLanguage } from "../i18n";
import { haptic } from "./haptics";
import { useSidebarLayout } from "./layout";
import { Pressable } from "./Pressable";
import { Symbol } from "./Symbol";
import { usePalette } from "./theme";

export function SidebarToggle({ size = 24 }: { size?: number }) {
  useLanguage();
  const p = usePalette();
  const { wide, collapsed, toggleSidebar } = useSidebarLayout();
  if (!wide) return null;
  const showList = collapsed;
  const label = showList ? t("Show chat list") : t("Hide chat list");
  const icon = showList ? "sidebar.left" : "sidebar.leadinghalf.filled";
  return (
    <Pressable
      onPress={() => {
        haptic.light();
        toggleSidebar();
      }}
      style={Platform.OS === "android" ? { width: 48, height: 48, alignItems: "center", justifyContent: "center" } : undefined}
      ripple={Platform.OS === "android" ? "borderless" : undefined}
      accessibilityRole="button"
      accessibilityLabel={label}
    >
      <Symbol name={icon} size={size} color={p.label} />
    </Pressable>
  );
}
