import type { RasterResolvedTheme } from "./types/index.js";

type RasterThemeRuntimeGlobal = typeof globalThis & {
  __rasterNative?: {
    getTheme?(): string | RasterResolvedTheme;
  };
};

export function useTheme(): RasterResolvedTheme | null {
  const nativeTheme = (globalThis as RasterThemeRuntimeGlobal).__rasterNative?.getTheme?.();
  if (nativeTheme == null) {
    return null;
  }
  if (typeof nativeTheme !== "string") {
    return isResolvedTheme(nativeTheme) ? nativeTheme : null;
  }
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
