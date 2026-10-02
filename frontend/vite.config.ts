import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    sveltekit({
      compilerOptions: {
        // Force runes mode for the project, except for libraries. Can be removed in svelte 6.
        runes: ({ filename }) =>
          filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
      },

      adapter: adapter(),
    }),
  ],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:8080",
      "/login": "http://127.0.0.1:8080",
      "/callback": "http://127.0.0.1:8080",
      "/dashboard/tree.json": "http://127.0.0.1:8080",
      "/dashboard/layout.json": "http://127.0.0.1:8080",
      "/dashboard/hackatime-projects.json": "http://127.0.0.1:8080",
      "/dashboard/fulfillment-rounds.json": "http://127.0.0.1:8080",
    },
  },
});
