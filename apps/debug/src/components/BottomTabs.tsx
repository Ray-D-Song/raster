import { Tab, TabBar, View } from "raster-js/components";
import type { IconName } from "raster-js/components";
import type { AppTab, ThemePreference } from "../types";
import { borderColor, panelBackground } from "../styles";

const tabs: Array<{ id: AppTab; label: string; icon: IconName }> = [
  { id: "overview", label: "Overview", icon: "layout-dashboard" },
  { id: "transactions", label: "Activity", icon: "file" },
  { id: "budget", label: "Budget", icon: "chart-pie" },
  { id: "settings", label: "Settings", icon: "circle-user" },
];

interface BottomTabsProps {
  tab: AppTab;
  onTabChange: (tab: AppTab) => void;
  theme: ThemePreference;
}

export function BottomTabs({ tab, onTabChange, theme }: BottomTabsProps) {
  const selectedIndex = Math.max(0, tabs.findIndex((item) => item.id === tab));
  return (
    <View
      style={{
        padding: { top: 8, right: 10, bottom: 10, left: 10 },
        borderTopWidth: 1,
        borderColor: borderColor(theme),
        backgroundColor: panelBackground(theme),
      }}
    >
      <TabBar selectedIndex={selectedIndex} onClick={(value) => onTabChange(tabs[Number(value)]?.id ?? "overview")}>
        {tabs.map((item) => (
          <Tab key={item.id} label={item.label} icon={item.icon} />
        ))}
      </TabBar>
    </View>
  );
}
