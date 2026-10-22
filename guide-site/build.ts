// The guide as a static site (ADR 0600): each page of docs/guide an HTML
// page, `README.md` the index, laid out by one template, with the site's CSS
// and JS. The nav is the guide's README, its one index: each `##` a section,
// each link under it a page. A link to a guide page is to its `.html`; to
// anything else of the repository, a decision or the architecture, to it on
// GitHub. A link that leads nowhere fails the build.
//
//   bun guide-site/build.ts [out]     # out: target/guide-site

import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, posix, relative, resolve } from "node:path";
import { highlight } from "./highlight";
import { escapeHtml, inline, render, type Heading } from "./markdown";

export const REPOSITORY = "https://github.com/rust-js-lang/rust-js";
const root = resolve(import.meta.dir, "..");
const guide = join(root, "docs", "guide");

interface NavItem {
  title: string;
  /** The page's path in the guide, `tutorial/vite.md`, or a URL. */
  href: string;
  external: boolean;
}

interface NavSection {
  title: string;
  items: NavItem[];
}

interface Page {
  /** Its path in the guide, `interop/types.md`. */
  source: string;
  /** Its path in the site, `interop/types.html`. */
  output: string;
  title: string;
  html: string;
  headings: Heading[];
}

/** Every Markdown file of the guide, by its path in it. */
function guidePages(dir = guide): string[] {
  return readdirSync(dir)
    .flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return guidePages(path);
      return name.endsWith(".md") ? [relative(guide, path).split("\\").join("/")] : [];
    })
    .sort();
}

/** A page's path in the site: a `README.md` its folder's `index.html`. */
export function outputPath(source: string): string {
  return source.endsWith("README.md") ? source.replace(/README\.md$/, "index.html") : source.replace(/\.md$/, ".html");
}

/** Where `href`, written in the guide page `from`, goes in the site, as a
 * link from `from`'s page; `missing` gets what it can't find. */
function resolveLink(href: string, from: string, missing: (href: string) => void): string {
  if (/^[a-z]+:/.test(href) || href.startsWith("#")) return href;
  const [path, hash] = href.split("#");
  const target = posix.normalize(posix.join(posix.dirname(from), path));
  const anchor = hash ? `#${hash}` : "";
  if (!target.startsWith("..")) {
    if (!existsSync(join(guide, target))) missing(href);
    if (target.endsWith(".md")) return posix.relative(posix.dirname(outputPath(from)), outputPath(target)) + anchor || `#${hash}`;
    return href;
  }
  // Out of the guide, into the repository: on GitHub.
  const inRepo = posix.normalize(posix.join("docs/guide", posix.dirname(from), path));
  if (!existsSync(join(root, inRepo))) missing(href);
  const kind = existsSync(join(root, inRepo)) && statSync(join(root, inRepo)).isDirectory() ? "tree" : "blob";
  return `${REPOSITORY}/${kind}/main/${inRepo}${anchor}`;
}

/** The nav, of the guide's README: each `##` a section, each list item's
 * first link a page. Its last, "How these pages are written", isn't one. */
function nav(readme: string): NavSection[] {
  const sections: NavSection[] = [];
  for (const line of readme.split("\n")) {
    const heading = /^##\s+(.*)$/.exec(line);
    if (heading) {
      sections.push({ title: heading[1].trim(), items: [] });
      continue;
    }
    const item = /^- \[([^\]]+)\]\(([^)]+)\)/.exec(line);
    if (item && sections.length > 0) {
      const href = item[2];
      const external = !href.replace(/#.*/, "").endsWith(".md") || posix.normalize(href).startsWith("..");
      sections[sections.length - 1].items.push({ title: item[1], href, external });
    }
  }
  return sections.filter((s) => s.items.length > 0);
}

/** Each page in the nav's order, the index first, then what the nav
 * doesn't list, by path: what "previous" and "next" step through. */
