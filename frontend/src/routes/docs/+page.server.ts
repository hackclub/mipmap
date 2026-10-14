import { Marked } from "marked";
import { tiers } from "$lib/projects/tiers";
import source from "./docs.md?raw";

const escape = (text: string) =>
  text.replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!,
  );

const keywords = new RegExp(
  `\\b(${tiers.map((tier) => tier.name).join("|")}|Mips?|[Tt]okens?|[Pp]rizes?)\\b`,
  "g",
);

const kind = (word: string) =>
  word.startsWith("Tier ")
    ? "tier"
    : word.startsWith("Mip")
      ? "mip"
      : /^[Tt]oken/.test(word)
        ? "token"
        : "prize";

const marked = new Marked({
  renderer: {
    text(token) {
      if ("tokens" in token && token.tokens) return false;
      const html = "escaped" in token && token.escaped ? token.text : escape(token.text);
      return html.replace(keywords, (word) => `<span class="${kind(word)}">${word}</span>`);
    },
    link({ href, title, tokens }) {
      if (!/^https?:\/\//.test(href)) return false;
      const label = this.parser.parseInline(tokens);
      const titled = title ? ` title="${escape(title)}"` : "";
      return `<a href="${escape(href)}"${titled} target="_blank" rel="noopener">${label}</a>`;
    },
  },
});

export const load = () => ({ html: marked.parse(source, { async: false }) });
