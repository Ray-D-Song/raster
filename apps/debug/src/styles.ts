import type { RasterStyle, RasterThemeConfig } from "raster-js/components";
import type { ThemePreference } from "./types";

export const colors = {
  ink: "#09090b",
  muted: "#71717b",
  faint: "#9f9fa9",
  line: "#e4e4e7",
  softLine: "#f4f4f5",
  panel: "#ffffff",
  canvas: "#ffffff",
  blue: "#0069a8",
  green: "#0084d1",
  red: "#e7000b",
  amber: "#52525c",
  chart1: "#d4d4d8",
  chart2: "#71717b",
  chart3: "#52525c",
  chart4: "#3f3f46",
  chart5: "#27272a",
} as const;

const themeTokens = {
  light: {
    background: "#ffffff",
    foreground: "#09090b",
    card: "#ffffff",
    border: "#e4e4e7",
    mutedForeground: "#71717b",
    primary: "#0069a8",
    primaryForeground: "#f0f9ff",
    secondary: "#f4f4f5",
    secondaryForeground: "#18181b",
    danger: "#e7000b",
  },
  dark: {
    background: "#09090b",
    foreground: "#fafafa",
    card: "#18181b",
    border: "rgba(255, 255, 255, 0.1)",
    mutedForeground: "#9f9fa9",
    primary: "#00598a",
    primaryForeground: "#f0f9ff",
    secondary: "#27272a",
    secondaryForeground: "#fafafa",
    danger: "#ff6467",
  },
} as const;

const tokens = (theme: ThemePreference) => themeTokens[theme];

export const rasterTheme = (theme: ThemePreference): RasterThemeConfig => {
  const current = tokens(theme);
  return {
    mode: theme,
    radius: 8,
    radiusLg: 8,
    colors: {
      background: current.background,
      foreground: current.foreground,
      border: current.border,
      input: current.border,
      primary: current.primary,
      primaryForeground: current.primaryForeground,
      secondary: current.secondary,
      secondaryForeground: current.secondaryForeground,
      accent: current.secondary,
      accentForeground: current.secondaryForeground,
      muted: current.secondary,
      mutedForeground: current.mutedForeground,
      popover: current.card,
      popoverForeground: current.foreground,
      ring: current.mutedForeground,
      danger: current.danger,
    },
  };
};

export const appBackground = (theme: ThemePreference): string => tokens(theme).background;
export const panelBackground = (theme: ThemePreference): string => tokens(theme).card;
export const textColor = (theme: ThemePreference): string => tokens(theme).foreground;
export const secondaryText = (theme: ThemePreference): string => tokens(theme).mutedForeground;
export const borderColor = (theme: ThemePreference): string => tokens(theme).border;

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
  padding: { top: 18, right: 18, bottom: 18, left: 18 },
};
