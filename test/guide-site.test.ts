// The guide's site (ADR 0600): each page of docs/guide an HTML page, every
// link of the guide leading somewhere, and what each page is made of.
import { expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { build, outputPath } from "../guide-site/build";
import { highlight } from "../guide-site/highlight";
import { render } from "../guide-site/markdown";
import { fixture, root } from "./support";

const guide = join(root, "docs", "guide");
const pages = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return pages(path);
    return name.endsWith(".md") ? [relative(guide, path)] : [];
  });

test("each guide page is a page of the site, and every link leads somewhere", () => {
  const out = fixture("guide-site");
  const built = build(out);
  expect(built.missing).toEqual([]);
  const sources = pages(guide);
  expect(built.pages).toBe(sources.length);
  for (const source of sources) expect(existsSync(join(out, outputPath(source)))).toBe(true);
  for (const asset of ["style.css", "site.js", "logo.png", "search.json", "404.html", ".nojekyll"]) {
    expect(existsSync(join(out, asset))).toBe(true);
  }

  // The nav is the guide's README, the page itself marked; a page's link to
  // another is to its `.html`, and out of the guide to it on GitHub.
  const basics = readFileSync(join(out, "for-js-developers", "basics.html"), "utf8");
  expect(basics).toContain('<a href="basics.html" aria-current="page">Basics: values, functions, control flow</a>');
  expect(basics).toContain('<a href="collections.html">Collections</a>');
  expect(basics).toContain('href="https://github.com/rust-js-lang/rust-js/blob/main/docs/decisions/0573-every-rust-name-is-camel-case-in-js.md"');
  expect(basics).toContain('<link rel="stylesheet" href="../style.css">');
  // Its headings keep GitHub's anchors, which its own contents link to.
  expect(basics).toContain('<a href="#null-and-undefined-option">');
  expect(basics).toContain('<h2 id="null-and-undefined-option">');
  // Its Rust and its JS, each highlighted.
  expect(basics).toContain('<div class="code" data-lang="rust"><span class="code-lang">rust</span><pre><code><span class="k">pub</span> <span class="k">fn</span>');

  const index = readFileSync(join(out, "index.html"), "utf8");
  expect(index).toContain("<title>rust-js — Rust in. Readable JavaScript out.</title>");
  const search = JSON.parse(readFileSync(join(out, "search.json"), "utf8")) as { url: string; heading: string }[];
  expect(search.some((e) => e.url === "for-js-developers/basics.html#numbers" && e.heading === "Numbers")).toBe(true);
});

test("the guide's Markdown: a table, a list item's fence, code spans of backticks", () => {
  const options = { link: (href: string) => href.replace(/\.md$/, ".html"), highlight };
  const { html, headings } = render(
    [
      "# Title",
      "",
      "## `null` and `undefined`: `Option`",
      "",
      "| Rust | JS |",
      "|---|---|",
      "| `a \\| b` | [types](types.md) |",
      "",
      "- **One** item, `x`:",
      "",
      "  ```text",
      "  error: here",
      "  ```",
      "- Two, with `` `ticks` ``",
    ].join("\n"),
    options,
  );
  expect(headings.map((h) => h.id)).toEqual(["title", "null-and-undefined-option"]);
  expect(html).toContain("<td><code>a | b</code></td><td><a href=\"types.html\">types</a></td>");
  expect(html).toContain("<li><p><strong>One</strong> item, <code>x</code>:</p>\n<div class=\"code\" data-lang=\"text\">");
  expect(html).toContain("<pre><code>error: here</code></pre>");
  expect(html).toContain("<p>Two, with <code>`ticks`</code></p>");
});

test("a fence's Rust and JS are highlighted, and what's unknown is escaped", () => {
  expect(highlight('let s: &\'static str = "a<b"; // done', "rust")).toBe(
    '<span class="k">let</span> s: &amp;<span class="l">\'static</span> <span class="t">str</span> = <span class="s">&quot;a&lt;b&quot;</span>; <span class="c">// done</span>',
  );
  expect(highlight("const x = `${a}`; f(<App />)", "jsx")).toBe(
    '<span class="k">const</span> x = <span class="s">`${a}`</span>; <span class="f">f</span>(&lt;<span class="g">App</span> /&gt;)',
  );
  expect(highlight("<b>", "text")).toBe("&lt;b&gt;");
});
