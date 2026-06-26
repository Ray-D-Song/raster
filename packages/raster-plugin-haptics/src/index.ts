import { registerPlugin } from "@raster/plugin-core";

export type ImpactStyle = "light" | "medium" | "heavy";

export interface HapticsPlugin {
  impact(options?: { style?: ImpactStyle }): Promise<{ ok: true }>;
  vibrate(options?: { duration?: number }): Promise<{ ok: true }>;
}

export const Haptics = registerPlugin<HapticsPlugin>("Haptics");