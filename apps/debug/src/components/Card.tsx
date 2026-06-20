import type { ReactNode } from "react";
import { View } from "raster-js/components";
import type { RasterStyle } from "raster-js/components";
import { type AppTheme, borderColor, panelBackground } from "../styles";


interface CardProps {
  children: ReactNode;
  theme: AppTheme;
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
