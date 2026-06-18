import type { ReactNode } from "react";
import { View } from "raster-js/components";
import type { RasterStyle } from "raster-js/components";
import { borderColor, panelBackground } from "../styles";
import type { ThemePreference } from "../types";

interface CardProps {
  children: ReactNode;
  theme: ThemePreference;
  style?: RasterStyle;
}

export function Card({ children, theme, style }: CardProps) {
  return (
    <View
      style={{
        backgroundColor: panelBackground(theme),
        borderColor: borderColor(theme),
        borderWidth: 1,
        borderRadius: 8,
        padding: 14,
        ...style,
      }}
    >
      {children}
    </View>
  );
}
