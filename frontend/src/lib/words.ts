import { tiers } from "$lib/projects/tiers";

type Word = {
  name: string;
  colour: string;
  match: string;
  description: string | ((text: string) => string);
};

const tier_line = (tier: (typeof tiers)[number]) => `${tier.name}: ${tier.description}`;

export const words: Word[] = [
  {
    name: "mipmap",
    colour: "#8fcf6b",
    match: "(?:hack club )?mipmap",
    description: "Hack Club program where you make a game without an engine and earn prizes!",
  },
  {
    name: "mip",
    colour: "#ff9a3d",
    match: "mips?",
    description: "Currency for unlocking nodes down the Tech Tree",
  },
  {
    name: "tier",
    colour: "#ffe066",
    match: `tiers?(?: (?:${tiers.map((tier) => tier.id).join("|")}))?`,
    description: (text) => {
      const tier = tiers.find((tier) => text.toLowerCase().includes(tier.id));
      return tier ? tier_line(tier) : tiers.map(tier_line).join("\n");
    },
  },
  {
    name: "tech-tree",
    colour: "#7cc4f0",
    match: "tech trees?",
    description: "Your tree of fancy prizes!",
  },
  {
    name: "token",
    colour: "#c3a3e6",
    match: "tokens?",
    description: "Currency for actually redeeming unlocked nodes to your door",
  },
  {
    name: "prize",
    colour: "#f0788a",
    match: "prizes?",
    description: "Real stuff down your Tech Tree, sent to your door with a Token",
  },
  {
    name: "snapshot",
    colour: "#e8c58c",
    match: "snapshots?",
    description: "A little snippet of what you're working on, caption + screenshot",
  },
  {
    name: "hackatime",
    colour: "#d6ab70",
    match: "hackatime",
    description:
      "Tool for tracking your time, check it out [here](https://hackatime.hackclub.com/)!",
  },
];

type Hit = { range: Range; word: Word };

const pattern = new RegExp(
  `\\b(?:${words.map((word) => `(${word.match})`).join("|")})\\b`,
  "gi",
);
const skip = "a, script, style, noscript, textarea, [data-plain]";
const controls =
  "button, input, select, label, summary, [role=button], [role=link], [contenteditable]";
const key = (name: string) => `word-${name}`;

let hits = new Map<Node, Hit[]>();

function scan(tip: HTMLElement) {
  const ranges = words.map(() => [] as Range[]);
  const found = new Map<Node, Hit[]>();
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
      const word = words[match.slice(1).findIndex(Boolean)];
      ranges[words.indexOf(word)].push(range);
      if (!tip.contains(node)) {
        found.set(node, [...(found.get(node) ?? []), { range, word }]);
      }
    }
  }
  hits = found;
  words.forEach((word, i) => CSS.highlights.set(key(word.name), new Highlight(...ranges[i])));
}

function hit_at(x: number, y: number) {
  const element = document.elementFromPoint(x, y);
  if (!element) return null;
  for (const child of element.childNodes) {
    for (const hit of hits.get(child) ?? []) {
      for (const rect of hit.range.getClientRects()) {
        if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
          return { hit, rect };
        }
      }
    }
  }
  return null;
}

function fill(tip: HTMLElement, text: string) {
  tip.replaceChildren(
    ...text.split("\n").map((line) => {
      const p = document.createElement("p");
      let last = 0;
      for (const link of line.matchAll(/\[([^\]]+)\]\(([^)]+)\)/g)) {
        const a = document.createElement("a");
        a.href = link[2];
        a.target = "_blank";
        a.rel = "noopener";
        a.textContent = link[1];
        p.append(line.slice(last, link.index), a);
        last = link.index + link[0].length;
      }
      p.append(line.slice(last));
      return p;
    }),
  );
}

