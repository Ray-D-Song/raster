import raster from "unplugin-raster/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    raster({
      outfile: "../../target/raster/template/app.js",
      minify: false,
      sourcemap: true,
    }),
  ],
});
