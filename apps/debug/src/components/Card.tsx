import type { ReactNode } from "react";
import { View } from "raster-js/components";
import type { RasterStyle } from "raster-js/components";
import { vitalityColors } from "../data";
import { type AppTheme, ambientShadow, borderColor, cardShadow, panelBackground } from "../styles";

export type CardVariant = "default" | "tinted" | "accent";

interface CardProps {
  children: ReactNode;
  theme: AppTheme;
  style?: RasterStyle;
  /** @deprecated Use `variant="tinted"` */
  tinted?: boolean;
  variant?: CardVariant;
}

function resolveVariant(variant: CardVariant | undefined, tinted: boolean): CardVariant {
  if (variant != null) return variant;
  return tinted ? "tinted" : "default";
}

function variantStyle(variant: CardVariant, theme: AppTheme): RasterStyle {
  switch (variant) {
    case "accent":
      return {
        backgroundColor: vitalityColors.primaryContainer,
        borderWidth: 0,
        boxShadow: cardShadow,
      };
    case "tinted":
      return {
        backgroundColor: "rgba(45, 212, 191, 0.1)",
        borderColor: "rgba(45, 212, 191, 0.2)",
        borderWidth: 1,
        boxShadow: ambientShadow,
      };
    default:
      return {
        backgroundColor: panelBackground(theme),
        borderColor: borderColor(theme),
        borderWidth: 1,
        boxShadow: cardShadow,
      };
  }
}

export function Card({ children, theme, style, tinted = false, variant }: CardProps) {
  const resolved = resolveVariant(variant, tinted);

  return (
    <View
      style={{
        ...variantStyle(resolved, theme),
        borderRadius: 24,
        padding: 20,
        ...style,
      }}
    >
      {children}
    </View>
  );
}