// What a component renders is any node React renders (#1810): a `Vec` of
// elements, nested `Option`s and `Vec`s, text, a number, nothing, a
// fragment, or an alias of one, as @types/react's component returns a
// `ReactNode`. Which function is a component is JSX's, a capitalized one of
// one props argument or none; what it returns is rustc's to check, through
// react's component binding. Each against React rendering the same nodes.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { createElement, Fragment } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { buildReact, compiler, fixture, root, run, target, bunTest } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];

const program = `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct TokensProps<'a> {
    pub text: &'a str,
}

/// An array of elements, each a token.
pub fn Tokens(TokensProps { text }: TokensProps) -> Vec<JSX::Element> {
    text.split(' ').map(|word| jsx! { <b key={word}>{word}</b> }).collect()
}

/// Nested: some of a list, each maybe nothing.
pub fn Maybe(TokensProps { text }: TokensProps) -> Option<Vec<Option<JSX::Element>>> {
    (!text.is_empty()).then(|| text.split(' ').map(|word| (word != "skip").then(|| jsx! { <i key={word}>{word}</i> })).collect())
}

/// Text, borrowed and owned.
pub fn Label() -> &'static str {
    "label"
}

pub fn Shouted(TokensProps { text }: TokensProps) -> String {
    text.to_uppercase()
}

/// A number.
pub fn Count(TokensProps { text }: TokensProps) -> f64 {
    text.split(' ').count() as f64
}

/// Nothing.
pub fn Nothing() -> Option<JSX::Element> {
    None
}

/// A fragment, an element.
pub fn Pair() -> JSX::Element {
    jsx! { <><u>{"a"}</u><u>{"b"}</u></> }
}

/// An alias of what it renders, a list of maybe-elements.
pub type Nodes = Vec<Option<JSX::Element>>;

pub fn Aliased(TokensProps { text }: TokensProps) -> Nodes {
    vec![Some(jsx! { <s key="one">{text}</s> }), None]
}

/// A constant of a component's function type, a component too.
pub const T: for<'a> fn(TokensProps<'a>) -> Vec<JSX::Element> = Tokens;

/// A function of one argument, not capitalized, is no component: no tag's
/// macro of its name, which the glob below would bring in, hiding std's
/// \`format!\`.
pub mod util {
    pub fn format(text: String) -> String {
        text.to_uppercase()
    }
}

use util::*;

pub fn Formatted() -> String {
    format!("{}{}", format("a".to_string()), 1)
}

/// A constant of a function's type, not a component's: no tag of it.
pub const SCALE: fn(f64) -> f64 = double;

fn double(x: f64) -> f64 {
    x * 2.0
}

pub fn Page() -> JSX::Element {
    jsx! {
        <main>
            <Tokens text="hello rust js" />
            <Maybe text="keep skip this" />
            <Label />
            <Shouted text="loud" />
            <Count text="a b c" />
            <Nothing />
            <Pair />
            <Aliased text="alias" />
            <T text="by const" />
            {SCALE(2.0)}
            <Formatted />
        </main>
    }
}
`;

test("a component renders any node React renders, as React given them natively", async () => {
  const dir = fixture("jsx-renders");
  writeFileSync(join(dir, "lib.rs"), program);
  const built = Bun.spawnSync([compiler, ...flags(dir)]);
  // Built quietly: a constant of a function's type made no tag's macro that
  // goes unused.
  expect([built.exitCode, built.stderr.toString()]).toEqual([0, ""]);
  const { Page } = await import(join(dir, "lib.jsx"));
  const h = createElement;
  const native = h(
    "main",
    null,
    ["hello", "rust", "js"].map((word) => h("b", { key: word }, word)),
    [h("i", { key: "keep" }, "keep"), null, h("i", { key: "this" }, "this")],
    "label",
    "LOUD",
    3,
    null,
    h(Fragment, null, h("u", null, "a"), h("u", null, "b")),
    [h("s", { key: "one" }, "alias"), null],
    ["by", "const"].map((word) => h("b", { key: word }, word)),
    4,
    "A1",
  );
  expect(renderToStaticMarkup(h(Page))).toBe(renderToStaticMarkup(native));
});

// A return React doesn't render, of a component used as a tag, is rustc's
// error through react's component binding, which names `ReactNode`; and a
// required prop left out is said missing, of a component of any return.
test("a component whose return isn't a node is refused by rustc, naming ReactNode", () => {
  const dir = fixture("jsx-renders-refused");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Size {
    pub width: f64,
}

pub fn Measured() -> Size {
    Size { width: 1.0 }
}

pub fn Page() -> JSX::Element {
    jsx! { <main><Measured /></main> }
}
`);
  const failed = Bun.spawnSync([compiler, ...flags(dir)]);
  // And a required prop of one left out, its companion's own refusal.
  writeFileSync(join(dir, "missing.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx};
pub struct TokensProps<'a> {
    pub text: &'a str,
}
pub fn Tokens(TokensProps { text }: TokensProps) -> Vec<JSX::Element> {
    text.split(' ').map(|word| jsx! { <b key={word}>{word}</b> }).collect()
}
pub fn Page() -> JSX::Element {
    jsx! { <main><Tokens /></main> }
}
`);
  const missing = Bun.spawnSync([compiler, join(dir, "missing.rs"), "-o", join(dir, "missing.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  expect([missing.exitCode === 0, missing.stderr.toString()]).toEqual([false, expect.stringContaining("missing prop `text` of `TokensProps`")]);
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([
    false,
    expect.stringContaining("`fn() -> Size {Measured}` isn't a component of these props: a component takes its props, or none, and returns what React renders, a `ReactNode`"),
  ]);
});

// React renders a component again with the props it kept, twice in
// StrictMode: one that renders a `Vec`, changing a part of its props, is
// given a copy, as one that renders an element is (#1723), by the one
// `Rendered` every node is.
test("a component that renders a Vec copies the props it changes, Rust's answer under StrictMode", () => {
  const dir = fixture("jsx-renders-kept");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Point {
    pub x: f64,
}

pub struct DotProps {
    pub at: Point,
}

pub fn Moved(props: DotProps) -> Vec<JSX::Element> {
    let mut at = props.at;
    at.x += 1.0;
    vec![jsx! { <i key="x">{at.x}</i> }]
}
`);
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(js).toContain("export function Moved(props) {\n  const at = { ...props.at };\n  at.x++;\n");
  writeFileSync(join(dir, "driver.test.jsx"), `import { expect, test } from "bun:test";
import { StrictMode, createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { Moved } from "./lib.jsx";

test("it renders Rust's answer under StrictMode", () => {
  const at = { x: 1 };
  const container = document.createElement("div");
  flushSync(() => createRoot(container).render(createElement(StrictMode, null, createElement(Moved, { at }))));
  expect(container.textContent).toBe("2");
  expect(at).toEqual({ x: 1 });
});
`);
  const p = bunTest(["--preload", "./test/happydom.js", join(dir, "driver.test.jsx")]);
  const output = p.stdout + p.stderr;
  expect([p.code, output.match(/(\d+) pass/)?.[1]], output).toEqual([0, "1"]);
});
