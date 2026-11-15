// How a component's props are read (#1908): its own fields as plain data,
// as React builds them, and what they hold as the caller's objects, a
// getter of which may answer each read anew. Each test is a program of its
// own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

function compile(source: string) {
  const dir = fixture("jsx-props-read");
  writeFileSync(join(dir, "lib.rs"), source);
  const args = [compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--manifest", join(dir, "manifest.json"),
    "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  return { dir, args };
}

// A component's own props are plain data, as React builds them, a fresh
// object of its props' values: `p.title.map(..)` reads `p.title` where it's
// tested and where it's used, as upstream does. A struct they hold is the
// caller's, a getter of which may answer anew, so what Rust reads of it
// once is read once, `const option = p.style.margin` (#1908). A component
// JS calls directly, not through React, with a getter for a prop sees that
// prop read twice: ADR 0262's rare and harmless exception, the user's
// decision (2026-11-15).
test("a component's own props are plain data, what they hold is read once", async () => {
  const { dir, args } = compile(`#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct Style { pub margin: Option<f64> }
pub struct P { pub title: Option<&'static str>, pub style: &'static Style }
pub fn Badge(p: P) -> JSX::Element {
    let margin = p.style.margin.map_or(0.0, |m| m * 2.0);
    jsx! { <svg>{p.title.map(|t| jsx! { <title>{t}</title> })}{margin}</svg> }
}
`);
  run(args);
  const jsx = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(jsx).toContain("{p.title != null && <title>{p.title}</title>}");
  expect(jsx).toContain("const m = p.style.margin;");
  const { Badge } = await import(join(dir, "lib.jsx"));
  // Called directly with getters, as React never calls it: the prop read
  // twice, the field of what it holds once.
  const reads: string[] = [];
  const style = { get margin() { reads.push("margin"); return 2; } };
  const props = { get title() { reads.push("title"); return "t"; }, style };
  expect(renderToStaticMarkup(Badge(props))).toBe("<svg><title>t</title>4</svg>");
  expect(reads).toEqual(["margin", "title", "title"]);
});
