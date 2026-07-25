import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    host: "0.0.0.0",
    port: 3000,
    proxy: {
      "/v1": {
        target: process.env.DRP_API_PROXY || "http://127.0.0.1:8080",
        changeOrigin: true,
      },
      "/readyz": {
        target: process.env.DRP_API_PROXY || "http://127.0.0.1:8080",
        changeOrigin: true,
      },
      "/livez": {
        target: process.env.DRP_API_PROXY || "http://127.0.0.1:8080",
        changeOrigin: true,
      },
      "/metrics": {
        target: process.env.DRP_API_PROXY || "http://127.0.0.1:8080",
        changeOrigin: true,
      },
      "/health": {
        target: process.env.DRP_API_PROXY || "http://127.0.0.1:8080",
        changeOrigin: true,
      },
    },
  },
});
