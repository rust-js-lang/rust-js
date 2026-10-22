// The guide's Markdown as HTML: what the guide's pages use, and no more.
// Headings, with GitHub's anchor ids, so a page's own `#links` still land;
// fenced code, highlighted by the caller; tables; lists, whose items may hold
// paragraphs and fences; and paragraphs of inline code, links, bold and
// italics. Anything else is plain text, escaped.

export interface Heading {
  level: number;
  /** The heading as plain text, its code spans' text kept. */
  text: string;
  /** The heading as HTML, its code spans kept. */
  html: string;
  id: string;
}

export interface Rendered {
  title: string;
  html: string;
  headings: Heading[];
}

export interface Options {
  /** What a link's `href` becomes, a page's `.md` its `.html`. */
  link: (href: string) => string;
  /** A fence's code, highlighted, as HTML. */
  highlight: (code: string, lang: string) => string;
}

export function escapeHtml(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

const FENCE = /^(`{3,})\s*([\w-]*)\s*$/;
const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const LIST_ITEM = /^(\s*)([-*]|\d+\.)\s+/;
const TABLE_RULE = /^\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;

export function render(markdown: string, options: Options): Rendered {
  const state = { options, headings: [] as Heading[], ids: new Map<string, number>() };
  const html = blocks(markdown.replace(/\r\n/g, "\n").split("\n"), state, false);
  const title = state.headings.find((h) => h.level === 1)?.text ?? "";
  return { title, html, headings: state.headings };
}

interface State {
  options: Options;
  headings: Heading[];
  ids: Map<string, number>;
}

const blank = (line: string) => line.trim() === "";
const startsBlock = (line: string, next: string | undefined) =>
  FENCE.test(line) || HEADING.test(line) || LIST_ITEM.test(line) || (line.startsWith("|") && next !== undefined && TABLE_RULE.test(next));

/** `lines` as blocks; `tight`, a tight list item's, keeps its paragraphs bare. */
function blocks(lines: string[], state: State, tight: boolean): string {
  const out: string[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (blank(line)) {
      i++;
      continue;
    }
    const fence = FENCE.exec(line);
    if (fence) {
      const close = new RegExp(`^\`{${fence[1].length},}\\s*$`);
      const code: string[] = [];
      i++;
      while (i < lines.length && !close.test(lines[i])) code.push(lines[i++]);
      i++;
      out.push(codeBlock(code.join("\n"), fence[2], state));
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      out.push(headingHtml(heading[1].length, heading[2], state));
      i++;
      continue;
    }
    if (line.startsWith("|") && i + 1 < lines.length && TABLE_RULE.test(lines[i + 1])) {
      const rows: string[] = [];
      while (i < lines.length && lines[i].startsWith("|")) rows.push(lines[i++]);
      out.push(table(rows, state));
      continue;
    }
    if (LIST_ITEM.test(line)) {
      const [html, next] = list(lines, i, state);
      out.push(html);
      i = next;
      continue;
    }
    const paragraph: string[] = [];
    while (i < lines.length && !blank(lines[i]) && (paragraph.length === 0 || !startsBlock(lines[i], lines[i + 1]))) {
      paragraph.push(lines[i++].trim());
    }
    const text = inline(paragraph.join("\n"), state);
    out.push(tight ? text : `<p>${text}</p>`);
  }
  return out.join("\n");
}

function codeBlock(code: string, lang: string, state: State): string {
  const label = lang ? `<span class="code-lang">${escapeHtml(lang)}</span>` : "";
  const body = lang ? state.options.highlight(code, lang) : escapeHtml(code);
  return `<div class="code" data-lang="${escapeHtml(lang || "text")}">${label}<pre><code>${body}</code></pre></div>`;
}

/** GitHub's anchor for a heading: its text lowercased, of letters, digits,
 * spaces, `-` and `_`, each space a `-`; a repeat numbered `-1`, `-2`. */
export function slug(text: string, ids: Map<string, number>): string {
  const base = text
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    .replace(/ /g, "-");
  const seen = ids.get(base) ?? 0;
  ids.set(base, seen + 1);
  return seen === 0 ? base : `${base}-${seen}`;
}

