import type { ReactNode } from "react";
import { View } from "raster-js/components";
import type { RasterStyle } from "raster-js/components";
import { type AppTheme, borderColor, panelBackground } from "../styles";

interface CardProps {
  children: ReactNode;
  theme: AppTheme;
  style?: RasterStyle;
  tinted?: boolean;
}

export function Card({ children, theme, style, tinted = false }: CardProps) {
  return (
    <View
      style={{
        backgroundColor: tinted ? "rgba(45, 212, 191, 0.12)" : panelBackground(theme),
        borderColor: borderColor(theme),
        borderWidth: 1,
        borderRadius: 24,
        padding: 20,
        ...style,
      }}
    >
      {children}
    </View>
  );
}