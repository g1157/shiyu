import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

function getNodeModulePackageName(id: string) {
  const normalizedId = id.replace(/\\/g, "/");
  const parts = normalizedId.split("/node_modules/");
  const packagePath = parts[parts.length - 1];
  const match = packagePath.match(/^((?:@[^/]+\/[^/]+)|[^/]+)/);
  return match ? match[1] : null;
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          const pkg = getNodeModulePackageName(id);

          if (pkg === "markmap-lib" || pkg === "markmap-common") {
            return "mindmap-core";
          }

          if (pkg === "markmap-view" || pkg === "d3" || pkg?.startsWith("d3-")) {
            return "mindmap-view-vendor";
          }

          if (pkg === "md-editor-v3") {
            return "markdown-editor-core";
          }

          if (
            pkg === "@codemirror/language-data" ||
            pkg === "@codemirror/legacy-modes" ||
            pkg?.startsWith("@codemirror/lang-") ||
            pkg?.startsWith("@lezer/")
          ) {
            return `editor-${pkg.replace("@", "").replace(/[\\/]/g, "-")}`;
          }

          if (
            pkg === "codemirror" ||
            pkg?.startsWith("@codemirror/")
          ) {
            return "markdown-editor-vendor";
          }

          if (
            pkg === "highlight.js" ||
            pkg === "markdown-it" ||
            pkg?.startsWith("markdown-it-") ||
            pkg === "@vscode/markdown-it-katex"
          ) {
            return "markdown-render-vendor";
          }

          if (pkg === "epubjs") {
            return "epub-vendor";
          }
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
