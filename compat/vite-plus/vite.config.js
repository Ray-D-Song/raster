import { defineConfig } from "vite-plus";

export default defineConfig({
  build: {
    lib: {
      entry: "src/index.tsx",
      formats: ["es", "cjs"],
      fileName: (format) => format === "es" ? "index.js" : "index.cjs",
      cssFileName: "style"
    },
    manifest: true,
    rollupOptions: {
      external: ["react", "react/jsx-runtime"]
    }
  }
});
