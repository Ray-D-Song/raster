import { useSyncExternalStore } from "react";

import { addRasterRuntimeEventListener } from "./runtime-events.js";
import type { RasterResolvedTheme } from "./types/index.js";

type RasterThemeRuntimeGlobal = typeof globalThis & {
  __rasterNative?: {
    getTheme?(): string | RasterResolvedTheme;
  };
};

export function useTheme(): RasterResolvedTheme | null {
  const nativeTheme = useSyncExternalStore(subscribeThemeChange, readNativeThemeSnapshot, readServerThemeSnapshot);
  return parseNativeTheme(nativeTheme);
}

function subscribeThemeChange(onChange: () => void): () => void {
  return addRasterRuntimeEventListener("themechange", onChange);
}

function readServerThemeSnapshot(): string {
  return "";
}

function readNativeThemeSnapshot(): string {
  const nativeTheme = (globalThis as RasterThemeRuntimeGlobal).__rasterNative?.getTheme?.();
  if (nativeTheme == null) {
    return "";
  }
  if (typeof nativeTheme !== "string") {
    return JSON.stringify(nativeTheme);
  }
  return nativeTheme;
}

function parseNativeTheme(nativeTheme: string): RasterResolvedTheme | null {
  if (nativeTheme.length === 0) return null;
  try {
    const theme = JSON.parse(nativeTheme);
    return isResolvedTheme(theme) ? theme : null;
  } catch {
    return null;
  }
}

function isResolvedTheme(value: unknown): value is RasterResolvedTheme {
  if (value == null || typeof value !== "object") {
    return false;
  }
  const theme = value as Partial<RasterResolvedTheme>;
  return (
    (theme.mode === "light" || theme.mode === "dark") &&
    typeof theme.colors?.background === "string" &&
    typeof theme.colors.foreground === "string" &&
    typeof theme.colors.primary === "string"
  );
}
