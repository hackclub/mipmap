import { Marked } from "marked";
import source from "./docs.md?raw";

const escape = (text: string) =>
  text.replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!,
  );

const marked = new Marked({
  renderer: {
    link({ href, title, tokens }) {
      if (!/^https?:\/\//.test(href)) return false;
      const label = this.parser.parseInline(tokens);
      const titled = title ? ` title="${escape(title)}"` : "";
      return `<a href="${escape(href)}"${titled} target="_blank" rel="noopener">${label}</a>`;
    },
  },
});

export const load = () => ({ html: marked.parse(source, { async: false }) });
