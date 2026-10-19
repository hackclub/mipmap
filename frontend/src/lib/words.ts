import { tiers } from "$lib/projects/tiers";

export const words = [
  { name: "mipmap", colour: "#8fcf6b", match: "(?:hack club )?mipmap" },
  { name: "mip", colour: "#ff9a3d", match: "mips?" },
  {
    name: "tier",
    colour: "#ffe066",
    match: `tiers?(?: (?:${tiers.map((tier) => tier.id).join("|")}))?`,
  },
  { name: "tech-tree", colour: "#7cc4f0", match: "tech trees?" },
  { name: "token", colour: "#c3a3e6", match: "tokens?" },
  { name: "prize", colour: "#f0788a", match: "prizes?" },
  { name: "snapshot", colour: "#e8c58c", match: "snapshots?" },
  { name: "hackatime", colour: "#d6ab70", match: "hackatime" },
];

const pattern = new RegExp(
  `\\b(?:${words.map((word) => `(${word.match})`).join("|")})\\b`,
  "gi",
);
const skip = "a, script, style, noscript, textarea, [data-plain]";
const key = (name: string) => `word-${name}`;

function scan() {
  const ranges = words.map(() => [] as Range[]);
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) =>
      node.parentElement?.closest(skip)
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT,
  });
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    for (const match of node.nodeValue!.matchAll(pattern)) {
      const range = new Range();
      range.setStart(node, match.index);
      range.setEnd(node, match.index + match[0].length);
      ranges[match.slice(1).findIndex(Boolean)].push(range);
    }
  }
  words.forEach((word, i) => CSS.highlights.set(key(word.name), new Highlight(...ranges[i])));
}

export function colour_words() {
  if (!("highlights" in CSS)) return () => {};
  const style = document.createElement("style");
  style.textContent = words
    .map((word) => `::highlight(${key(word.name)}) { color: ${word.colour}; }`)
    .join("\n");
  document.head.append(style);
  let frame = 0;
  const observer = new MutationObserver(() => {
    frame ||= requestAnimationFrame(() => {
      frame = 0;
      scan();
    });
  });
  observer.observe(document.body, { childList: true, characterData: true, subtree: true });
  scan();
  return () => {
    observer.disconnect();
    cancelAnimationFrame(frame);
    style.remove();
    for (const word of words) CSS.highlights.delete(key(word.name));
  };
}
