import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      "@niupanel/plugin-sdk": fileURLToPath(
        new URL("../../../../packages/plugin-sdk/src", import.meta.url),
      ),
    },
  },
  build: {
    emptyOutDir: true,
    lib: {
      entry: "src/plugin.ts",
      formats: ["es"],
      fileName: () => "niupanel-plugin.js",
    },
    rollupOptions: {
      external: [],
      output: {
        assetFileNames: "assets/[name][extname]",
      },
    },
  },
});
