import type { RasterResolvedThemeColors, RasterStyle } from "raster-js/components";

export type AppTheme = RasterResolvedThemeColors;

export const panelBackground = (theme: AppTheme): string => theme.popover;
export const secondaryText = (theme: AppTheme): string => theme.mutedForeground;
export const borderColor = (theme: AppTheme): string => theme.border;

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
