import { site_url } from "$lib/SiteHead.svelte";

export const prerender = true;

const pages = ["/", "/docs", "/stats"];

const xml = (text: string) =>
  text.replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&apos;" })[c]!,
  );

export const GET = () => {
  if (!site_url) console.warn("VITE_SITE_URL is not set, so the sitemap has no pages");
  const urls = site_url
    ? pages.map((page) => `  <url><loc>${xml(site_url + page)}</loc></url>\n`).join("")
    : "";
  return new Response(
    `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}</urlset>
`,
    { headers: { "Content-Type": "application/xml" } },
  );
};
