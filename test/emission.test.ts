import { beforeAll, expect, test } from "bun:test";
import { join, relative } from "node:path";
import { root, target, run, buildReact, compiler } from "./support";

test("copied trait defaults map to their original source file", async () => {
  const { buildCompiler, fixture, compiler } = await import("./support");
  const { writeFileSync, readFileSync } = await import("node:fs");
  const { decodeMappings } = await import("./sourcemap");
  buildCompiler();
  const dir = fixture("source-origins");
  writeFileSync(join(dir, "lib.rs"), `mod contract; mod implementation;
    pub fn answer() -> i32 { implementation::answer() }`);
  const contract = `pub trait Value { fn value(&self) -> i32 { 42 } }`;
  writeFileSync(join(dir, "contract.rs"), contract);
  writeFileSync(join(dir, "implementation.rs"), `use crate::contract::Value;
    pub struct Item;
    impl Value for Item {}
    pub fn answer() -> i32 { Item.value() }`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect((await import(join(dir, "lib.js"))).answer()).toBe(42);
  const map = JSON.parse(readFileSync(join(dir, "implementation.js.map"), "utf8"));
  const source = map.sources.findIndex((path: string) => path.endsWith("contract.rs"));
  expect(source).toBeGreaterThanOrEqual(0);
  expect(map.sourcesContent[source]).toBe(contract);
  expect(decodeMappings(map.mappings).some(segment => segment.source === source
    && contract.slice(segment.srcCol).startsWith("42"))).toBe(true);
});

beforeAll(() => {
  buildReact();
  run([compiler, "examples/fib.rs", "-o", join(target, "fib.js")]);
}, 600_000);

// Source map: generated JS positions must point at the Rust that produced them.
test("source map points from fib.js back into fib.rs", async () => {
  const out = "fib.js";
  const { decodeMappings, lookup } = await import("./sourcemap.ts");
  const js = (await Bun.file(join(target, out)).text()).split("\n");
  const map = await Bun.file(join(target, `${out}.map`)).json();
  const rustSource = await Bun.file(join(root, "examples/fib.rs")).text();
  const rs = rustSource.split("\n");
  const segments = decodeMappings(map.mappings);

  // The map sits in target/, so it names the source relative to there,
  // wherever that is, a mutation's own too, and embeds the Rust source so
  // a debugger can show it.
  expect(map.sources).toEqual([relative(target, join(root, "examples/fib.rs"))]);
  expect(map.sourcesContent).toEqual([rustSource]);

  // Nothing in the header or runtime helpers maps to Rust.
  const firstFunction = js.findIndex((l) => l.startsWith("export function"));
  expect(Math.min(...segments.map((s) => s.jsLine))).toBe(firstFunction);

  // Every mapping lands inside the Rust file.
  for (const s of segments) {
    expect(s.srcLine).toBeLessThan(rs.length);
    expect(s.srcCol).toBeLessThanOrEqual(rs[s.srcLine].length);
  }

  // JS snippet  →  the Rust text its position maps to.
  const probes: [string, string][] = [
    ["export function fib(", "pub fn fib("],
    ["fib(n - 1)", "fib(n - 1)"],
    // A `while` that steps its own variable last is a `for` (ADR 0974),
    // whose test is the `while`'s.
    ["i < n; i++)", "i < n"],
    ["if (i === n)", "if i == n"],
    ["(((x * 3) | 0) - 7) | 0", "x * 3 - 7"],
    ["$div(a, b, -2147483648) | 0", "a / b"],
    // A two-arm match's test, written as an if's (ADR 0237, amended), is
    // where the match is.
    ['order === "Ascending"', "match order"],
    ["fibIter((20 - n) >>> 0)", "fib_iter(20 - n)"],
    // A `return` where its value is written, not where the body ends.
    ["return a;", "a"],
  ];
  for (const [jsText, rustText] of probes) {
    const line = js.findIndex((l, i) => i >= firstFunction && l.includes(jsText));
    const col = js[line].indexOf(jsText);
    const hit = lookup(segments, line, col);
    const mapped = hit ? rs[hit.srcLine].slice(hit.srcCol) : "(no mapping)";
    expect([jsText, mapped.startsWith(rustText) ? rustText : mapped]).toEqual([jsText, rustText]);
  }
});

// A stack trace through the map (Jest's source-map-support, Node's
// --enable-source-maps) names a frame by the map's name where the frame is
// called from, so a name would be what the frame shows: `callback: impl
// FnOnce(A, B) -> T` for `callback(a, b)`, a closure's text for the function
// it became, `begin_work` for `beginWork`. The frame shows the JS's own name,
// as a person's JS would: the map has none (ADR 0018, amended).
test("a source map names nothing, so a frame through it is named as the JS is", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync, readFileSync } = await import("node:fs");
  const dir = fixture("source-map-names");
  writeFileSync(join(dir, "lib.rs"), `pub fn run_with<A, B, T>(callback: impl FnOnce(A, B) -> T, a: A, b: B) -> T {
    callback(a, b)
}
pub fn begin_work(current: Option<u32>, lanes: u32) -> u32 {
    current.unwrap_or(lanes)
}
pub fn perform(current: Option<u32>, lanes: u32) -> u32 {
    run_with(|current: Option<u32>, lanes: u32| begin_work(current, lanes), current, lanes)
}
pub fn perform_after(current: Option<u32>, lanes: u32) -> u32 {
    let shift = lanes;
    run_with(|current: Option<u32>, lanes: u32| begin_work(current, lanes) + shift, current, lanes)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return runWith(beginWork, current, lanes);");
  const map = JSON.parse(readFileSync(join(dir, "lib.js.map"), "utf8"));
  expect(map.names).toEqual([]);
  expect((await import(join(dir, "lib.js"))).performAfter(null, 2)).toBe(4);
});

// JSX's output extension must participate in collision checks before writing.
test("root and child output collisions leave no partial artifacts", async () => {
  const { fixture, compiler } = await import("./support");
  const { existsSync, writeFileSync } = await import("node:fs");
  for (const jsx of [false, true]) {
    const dir = fixture("collision");
    const input = join(dir, "lib.rs"), output = join(dir, "same.js");
    const body = jsx ? 'use react::jsx; pub fn f() -> react::JSX::Element { jsx! { <div /> } }' : 'pub fn f() -> i32 { 1 }';
    writeFileSync(input, `${body}\npub mod same { ${body} }`);
    const p = Bun.spawnSync([compiler, input, "-o", output, "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
    expect(p.exitCode).not.toBe(0);
    // The same file twice, not two that differ by case.
    expect(p.stderr.toString()).toContain("output collision at `");
    expect(p.stderr.toString()).toContain("; pick another output name");
    expect(existsSync(output)).toBe(false);
    expect(existsSync(join(dir, "same.jsx"))).toBe(false);
  }
});

