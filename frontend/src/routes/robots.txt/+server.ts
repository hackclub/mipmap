import { site_url } from "$lib/SiteHead.svelte";

export const prerender = true;

export const GET = () =>
  new Response(
    `User-agent: *
Disallow: /dashboard
Disallow: /api/
Disallow: /login
Disallow: /callback
Disallow: /hackatime/
${site_url ? `\nSitemap: ${site_url}/sitemap.xml\n` : ""}`,
    { headers: { "Content-Type": "text/plain" } },
  );
