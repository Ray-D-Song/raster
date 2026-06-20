export type RasterThemeMode = "light" | "dark" | "system";

export interface RasterThemeColors {
  background?: string;
  foreground?: string;
  border?: string;
  input?: string;
  primary?: string;
  primaryForeground?: string;
  secondary?: string;
  secondaryForeground?: string;
  accent?: string;
  accentForeground?: string;
  muted?: string;
  mutedForeground?: string;
  popover?: string;
  popoverForeground?: string;
  ring?: string;
  danger?: string;
  success?: string;
  warning?: string;
  info?: string;
}

export interface RasterThemeConfig {
  mode?: RasterThemeMode;
  radius?: number;
  radiusLg?: number;
  fontSize?: number;
  fontFamily?: string;
  monoFontSize?: number;
  monoFontFamily?: string;
  colors?: RasterThemeColors;
}


