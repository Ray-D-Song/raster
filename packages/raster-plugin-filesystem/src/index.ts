import { registerPlugin } from "@raster/plugin-core";

export interface FilesystemPlugin {
  getCacheDirectory(): Promise<{ uri: string }>;
  readText(options: { uri: string }): Promise<{ text: string }>;
  writeText(options: { uri: string; text: string }): Promise<{ uri: string }>;
}

export const Filesystem = registerPlugin<FilesystemPlugin>("Filesystem");