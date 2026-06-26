import { registerPlugin } from "@raster/plugin-core";

export interface __PLUGIN_NAME__Plugin {
  ping(): Promise<{ ok: boolean }>;
}

export const __PLUGIN_NAME__ = registerPlugin<__PLUGIN_NAME__Plugin>("__PLUGIN_NAME__");