function readingOrder(sections: NavSection[], pages: string[]): string[] {
  const listed = sections.flatMap((s) => s.items.filter((i) => !i.external).map((i) => posix.normalize(i.href.replace(/#.*/, ""))));
  const order = ["README.md", ...listed.filter((p, k) => listed.indexOf(p) === k)];
  return [...order, ...pages.filter((p) => !order.includes(p))].filter((p) => pages.includes(p));
}

const inlineOptions = (from: string, missing: (href: string) => void) => ({
  options: { link: (href: string) => resolveLink(href, from, missing), highlight },
});

function navHtml(sections: NavSection[], page: Page, extra: Page[]): string {
  const link = (target: string) => posix.relative(posix.dirname(page.output), outputPath(target)) || posix.basename(outputPath(target));
  const items = sections.map((section) => {
    const lis = section.items.map((item) => {
      if (item.external) {
        const href = resolveLink(item.href, "README.md", () => {});
        return `<li><a href="${escapeHtml(href)}" rel="noopener">${inline(item.title, inlineOptions("README.md", () => {}))}<span class="ext" aria-hidden="true">↗</span></a></li>`;
      }
      const target = posix.normalize(item.href.replace(/#.*/, ""));
      const current = target === page.source ? ' aria-current="page"' : "";
      return `<li><a href="${escapeHtml(link(target))}"${current}>${inline(item.title, inlineOptions("README.md", () => {}))}</a></li>`;
    });
    return `<section><h2>${escapeHtml(section.title)}</h2><ul>${lis.join("")}</ul></section>`;
  });
  if (extra.length > 0) {
    const lis = extra.map((p) => {
      const current = p.source === page.source ? ' aria-current="page"' : "";
      return `<li><a href="${escapeHtml(link(p.source))}"${current}>${escapeHtml(p.title)}</a></li>`;
    });
    items.push(`<section><h2>More</h2><ul>${lis.join("")}</ul></section>`);
  }
  return items.join("\n");
}

function tocHtml(headings: Heading[]): string {
  const entries = headings.filter((h) => h.level === 2 || h.level === 3);
  if (entries.length < 2) return "";
  const lis = entries.map((h) => `<li class="toc-${h.level}"><a href="#${h.id}">${h.html}</a></li>`).join("");
  return `<nav class="toc" aria-label="On this page"><h2>On this page</h2><ul>${lis}</ul></nav>`;
}

function template(page: Page, sections: NavSection[], extra: Page[], previous?: Page, next?: Page): string {
  const base = posix.relative(posix.dirname(page.output), ".") || ".";
  const asset = (name: string) => `${base}/${name}`;
  const pageLink = (p: Page) => posix.relative(posix.dirname(page.output), p.output) || posix.basename(p.output);
  const pager = [
    previous ? `<a class="prev" href="${escapeHtml(pageLink(previous))}"><span>Previous</span>${escapeHtml(previous.title)}</a>` : "<span></span>",
    next ? `<a class="next" href="${escapeHtml(pageLink(next))}"><span>Next</span>${escapeHtml(next.title)}</a>` : "<span></span>",
  ].join("");
  const title = page.source === "README.md" ? "rust-js — Rust in. Readable JavaScript out." : `${page.title} — rust-js`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="rust-js compiles Rust to readable JavaScript.">
<link rel="icon" href="${asset("logo.png")}" type="image/png">
<link rel="stylesheet" href="${asset("style.css")}">
<script>document.documentElement.dataset.theme = localStorage.getItem("theme") || (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");</script>
<script src="${asset("site.js")}" defer></script>
</head>
<body data-base="${escapeHtml(base)}">
<a class="skip" href="#content">Skip to content</a>
<header class="top">
  <button class="menu" aria-label="Menu" aria-expanded="false" aria-controls="sidebar"><span></span></button>
  <a class="brand" href="${escapeHtml(asset("index.html"))}"><img src="${asset("logo.png")}" alt="" width="32" height="23"><span>rust-js</span></a>
  <button class="search-open" aria-label="Search"><svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5"/><path d="m13 13 4.5 4.5"/></svg><span>Search the guide</span><kbd>/</kbd></button>
  <div class="actions">
    <a href="${REPOSITORY}" rel="noopener" aria-label="rust-js on GitHub" class="icon"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.38v-1.33c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.06-.49.06-.49.8.06 1.23.83 1.23.83.72 1.22 1.87.87 2.33.66.07-.52.28-.87.5-1.07-1.78-.2-3.65-.89-3.65-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 8 0Z"/></svg></a>
    <button class="theme icon" aria-label="Toggle dark mode"><svg viewBox="0 0 20 20" aria-hidden="true"><path class="sun" d="M10 14a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm0-12v2m0 12v2M2 10h2m12 0h2M4.3 4.3l1.4 1.4m8.6 8.6 1.4 1.4m0-11.4-1.4 1.4m-8.6 8.6-1.4 1.4"/><path class="moon" d="M16.5 12.5A7 7 0 0 1 7.5 3.5a7 7 0 1 0 9 9Z"/></svg></button>
  </div>
</header>
<div class="layout">
  <aside id="sidebar" class="sidebar" aria-label="Guide">
    <nav>${navHtml(sections, page, extra)}</nav>
  </aside>
  <main id="content" class="content">
    <article class="page">
${page.html}
    </article>
    <nav class="pager" aria-label="Previous and next">${pager}</nav>
    <footer class="foot">
      <a href="${REPOSITORY}/blob/main/docs/guide/${page.source}" rel="noopener">Edit this page on GitHub</a>
      <span>rust-js · Rust in. Readable JavaScript out.</span>
    </footer>
  </main>
  <aside class="aside">${tocHtml(page.headings)}</aside>
</div>
<div class="search" hidden>
  <div class="search-box" role="dialog" aria-modal="true" aria-label="Search the guide">
    <input type="search" placeholder="Search the guide" aria-label="Search" autocomplete="off" spellcheck="false">
    <ul class="results" role="listbox"></ul>
    <p class="search-hint"><kbd>↑</kbd><kbd>↓</kbd> to move, <kbd>Enter</kbd> to go, <kbd>Esc</kbd> to close</p>
  </div>
</div>
</body>
</html>
`;
}

/** Each section of each page, by its heading, for the site's search: its
 * page, heading, anchor and text, in plain words. */
function searchIndex(pages: Page[]): object[] {
  const entries: object[] = [];
  for (const page of pages) {
    const sections = page.html.split(/(?=<h[1-3] id=")/);
    for (const section of sections) {
      const id = /^<h[1-3] id="([^"]+)"/.exec(section)?.[1];
      const heading = page.headings.find((h) => h.id === id);
      const text = section
        .replace(/<span class="code-lang">[^<]*<\/span>/g, "")
        .replace(/<h[1-3][^>]*>[\s\S]*?<\/h[1-3]>/, "")
        .replace(/<\/(p|li|td|th|tr|pre|div|h\d)>/g, " ")
        .replace(/<[^>]+>/g, "")
        .replace(/&lt;/g, "<")
        .replace(/&gt;/g, ">")
        .replace(/&quot;/g, '"')
        .replace(/&amp;/g, "&")
        .replace(/\s+/g, " ")
        .trim();
      entries.push({
        page: page.title,
        heading: heading && heading.level > 1 ? heading.text : "",
        url: page.output + (heading && heading.level > 1 ? `#${heading.id}` : ""),
        text: text.slice(0, 600),
      });
    }
  }
  return entries;
}

export function build(out: string): { pages: number; missing: string[] } {
  const sources = guidePages();
  const missing: string[] = [];
  const pages: Page[] = sources.map((source) => {
    const markdown = readFileSync(join(guide, source), "utf8");
    const rendered = render(markdown, {
      link: (href) => resolveLink(href, source, (h) => missing.push(`${source}: ${h}`)),
      highlight,
    });
    return { source, output: outputPath(source), title: rendered.title || source, html: rendered.html, headings: rendered.headings };
  });
  const sections = nav(readFileSync(join(guide, "README.md"), "utf8"));
  const order = readingOrder(sections, sources).map((s) => pages.find((p) => p.source === s)!);
  const listed = new Set(sections.flatMap((s) => s.items.filter((i) => !i.external).map((i) => posix.normalize(i.href.replace(/#.*/, "")))));
  const extra = order.filter((p) => p.source !== "README.md" && !listed.has(p.source));
  rmSync(out, { recursive: true, force: true });
  for (const [k, page] of order.entries()) {
    const file = join(out, page.output);
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, template(page, sections, extra, order[k - 1], order[k + 1]));
  }
  const assets = join(import.meta.dir, "assets");
  for (const name of readdirSync(assets)) copyFileSync(join(assets, name), join(out, name));
  // The logo, the repository's one, `docs/logo`.
  copyFileSync(join(root, "docs", "logo", "rust-js.png"), join(out, "logo.png"));
  writeFileSync(join(out, "search.json"), JSON.stringify(searchIndex(order)));
  // GitHub Pages serves the site as it is, no Jekyll, and its 404 page.
  writeFileSync(join(out, ".nojekyll"), "");
  const notFound: Page = {
    source: "404.md",
    output: "404.html",
    title: "Page not found",
    html: `<h1 id="page-not-found">Page not found</h1>\n<p>That page isn't in the guide. Start from <a href="./index.html">the guide's contents</a>, or search it.</p>`,
    headings: [],
  };
  writeFileSync(join(out, "404.html"), template(notFound, sections, extra).replace(/(href|src)="\.\//g, '$1="/rust-js/'));
  return { pages: order.length, missing };
}

if (import.meta.main) {
  const out = resolve(process.argv[2] ?? join(root, "target", "guide-site"));
  const { pages, missing } = build(out);
  if (missing.length > 0) {
    console.error(`links that lead nowhere:\n${missing.map((m) => `  ${m}`).join("\n")}`);
    process.exit(1);
  }
  console.log(`wrote ${pages} pages to ${relative(process.cwd(), out) || "."}`);
}
