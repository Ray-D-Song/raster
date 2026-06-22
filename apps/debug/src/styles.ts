import type { RasterResolvedThemeColors, RasterStyle } from "raster-js/components";
import type { RasterThemeConfig } from "raster-js/components";
import { vitalityColors } from "./data";

export type AppTheme = RasterResolvedThemeColors;

export const vitalityTheme: RasterThemeConfig = {
  mode: "light",
  radius: 16,
  radiusLg: 24,
  fontFamily: "Inter",
  colors: {
    background: vitalityColors.background,
    foreground: vitalityColors.onSurface,
    border: vitalityColors.outlineVariant,
    primary: vitalityColors.primary,
    primaryForeground: "#ffffff",
    secondary: vitalityColors.secondary,
    secondaryForeground: "#ffffff",
    accent: vitalityColors.primaryContainer,
    accentForeground: vitalityColors.onPrimaryContainer,
    muted: vitalityColors.surfaceContainerLow,
    mutedForeground: vitalityColors.onSurfaceVariant,
    popover: "#ffffff",
    popoverForeground: vitalityColors.onSurface,
    danger: vitalityColors.error,
    success: vitalityColors.primaryContainer,
    info: vitalityColors.secondary,
  },
};

export const panelBackground = (_theme: AppTheme): string => "#ffffff";
export const secondaryText = (theme: AppTheme): string => theme.mutedForeground;
export const borderColor = (_theme: AppTheme): string => vitalityColors.surfaceContainer;

export const row: RasterStyle = {
  flexDirection: "row",
  alignItems: "center",
};

export const spaceBetween: RasterStyle = {
  flexDirection: "row",
  alignItems: "center",
  justifyContent: "space-between",
};

export const pagePadding: RasterStyle = {
  padding: { top: 20, right: 20, bottom: 96, left: 20 },
};

export const labelCaps: RasterStyle = {
  fontSize: 12,
  fontWeight: "600",
};