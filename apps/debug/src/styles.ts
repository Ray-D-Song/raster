import type { BoxShadowInput, BoxShadowPreset, RasterResolvedThemeColors, RasterStyle } from "raster-js/components";
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
    secondaryHover: vitalityColors.secondaryHover,
    secondaryActive: vitalityColors.secondaryActive,
    accent: vitalityColors.primaryContainer,
    accentForeground: vitalityColors.onPrimaryContainer,
    muted: vitalityColors.surfaceContainerLow,
    mutedForeground: vitalityColors.onSurfaceVariant,
    popover: "#ffffff",
    popoverForeground: vitalityColors.onSurface,
    danger: vitalityColors.error,
    success: vitalityColors.primaryContainer,
    info: vitalityColors.secondary,
    switch: vitalityColors.outlineVariant,
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

/** Level 1 — cards, list items, input panels */
export const cardShadow: BoxShadowPreset = "md";

/** Teal ambient glow for tinted bento surfaces (design ambient-shadow) */
export const ambientShadow: BoxShadowInput = {
  offsetY: 10,
  blurRadius: 30,
  spreadRadius: -10,
  color: "rgba(0, 107, 95, 0.08)",
};

/** Level 2 — hero widgets, primary CTAs, highlighted cards */
export const elevatedShadow: BoxShadowPreset = "xl";

/** Bottom navigation bar */
export const tabBarShadow: BoxShadowPreset = "md";