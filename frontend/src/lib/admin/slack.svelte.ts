import channel_ids from "$lib/assets/slack_channels.json";

type Emoji = Record<string, string | string[]>;

const channel_names = Object.fromEntries(
  Object.entries(channel_ids as Record<string, string>).map(([name, id]) => [id, name]),
);

let emoji = $state.raw<Emoji>({});
import("$lib/assets/slack_emoji.json").then((module) => {
  emoji = module.default as Emoji;
});

const control = /^<(?:https?:\/\/|mailto:|[@#!])[^<>\n]*>/;

export function escape(text: string) {
  let out = "";
  let line_start = true;
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    const sequence = c === "<" ? control.exec(text.slice(i)) : null;
    if (sequence) {
      out += sequence[0];
      i += sequence[0].length;
      line_start = false;
      continue;
    }
    if (c === "<") out += "&lt;";
    else if (c === ">") out += line_start ? ">" : "&gt;";
    else if (c === "&")
      out += /^&(?:amp|lt|gt);/.test(text.slice(i)) ? "&" : "&amp;";
    else out += c;
    line_start = c === "\n" || (line_start && c === ">");
    i++;
  }
  return out;
}

const code = /^(?:```[\s\S]*?```|`[^`\n]*`)/;
const channel_name = /^#([a-z0-9_-]+)/i;

function channels(text: string) {
  let out = "";
  let prev = "";
  let i = 0;
  while (i < text.length) {
    const rest = text.slice(i);
    const skip =
      rest[0] === "<" ? control.exec(rest) : rest[0] === "`" ? code.exec(rest) : null;
    if (skip) {
      out += skip[0];
      prev = skip[0].at(-1)!;
      i += skip[0].length;
      continue;
    }
    const name = rest[0] === "#" && !/[\p{L}\p{N}_\-&/#:]/u.test(prev) ? channel_name.exec(rest) : null;
    const id = name && (channel_ids as Record<string, string>)[name[1].toLowerCase()];
    if (name && id) {
      out += `<#${id}>`;
      prev = ">";
      i += name[0].length;
      continue;
    }
    out += rest[0];
    prev = rest[0];
    i++;
  }
  return out;
}

export const mrkdwn = (text: string) => escape(channels(text));

export const decode = (text: string) =>
  text.replace(/&(lt|gt|amp);/g, (_, entity) =>
    entity === "lt" ? "<" : entity === "gt" ? ">" : "&",
  );

const html = (text: string) =>
  text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

const text_html = (mrkdwn: string) => html(decode(mrkdwn));

function link(url: string, label: string) {
  const href = decode(url);
  if (!/^(https?:\/\/|mailto:)/i.test(href)) return text_html(label);
  return `<a href="${html(href)}" target="_blank" rel="noopener noreferrer">${text_html(label)}</a>`;
}

const mention = (label: string) =>
  `<span class="mention">${text_html(label)}</span>`;

function sequence(inner: string) {
  const [target, label] = [
    inner.split("|")[0],
    inner.includes("|") ? inner.slice(inner.indexOf("|") + 1) : "",
  ];
  if (target.startsWith("@")) return mention(label ? `@${label}` : target);
  if (target.startsWith("#")) {
    const id = target.slice(1);
    const name = `#${label || channel_names[id] || id}`;
    if (!/^[A-Z0-9]+$/.test(id)) return mention(name);
    return `<a class="mention channel" href="https://hackclub.enterprise.slack.com/archives/${id}" target="_blank" rel="noopener noreferrer">${text_html(name)}</a>`;
  }
  if (target.startsWith("!")) {
    const command = target.slice(1);
    if (["here", "channel", "everyone"].includes(command))
      return mention(`@${command}`);
    if (command.startsWith("subteam^")) return mention(label || "@group");
    return text_html(label || `<${inner}>`);
  }
  return link(target, label || target.replace(/^mailto:/, ""));
}

function emoji_html(name: string, tone?: string): string | null {
  const found = emoji[name];
  if (!found) return null;
  if (typeof found === "string") return found + (tone ? emoji_html(tone) ?? "" : "");
  const index = tone ? Number(tone.slice(-1)) - 1 : 0;
  return found[index] ?? found[0];
}

const atoms =
  /`([^`\n]+)`|<([^<>\n]+)>|:([a-z0-9_+'-]+):(?::(skin-tone-[2-6]):)?|(https?:\/\/[^\s<>]+[^\s<>.,;:!?)\]'"])/g;
const edge = `\\s()\\[\\]{}"'.,;:!?<>\\-*_~\\u0000`;
const emphasis = (marker: string) =>
  new RegExp(
    `(^|[${edge}])\\${marker}(?=[^\\s${marker}])([^${marker}\\n]*?[^\\s${marker}])\\${marker}(?=$|[${edge}])`,
    "g",
  );
const styles: [RegExp, string][] = [
  [emphasis("*"), "strong"],
  [emphasis("_"), "em"],
  [emphasis("~"), "del"],
];

function inline(mrkdwn: string) {
  const saved: string[] = [];
  const keep = (value: string) => `\u0000${saved.push(value) - 1}\u0000`;
  let text = mrkdwn.replace(
    atoms,
    (whole, code, inner, name, tone, url) => {
      if (code !== undefined) return keep(`<code>${text_html(code)}</code>`);
      if (inner !== undefined) return keep(sequence(inner));
      if (name !== undefined) {
        const found = emoji_html(name, tone);
        return found ? keep(found) : whole;
      }
      return keep(link(url, url));
    },
  );
  text = text_html(text);
  for (const [pattern, tag] of styles)
    text = text.replace(
      pattern,
      (_, before, body) => `${before}<${tag}>${body}</${tag}>`,
    );
  return text.replace(/\u0000(\d+)\u0000/g, (_, i) => saved[Number(i)]);
}

function lines(mrkdwn: string) {
  let out = "";
  let quote: string[] = [];
  const flush = () => {
    if (quote.length)
      out += `<blockquote>${quote.map(inline).join("<br>")}</blockquote>`;
    quote = [];
  };
  const all = mrkdwn.split("\n");
  for (let i = 0; i < all.length; i++) {
    const line = all[i];
    if (line.startsWith(">>>")) {
      quote.push(line.slice(3).replace(/^ /, ""), ...all.slice(i + 1));
      break;
    }
    if (line.startsWith(">")) {
      quote.push(line.slice(1).replace(/^ /, ""));
      continue;
    }
    flush();
    out += `<p>${inline(line) || "<br>"}</p>`;
  }
  flush();
  return out;
}

export function render(text: string) {
  return mrkdwn(text.trim())
    .split(/```/)
    .map((part, i, parts) =>
      i % 2 && i < parts.length - 1
        ? `<pre>${text_html(part.replace(/^\n/, ""))}</pre>`
        : lines(i % 2 ? "```" + part : part),
    )
    .join("");
}