// `shapes.js` and `Shapes.js` are one file on macOS and Windows: refused,
// naming both, where Rust lets two inline modules differ by case alone.
test("modules whose files differ only by case are refused", async () => {
  const { fixture, compiler } = await import("./support");
  const { existsSync, writeFileSync } = await import("node:fs");
  const dir = fixture("case-collision");
  const input = join(dir, "lib.rs"), output = join(dir, "lib.js");
  writeFileSync(input, "pub mod shapes { pub fn f() -> i32 { 1 } }\n#[allow(non_snake_case)]\npub mod Shapes { pub fn g() -> i32 { 7 } }");
  const p = Bun.spawnSync([compiler, input, "-o", output]);
  expect(p.exitCode).not.toBe(0);
  expect(p.stderr.toString()).toContain(`output collision at \`${join(dir, "shapes.js")}\` and \`${join(dir, "Shapes.js")}\`, which differ only by case`);
  expect(existsSync(output)).toBe(false);
});

test("manifest owns artifacts and records even modules that emit no code", async () => {
  const { fixture, compiler } = await import("./support");
  const { existsSync, writeFileSync } = await import("node:fs");
  const dir = fixture("manifest");
  const input = join(dir, "lib.rs"), output = join(dir, "lib.js"), manifest = join(dir, "manifest.json");
  writeFileSync(join(dir, "types.rs"), "pub struct Props { pub x: i32 }");
  writeFileSync(input, 'mod types; use react::jsx; pub fn f() -> react::JSX::Element { jsx! { <div /> } }');
  const args = [compiler, input, "-o", output, "--manifest", manifest, "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  run(args);
  const first = await Bun.file(manifest).json();
  expect(first.sources).toContain(join(dir, "types.rs"));
  expect(first.artifacts.map((a: { file: string }) => a.file)).toContain(join(dir, "lib.jsx"));
  writeFileSync(input, "pub fn f() -> i32 { 2 }");
  run(args);
  expect(existsSync(join(dir, "lib.jsx"))).toBe(false);
  expect(existsSync(join(dir, "lib.jsx.map"))).toBe(false);
  expect(existsSync(output)).toBe(true);
  writeFileSync(output, "user edited this file");
  writeFileSync(input, 'use react::jsx; pub fn f() -> react::JSX::Element { jsx! { <div /> } }');
  run(args);
  expect(await Bun.file(output).text()).toBe("user edited this file");
});

test("mixed JS and JSX modules use their final paths in imports and the manifest", async () => {
  const { fixture, compiler } = await import("./support");
  const dir = fixture("mixed-modules");
  await Bun.write(join(dir, "lib.rs"), `mod view; mod plain; pub fn f() -> (react::JSX::Element, i32) { (view::render(), plain::value()) }`);
  await Bun.write(join(dir, "view.rs"), 'use react::jsx; pub fn render() -> react::JSX::Element { jsx! { <div /> } }');
  await Bun.write(join(dir, "plain.rs"), 'pub fn value() -> i32 { 1 }');
  const manifest = join(dir, "manifest.json");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--manifest", manifest, "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const code = await Bun.file(join(dir, "lib.js")).text();
  expect(code).toContain('from "./view.jsx"');
  expect(code).toContain('from "./plain.js"');
  const result = await Bun.file(manifest).json();
  expect(result.modules.find((m: { module: string[] }) => m.module.length === 0).imports.sort()).toEqual([join(dir, "plain.js"), join(dir, "view.jsx")]);
  expect(result.modules.find((m: { module: string[] }) => m.module[0] === "view").map).toBe(join(dir, "view.jsx.map"));
});

test("an output map symlink cannot overwrite an input source", async () => {
  const { fixture, compiler } = await import("./support");
  const { symlinkSync } = await import("node:fs");
  const dir = fixture("source-collision");
  const input = join(dir, "lib.rs"), output = join(dir, "out.js");
  const source = "pub fn value() -> i32 { 1 }";
  await Bun.write(input, source);
  symlinkSync(input, output + ".map");
  const p = Bun.spawnSync([compiler, input, "-o", output]);
  expect(p.exitCode).not.toBe(0);
  expect(p.stderr.toString()).toContain("collision");
  expect(await Bun.file(output).exists()).toBe(false);
  expect(await Bun.file(input).text()).toBe(source);
});

// A file's own leading comments, a license's, are its JS's, after rust-js's
// header, as the file it replaced had them (ADR 0383); a doc comment is the
// item's or the module's, and a block comment Rust nests, which JS can't
// hold, ends them. What's after them is where its code is (ADR 0444).
test("a file's leading comments are its JS's", async () => {
  const { buildCompiler, fixture, compiler } = await import("./support");
  const { writeFileSync, readFileSync } = await import("node:fs");
  buildCompiler();
  const dir = fixture("leading-comments");
  const license = "/*\n * Copyright (c) Meta Platforms, Inc. and affiliates.\n */\n\n// MIT licensed.";
  writeFileSync(join(dir, "lib.rs"), `${license}\n\n//! The crate's docs.\n\n/// The answer's docs.\npub fn answer() -> i32 {\n    42\n}\n\nmod nested;\nmod documented;\nmod tight;\n`);
  writeFileSync(join(dir, "documented.rs"), "// Kept too.\n/** The item's docs. */\npub fn two() -> i32 {\n    2\n}\n");
  // A doc comment right under the header, no blank line between, is its
  // item's, a JSDoc, not the header's, and right under it in the JS too,
  // as written (ADR 0383, amended).
  writeFileSync(join(dir, "tight.rs"), "// Kept.\n/// Its docs.\npub fn four() -> i32 {\n    4\n}\n");
  writeFileSync(join(dir, "nested.rs"), "// Kept.\n/* not /* nested */ kept */\n// The function's.\npub fn one() -> i32 {\n    1\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toStartWith(`// Generated by rust-js from ${join(dir, "lib.rs")}. Do not edit.\n\n${license}\n`);
  expect(js).toContain(`${license}\n\n// The crate's docs.\n\n/** The answer's docs. */\nexport function answer() {`);
  expect(readFileSync(join(dir, "tight.js"), "utf8")).toContain("// Kept.\n/** Its docs. */\nexport function four() {");
  const nested = readFileSync(join(dir, "nested.js"), "utf8");
  expect(nested).toContain("\n// Kept.\n");
  expect(nested).not.toContain("nested */");
  expect(nested).toContain("\n// Kept.\n// The function's.\nexport function one() {");
  // A line comment JS would end early, at a carriage return, U+2028 or
  // U+2029, where Rust's goes on, ends them: rustc's lexer tests have one.
  writeFileSync(join(dir, "crlf.rs"), "// Kept, its line's end CRLF.\r\n// Not kept: a\rbare CR.\npub fn three() -> i32 {\n    3\n}\n");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(dir, "lib.rs"), "utf8") + "mod crlf;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const crlf = readFileSync(join(dir, "crlf.js"), "utf8");
  expect(crlf).toContain("// Kept, its line's end CRLF.");
  expect(crlf).not.toContain("bare CR");
  expect(crlf).not.toContain("\r");
  const documented = readFileSync(join(dir, "documented.js"), "utf8");
  expect(documented).toContain("\n// Kept too.\n");
  expect(documented).toContain("\n// Kept too.\n/** The item's docs. */\nexport function two() {");
  // A run right above the code, no blank line between, is the code's, a
  // blank line parting it from the license (ADR 0383, amended).
  writeFileSync(join(dir, "after.rs"), "// The license.\n\n// The function's, right above it.\npub fn four() -> i32 {\n    4\n}\n");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(dir, "lib.rs"), "utf8") + "mod after;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "after.js"), "utf8")).toContain("Do not edit.\n\n// The license.\n\n// The function's, right above it.\nexport function four() {");
  // Of a module that imports, its header is above its imports: an item's
  // doc comment and the run right above its code stay the item's, after
  // them.
  writeFileSync(join(dir, "imported_docs.rs"), "// Kept.\n/** The item's docs. */\npub fn two() -> i32 {\n    answer() - 40\n}\n\nuse super::answer;\n");
  writeFileSync(join(dir, "imported_after.rs"), "// The license.\n\n// The function's, right above it.\npub fn four() -> i32 {\n    answer() / 10\n}\n\nuse super::answer;\n");
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(dir, "lib.rs"), "utf8") + "mod imported_docs;\nmod imported_after;\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "imported_docs.js"), "utf8")).toContain('Do not edit.\n\n// Kept.\n\nimport { answer } from "./lib.js";\n\n/** The item\'s docs. */\nexport function two() {');
  expect(readFileSync(join(dir, "imported_after.js"), "utf8")).toContain('Do not edit.\n\n// The license.\n\nimport { answer } from "./lib.js";\n\n// The function\'s, right above it.\nexport function four() {');
  expect((await import(join(dir, "lib.js"))).answer()).toBe(42);
});