function headingHtml(level: number, source: string, state: State): string {
  const html = inline(source, state);
  const text = plain(source);
  const id = slug(text, state.ids);
  state.headings.push({ level, text, html: inlineWithoutLinks(source, state), id });
  if (level === 1) return `<h1 id="${id}">${html}</h1>`;
  return `<h${level} id="${id}"><a class="anchor" href="#${id}" aria-label="Link to this section">#</a>${html}</h${level}>`;
}

/** A heading's text as GitHub reads it: its code's text, its links' text. */
function plain(source: string): string {
  return source
    .replace(/`+([^`]*?)`+/g, "$1")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[*_]{1,2}([^*_]+)[*_]{1,2}/g, "$1")
    .trim();
}

function inlineWithoutLinks(source: string, state: State): string {
  return inline(source.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1"), state);
}

function cells(row: string): string[] {
  const inner = row.trim().replace(/^\|/, "").replace(/\|$/, "");
  // A `|` in a code span is the span's, not a cell's edge.
  const parts: string[] = [];
  let current = "";
  let ticks = 0;
  for (let k = 0; k < inner.length; k++) {
    const c = inner[k];
    if (c === "`") ticks ^= 1;
    if (c === "\\" && inner[k + 1] === "|") {
      current += "|";
      k++;
      continue;
    }
    if (c === "|" && ticks === 0) {
      parts.push(current.trim());
      current = "";
      continue;
    }
    current += c;
  }
  parts.push(current.trim());
  return parts;
}

function table(rows: string[], state: State): string {
  const aligns = cells(rows[1]).map((c) => (c.startsWith(":") && c.endsWith(":") ? "center" : c.endsWith(":") ? "right" : ""));
  const cell = (tag: string, text: string, k: number) =>
    `<${tag}${aligns[k] ? ` style="text-align:${aligns[k]}"` : ""}>${inline(text, state)}</${tag}>`;
  const head = cells(rows[0]).map((c, k) => cell("th", c, k)).join("");
  const body = rows
    .slice(2)
    .map((r) => `<tr>${cells(r).map((c, k) => cell("td", c, k)).join("")}</tr>`)
    .join("\n");
  return `<div class="table"><table><thead><tr>${head}</tr></thead><tbody>\n${body}\n</tbody></table></div>`;
}

/** The list starting at `lines[start]`, and the line after it. Each item is
 * its marker's line and what's indented past its marker after it. */
function list(lines: string[], start: number, state: State): [string, number] {
  const first = LIST_ITEM.exec(lines[start])!;
  const indent = first[1].length;
  const ordered = /\d/.test(first[2]);
  const items: string[][] = [];
  let i = start;
  let loose = false;
  while (i < lines.length) {
    const marker = LIST_ITEM.exec(lines[i]);
    if (!marker || marker[1].length !== indent || /\d/.test(marker[2]) !== ordered) break;
    const width = marker[0].length;
    const item = [lines[i].slice(width)];
    i++;
    while (i < lines.length) {
      const line = lines[i];
      if (blank(line)) {
        // A blank line, then more of this item, or the next item: loose.
        const after = lines.slice(i + 1).find((l) => !blank(l));
        if (after === undefined) break;
        const continues = after.length - after.trimStart().length >= width;
        const sibling = LIST_ITEM.exec(after);
        if (!continues && !(sibling && sibling[1].length === indent)) break;
        loose = true;
        item.push("");
        i++;
        continue;
      }
      const indented = line.length - line.trimStart().length >= width;
      if (indented) {
        item.push(line.slice(width));
        i++;
        continue;
      }
      if (LIST_ITEM.test(line) || item[item.length - 1] === "") break;
      // A lazy continuation of the item's paragraph.
      item.push(line.trim());
      i++;
    }
    while (item.length > 0 && item[item.length - 1] === "") item.pop();
    items.push(item);
  }
  const tag = ordered ? "ol" : "ul";
  const start0 = ordered ? Number(first[2].slice(0, -1)) : 1;
  const open = ordered && start0 !== 1 ? `<ol start="${start0}">` : `<${tag}>`;
  const body = items.map((item) => `<li>${blocks(item, state, !loose)}</li>`).join("\n");
  return [`${open}\n${body}\n</${tag}>`, i];
}

