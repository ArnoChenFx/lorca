// The window's shape. A wide window (a tablet, or a tablet window with room to spare) shows the
// chat list as a sidebar beside the open chat; a narrow one stacks the chat over the list. The
// sidebar can be hidden so the chat pane uses the full width.

import { CompactWidthView } from "../../modules/lorca-core/CompactWidthView";
import { createContext, use, useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { StyleSheet, useWindowDimensions, View } from "react-native";
import { Sidebar } from "./Sidebar";
import { usePalette } from "./theme";

/// An iPad mini in portrait (744 pt) is wide; half of an 11-inch iPad's screen is not.
const WIDE_MIN = 700;

export function useWide(): boolean {
  return useWindowDimensions().width >= WIDE_MIN;
}

export function useSidebarWidth(): number {
  return useWindowDimensions().width >= 1100 ? 375 : 320;
}

const PaneWidthContext = createContext<number | null>(null);

/// The width of the pane its children are laid out in, where that is not the window's.
export const PaneWidth = PaneWidthContext.Provider;

export function usePaneWidth(): number {
  const pane = use(PaneWidthContext);
  const { width } = useWindowDimensions();
  return pane ?? width;
}

type SidebarLayoutValue = {
  wide: boolean;
  collapsed: boolean;
  showsSidebar: boolean;
  toggleSidebar: () => void;
};

const SidebarLayoutContext = createContext<SidebarLayoutValue | null>(null);

export function useSidebarLayout(): SidebarLayoutValue {
  const value = use(SidebarLayoutContext);
  if (!value) throw new Error("useSidebarLayout must be used under WideMainPanes");
  return value;
}

/// The row of sidebar and main pane. Hides the sidebar when the user collapses it on a wide window.
export function WideMainPanes({ children }: { children: ReactNode }) {
  const p = usePalette();
  const wide = useWide();
  const sidebarWidth = useSidebarWidth();
  const { width } = useWindowDimensions();
  const [collapsed, setCollapsed] = useState(false);
  useEffect(() => {
    if (!wide) setCollapsed(false);
  }, [wide]);
  const showsSidebar = wide && !collapsed;
  const paneWidth = showsSidebar ? width - sidebarWidth : width;
  const toggleSidebar = useCallback(() => setCollapsed((on) => !on), []);
  const layout = useMemo(
    () => ({ wide, collapsed, showsSidebar, toggleSidebar }),
    [wide, collapsed, showsSidebar, toggleSidebar],
  );
  return (
    <SidebarLayoutContext value={layout}>
      <View style={styles.panes}>
        {showsSidebar ? (
          <CompactWidthView style={[styles.sidebar, { width: sidebarWidth, borderRightColor: p.separator as string }]}>
            <Sidebar />
          </CompactWidthView>
        ) : null}
        <View style={styles.pane}>
          <PaneWidth value={paneWidth}>{children}</PaneWidth>
        </View>
      </View>
    </SidebarLayoutContext>
  );
}

const styles = StyleSheet.create({
  panes: { flex: 1, flexDirection: "row" },
  sidebar: { borderRightWidth: StyleSheet.hairlineWidth },
  pane: { flex: 1 },
});
