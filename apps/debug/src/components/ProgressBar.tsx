import { View } from "raster-js/components";
import { borderColor } from "../styles";
import type { ThemePreference } from "../types";

interface ProgressBarProps {
  value: number;
  color: string;
  theme: ThemePreference;
}

export function ProgressBar({ value, color, theme }: ProgressBarProps) {
  const width = `${Math.max(3, Math.min(100, Math.round(value * 100)))}%` as `${number}%`;
  return (
    <View
      style={{
        width: "100%",
        height: 8,
        borderRadius: 4,
        backgroundColor: borderColor(theme),
        overflow: "hidden",
      }}
    >
      <View style={{ width, height: 8, borderRadius: 4, backgroundColor: color }} />
    </View>
  );
}
