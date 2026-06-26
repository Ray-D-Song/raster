#!/usr/bin/env node
import path from "node:path";
import { linkPlugins } from "./index.js";

async function main(): Promise<void> {
  const root = process.cwd();
  const iosDir = process.env.RASTER_IOS_DIR;
  const androidDir = process.env.RASTER_ANDROID_DIR;
  const result = await linkPlugins({ root, iosDir, androidDir });
  console.log(`Linked ${result.plugins.length} Raster plugin(s).`);
  if (result.iosRegisterFile != null) {
    console.log(`iOS: ${result.iosRegisterFile}`);
  }
  if (result.androidRegisterFile != null) {
    console.log(`Android: ${result.androidRegisterFile}`);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});