// ADR 0649: a function marked `#[inline(never)]` is written with Closure
// Compiler's `/** @noinline */` before it, as React's testStringCoercion
// is, in its JSDoc where it has one; nor does rust-js put its body in its
// caller's place, a forwarding function's variable (ADR 0610). `#[inline]`
// and `#[inline(always)]` are a JS engine's to decide, and say nothing.
test("a function marked #[inline(never)] is @noinline", async () => {
  const { buildCompiler, fixture, compiler } = await import("./support");
  const { writeFileSync, readFileSync } = await import("node:fs");
  buildCompiler();
  const dir = fixture("noinline");
  writeFileSync(join(dir, "lib.rs"), `#[inline(never)]
fn test_string_coercion(value: f64) -> String {
    // If you ended up here by following an exception call stack, here's what's
    // happened.
    format!("{value}")
}

/// Its docs.
#[inline(never)]
pub fn documented(n: i32) -> i32 {
    n + 1
}

/// Its docs,
/// over two lines.
#[inline(never)]
pub fn long(n: i32) -> i32 {
    n + 2
}

#[inline]
pub fn hinted(n: i32) -> i32 {
    n * 2
}

#[inline(always)]
pub fn always(n: i32) -> i32 {
    n * 3
}

thread_local! {
    static GET: fn(i32) -> i32 = |n| n;
}

#[inline(never)]
pub fn forwarded(n: i32) -> i32 {
    GET.with(|get| get(n))
}

pub fn coerced(value: f64) -> String {
    test_string_coercion(value)
}

#[derive(Clone, Copy)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
}

impl Section {
    #[inline(never)]
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
        }
    }
}

pub fn named(section: Section) -> &'static str {
    section.as_str()
}

// React's \`UnknownOwner\`, whose inner frame \`Error.stack\` shows, kept out
// of line by Closure Compiler as upstream's IIFE is.
#[inline(never)]
pub fn unknown_owner() -> String {
    #[inline(never)]
    fn react_stack_top_frame() -> String {
        // The frame React's owner stack pops.
        "react-stack-top-frame".to_string()
    }
    react_stack_top_frame()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("/** @noinline */\nfunction testStringCoercion(value) {\n  // If you ended up here");
  expect(js).toContain("/**\n * Its docs.\n * @noinline\n */\nexport function documented(n) {");
  expect(js).toContain("/**\n * Its docs,\n * over two lines.\n * @noinline\n */\nexport function long(n) {");
  expect(js).toContain("\nexport function hinted(n) {");
  expect(js).toContain("\nexport function always(n) {");
  expect(js.match(/@noinline/g)?.length).toBe(7);
  // A function written in a body, its own comment before it (ADR 0649, amended).
  expect(js).toContain(
    "/** @noinline */\nexport function unknownOwner() {\n  /** @noinline */\n  function reactStackTopFrame() {\n    // The frame React's owner stack pops.\n",
  );
  expect(js).toContain("@noinline */\nexport function forwarded(n) {");
  // A function giving each variant its own name is called, not its
  // argument (ADR 0264).
  expect(js).toContain("  /** @noinline */\n  asStr(section) {");
  expect(js).toContain("return Section.asStr(section);");
});
