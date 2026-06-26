import { registerPlugin } from "@raster/plugin-core";

export interface ClipboardPlugin {
  getString(): Promise<{ value: string | null }>;
  setString(options: { value: string }): Promise<{ ok: true }>;
}

export const Clipboard = registerPlugin<ClipboardPlugin>("Clipboard");