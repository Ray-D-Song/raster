export interface IconifyIcon {
  body: string;
  width?: number;
  height?: number;
  left?: number;
  top?: number;
}

export type IconSrc = IconifyIcon | string;

export type JsonObject = { [key: string]: unknown };

export function iconifyIconToSvg(
  icon: IconifyIcon,
  defaults: { width?: number; height?: number } = { width: 24, height: 24 }
): string {
  const width = icon.width ?? defaults.width ?? 24;
  const height = icon.height ?? defaults.height ?? 24;
  const left = icon.left ?? 0;
  const top = icon.top ?? 0;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${left} ${top} ${width} ${height}">${icon.body}</svg>`;
}

export function normalizeIconSrc(src: IconSrc): string | undefined {
  if (typeof src === "string") {
    const trimmed = src.trim();
    if (trimmed.length === 0) return undefined;
    return trimmed;
  }
  if (typeof src.body === "string" && src.body.length > 0) {
    return iconifyIconToSvg(src);
  }
  return undefined;
}

export function attachIconSvgProp(
  props: JsonObject,
  sourceKey: string,
  targetKey = `${sourceKey}Svg`
): void {
  if (!(sourceKey in props)) {
    return;
  }
  const raw = props[sourceKey];
  delete props[sourceKey];
  if (raw == null) {
    return;
  }
  const svg = normalizeIconSrc(raw as IconSrc);
  if (svg != null) {
    props[targetKey] = svg;
  }
}