/** A paragraph's text: code spans, links, bold, italics, the rest escaped. */
export function inline(text: string, state: { options: Options }): string {
  let out = "";
  let k = 0;
  while (k < text.length) {
    const c = text[k];
    if (c === "\\" && k + 1 < text.length && /[\\`*_[\]()#|!-]/.test(text[k + 1])) {
      out += escapeHtml(text[k + 1]);
      k += 2;
      continue;
    }
    if (c === "`") {
      const run = /^`+/.exec(text.slice(k))![0];
      const end = findTicks(text, k + run.length, run.length);
      if (end >= 0) {
        let code = text.slice(k + run.length, end).replace(/\n/g, " ");
        if (code.startsWith(" ") && code.endsWith(" ") && code.trim() !== "") code = code.slice(1, -1);
        out += `<code>${escapeHtml(code)}</code>`;
        k = end + run.length;
        continue;
      }
      out += escapeHtml(run);
      k += run.length;
      continue;
    }
    if (c === "[") {
      const link = parseLink(text, k);
      if (link) {
        const href = state.options.link(link.href);
        const external = /^https?:/.test(href);
        out += `<a href="${escapeHtml(href)}"${external ? ' rel="noopener"' : ""}>${inline(link.text, state)}</a>`;
        k = link.end;
        continue;
      }
    }
    if (c === "*" && text[k + 1] === "*") {
      const end = text.indexOf("**", k + 2);
      if (end > k + 2) {
        out += `<strong>${inline(text.slice(k + 2, end), state)}</strong>`;
        k = end + 2;
        continue;
      }
    }
    if (c === "*" && text[k + 1] !== " " && text[k + 1] !== "*") {
      const end = findEmphasisEnd(text, k + 1);
      if (end > 0) {
        out += `<em>${inline(text.slice(k + 1, end), state)}</em>`;
        k = end + 1;
        continue;
      }
    }
    if (c === "\n") {
      out += "\n";
      k++;
      continue;
    }
    out += escapeHtml(c);
    k++;
  }
  return out;
}

/** The index of the run of exactly `n` backticks from `from`, or -1. */
function findTicks(text: string, from: number, n: number): number {
  let k = from;
  while (k < text.length) {
    const at = text.indexOf("`", k);
    if (at < 0) return -1;
    const run = /^`+/.exec(text.slice(at))![0].length;
    if (run === n) return at;
    k = at + run;
  }
  return -1;
}

function findEmphasisEnd(text: string, from: number): number {
  for (let k = from; k < text.length; k++) {
    if (text[k] === "`") {
      const run = /^`+/.exec(text.slice(k))![0].length;
      const end = findTicks(text, k + run, run);
      if (end < 0) return -1;
      k = end + run - 1;
      continue;
    }
    if (text[k] === "*" && text[k - 1] !== " " && text[k + 1] !== "*") return k;
  }
  return -1;
}

/** `[text](href)` at `start`: its text may hold code spans and brackets. */
function parseLink(text: string, start: number): { text: string; href: string; end: number } | undefined {
  let depth = 0;
  let k = start;
  for (; k < text.length; k++) {
    const c = text[k];
    if (c === "`") {
      const run = /^`+/.exec(text.slice(k))![0].length;
      const end = findTicks(text, k + run, run);
      if (end < 0) return undefined;
      k = end + run - 1;
      continue;
    }
    if (c === "[") depth++;
    if (c === "]" && --depth === 0) break;
  }
  if (k >= text.length || text[k + 1] !== "(") return undefined;
  let parens = 0;
  let j = k + 1;
  for (; j < text.length; j++) {
    if (text[j] === "(") parens++;
    if (text[j] === ")" && --parens === 0) break;
  }
  if (j >= text.length) return undefined;
  return { text: text.slice(start + 1, k), href: text.slice(k + 2, j).trim(), end: j + 1 };
}
