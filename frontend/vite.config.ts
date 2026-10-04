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
      "/api": "http://0.0.0.0:8080",
      "/login": "http://0.0.0.0:8080",
      "/callback": "http://0.0.0.0:8080",
      "/dashboard/tree.json": "http://0.0.0.0:8080",
      "/dashboard/layout.json": "http://0.0.0.0:8080",
    },
  },
});
