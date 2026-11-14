// A string child is one text node in the browser, laid out as React given
// the same string lays it out (#1807): its text nodes counted and its width
// measured in Chromium, against React rendering the same children natively,
// for Codex's four cases, a space at a text's start, its end, both, and a
// formula long enough to wrap. React's `renderToString` marks where one text
// node ends and the next starts, `<!-- -->`, so the page's text nodes are
// React's.

import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { createElement } from "react";
import { renderToString } from "react-dom/server";
import { buildReact, compiler, fixture, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const program = `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub fn Leading() -> JSX::Element {
    jsx! { <p><b>{"value"}</b>{" follows"}</p> }
}

pub fn Trailing() -> JSX::Element {
    jsx! { <p>{"precedes "}<b>{"value"}</b></p> }
}

pub fn Both() -> JSX::Element {
    jsx! { <p><b>{"a"}</b>{" between "}<b>{"b"}</b></p> }
}

pub fn Formula() -> JSX::Element {
    jsx! {
        <code><span>{"return"}</span>{" 0.55 + 0.45 * "}<span>{"cos(phase * 6.283185307179586 + offset)"}</span>{" * amplitude + baseline "}<span>{"end"}</span></code>
    }
}
`;

test("a string child is one text node, as wide as React's own, in Chromium", async () => {
  const dir = fixture("jsx-text-browser");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const made = await import(join(dir, "lib.jsx"));
  const h = createElement;
  const native: Record<string, unknown> = {
    Leading: h("p", null, h("b", null, "value"), " follows"),
    Trailing: h("p", null, "precedes ", h("b", null, "value")),
    Both: h("p", null, h("b", null, "a"), " between ", h("b", null, "b")),
    Formula: h("code", null, h("span", null, "return"), " 0.55 + 0.45 * ", h("span", null, "cos(phase * 6.283185307179586 + offset)"),
      " * amplitude + baseline ", h("span", null, "end")),
  };
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    // Each in a box of its own, its text nodes and its width.
    const measured = async (html: string) => {
      await page.setContent(`<div id="box" style="display:inline-block;font:16px serif">${html}</div>`);
      return page.evaluate(() => {
        const box = document.getElementById("box")!;
        const walker = document.createTreeWalker(box, NodeFilter.SHOW_TEXT);
        let texts = 0;
        while (walker.nextNode()) texts++;
        return { texts, width: box.getBoundingClientRect().width };
      });
    };
    for (const [name, element] of Object.entries(native)) {
      const ours = await measured(renderToString(h(made[name])));
      const theirs = await measured(renderToString(element as never));
      expect([name, ours]).toEqual([name, theirs]);
    }
  } finally {
    await browser.close();
  }
}, underLoad(120_000));
