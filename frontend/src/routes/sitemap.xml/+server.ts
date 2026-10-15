import { site_url } from "$lib/SiteHead.svelte";

export const prerender = true;

const pages = ["/", "/docs", "/stats"];

export const GET = () =>
  new Response(
    `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${pages.map((page) => `  <url><loc>${site_url}${page}</loc></url>`).join("\n")}
</urlset>
`,
    { headers: { "Content-Type": "application/xml" } },
  );
