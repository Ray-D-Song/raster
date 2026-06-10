import raster from "unplugin-raster/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    raster({
      minify: false,
      sourcemap: true,
    }),
  ],
});
