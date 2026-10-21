import { existsSync } from "node:fs";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const source =
  "https://fonts.googleapis.com/css2?family=Phudu:wght@300..900&family=Sansation:ital,wght@0,300;0,400;0,700;1,300;1,400;1,700&display=swap";
const agent =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const out = join(root, "static/fonts");
const css_path = join(out, "fonts.css");

const local = (css) => [...css.matchAll(/url\(\/fonts\/([^)]+)\)/g)].map((m) => m[1]);

async function cached() {
  if (!existsSync(css_path)) return false;
  const files = local(await readFile(css_path, "utf8"));
  return files.length > 0 && files.every((file) => existsSync(join(out, file)));
}

async function fetch_ok(url, init) {
  const response = await fetch(url, { ...init, signal: AbortSignal.timeout(30_000) });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response;
}

function name_for(block, subset) {
  const field = (key) =>
    block.match(new RegExp(`${key}:\\s*([^;]+);`))?.[1].trim().replace(/['"]/g, "") ?? "";
  return [field("font-family"), field("font-style"), field("font-weight"), subset]
    .join("-")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

async function download() {
  const css = await (await fetch_ok(source, { headers: { "User-Agent": agent } })).text();
  const files = new Map();
  const rewritten = css.replace(
    /(?:\/\*\s*([\w-]+)\s*\*\/\s*)?@font-face\s*{[^}]*}/g,
    (block, subset = "") =>
      block.replace(/url\((https:\/\/fonts\.gstatic\.com\/[^)]+\.woff2)\)/g, (_, url) => {
        let name = `${name_for(block, subset)}.woff2`;
        while (files.has(name) && files.get(name) !== url) name = name.replace(/\.woff2$/, "-x.woff2");
        files.set(name, url);
        return `url(/fonts/${name})`;
      }),
  );
  if (!files.size) throw new Error("the Google Fonts stylesheet had no woff2 files");
  await mkdir(out, { recursive: true });
  for (const [name, url] of files) {
    const path = join(out, name);
    if (existsSync(path)) continue;
    const bytes = Buffer.from(await (await fetch_ok(url)).arrayBuffer());
    await writeFile(`${path}.tmp`, bytes);
    await rename(`${path}.tmp`, path);
  }
  await writeFile(`${css_path}.tmp`, rewritten);
  await rename(`${css_path}.tmp`, css_path);
  return files.size;
}

if (await cached()) process.exit(0);
try {
  const count = await download();
  console.log(`fonts: saved ${count} font files to ${out}`);
} catch (e) {
  console.error(
    `fonts: couldn't download the Phudu and Sansation fonts from Google Fonts and no cached copy exists in ${out}.\n` +
      `fonts: ${e instanceof Error ? e.message : e}\n` +
      "fonts: check the network connection and run `node scripts/fonts.mjs` again.",
  );
  process.exit(1);
}
