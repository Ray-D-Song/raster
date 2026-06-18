import type { RasterStyle } from "raster-js/components";
import type { ThemePreference } from "./types";

export const colors = {
  ink: "#111827",
  muted: "#6b7280",
  faint: "#9ca3af",
  line: "#e5e7eb",
  softLine: "#f1f5f9",
  panel: "#ffffff",
  canvas: "#f8fafc",
  blue: "#2563eb",
  green: "#15803d",
  red: "#dc2626",
  amber: "#b45309",
};

export const appBackground = (theme: ThemePreference): string => (theme === "dark" ? "#111827" : colors.canvas);
export const panelBackground = (theme: ThemePreference): string => (theme === "dark" ? "#1f2937" : colors.panel);
export const textColor = (theme: ThemePreference): string => (theme === "dark" ? "#f9fafb" : colors.ink);
export const secondaryText = (theme: ThemePreference): string => (theme === "dark" ? "#cbd5e1" : colors.muted);
export const borderColor = (theme: ThemePreference): string => (theme === "dark" ? "#374151" : colors.line);

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