export function colour_words() {
  if (!("highlights" in CSS)) return () => {};
  const style = document.createElement("style");
  style.textContent = `${words
    .map((word) => `::highlight(${key(word.name)}) { color: ${word.colour}; }`)
    .join("\n")}
.word-tip {
  position: fixed;
  z-index: 2000;
  box-sizing: border-box;
  max-width: min(20rem, calc(100vw - 16px));
  padding: 0.5rem 0.75rem;
  background: #3d3029;
  color: #ead7c2;
  border: 2px solid #5f4838;
  border-radius: 7px;
  font-size: 0.9rem;
  font-weight: 400;
  line-height: 1.35;
  text-align: left;
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition: opacity 0.15s, visibility 0s 0.15s;
}
.word-tip.open {
  opacity: 1;
  visibility: visible;
  pointer-events: auto;
  transition: opacity 0.15s;
}
.word-tip p { margin: 0; }
.word-tip p + p { margin-top: 0.35rem; }`;
  document.head.append(style);

  const tip = document.createElement("div");
  tip.className = "word-tip";
  tip.setAttribute("role", "tooltip");
  tip.setAttribute("data-above-dialogs", "");
  document.body.append(tip);

  let shown: Range | null = null;
  let pending: Range | null = null;
  let hide_timer = 0;
  let show_timer = 0;
  let pointer = "mouse";

  const same = (a: Range | null, b: Range) =>
    a?.startContainer === b.startContainer && a.startOffset === b.startOffset;

  function hide() {
    clearTimeout(hide_timer);
    clearTimeout(show_timer);
    pending = null;
    tip.classList.remove("open");
    shown = null;
  }

  function open({ hit, rect }: { hit: Hit; rect: DOMRect }) {
    clearTimeout(hide_timer);
    clearTimeout(show_timer);
    pending = null;
    if (same(shown, hit.range)) return;
    shown = hit.range;
    const { description } = hit.word;
    fill(tip, typeof description === "function" ? description(hit.range.toString()) : description);
    tip.style.left = "0px";
    tip.style.top = "0px";
    const box = tip.getBoundingClientRect();
    const margin = 8;
    const left = Math.min(
      Math.max(margin, rect.left + rect.width / 2 - box.width / 2),
      innerWidth - box.width - margin,
    );
    const above = rect.top - box.height - 6;
    tip.style.left = `${left}px`;
    tip.style.top = `${above >= margin ? above : rect.bottom + 6}px`;
    tip.classList.add("open");
  }

  const on_pointerdown = (e: PointerEvent) => {
    pointer = e.pointerType;
    if (!tip.contains(e.target as Node)) hide();
  };

  const on_pointermove = (e: PointerEvent) => {
    if (e.pointerType !== "mouse") return;
    if (e.buttons) return hide();
    if (tip.contains(e.target as Node)) return clearTimeout(hide_timer);
    const { clientX: x, clientY: y } = e;
    const found = hit_at(x, y);
    if (found && shown) open(found);
    else if (found) {
      clearTimeout(hide_timer);
      if (same(pending, found.hit.range)) return;
      clearTimeout(show_timer);
      pending = found.hit.range;
      show_timer = window.setTimeout(() => {
        const still = hit_at(x, y);
        if (still && same(pending, still.hit.range)) open(still);
      }, 300);
    } else {
      clearTimeout(show_timer);
      pending = null;
      if (shown) {
        clearTimeout(hide_timer);
        hide_timer = window.setTimeout(hide, 200);
      }
    }
  };

  const on_click = (e: MouseEvent) => {
    if (pointer === "mouse" || tip.contains(e.target as Node)) return;
    const found = (e.target as Element).closest?.(controls) ? null : hit_at(e.clientX, e.clientY);
    if (found) open(found);
    else hide();
  };

  const on_keydown = (e: KeyboardEvent) => {
    if (e.key === "Escape") hide();
  };

  let frame = 0;
  const observer = new MutationObserver(() => {
    frame ||= requestAnimationFrame(() => {
      frame = 0;
      scan(tip);
      if (shown && !shown.startContainer.isConnected) hide();
    });
  });
  observer.observe(document.body, { childList: true, characterData: true, subtree: true });
  scan(tip);

  addEventListener("pointerdown", on_pointerdown, true);
  addEventListener("pointermove", on_pointermove);
  addEventListener("click", on_click);
  addEventListener("keydown", on_keydown);
  addEventListener("scroll", hide, true);
  addEventListener("wheel", hide, { passive: true });
  addEventListener("resize", hide);

  return () => {
    observer.disconnect();
    cancelAnimationFrame(frame);
    clearTimeout(hide_timer);
    clearTimeout(show_timer);
    removeEventListener("pointerdown", on_pointerdown, true);
    removeEventListener("pointermove", on_pointermove);
    removeEventListener("click", on_click);
    removeEventListener("keydown", on_keydown);
    removeEventListener("scroll", hide, true);
    removeEventListener("wheel", hide);
    removeEventListener("resize", hide);
    tip.remove();
    style.remove();
    for (const word of words) CSS.highlights.delete(key(word.name));
  };
}
