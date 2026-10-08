import { beforeAll, expect, test } from "bun:test";
import { chmodSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { decodeMappings, lookup } from "./sourcemap";

beforeAll(buildReact, 600_000);

function format(source: string) {
  return Bun.spawnSync([compiler, "--format-jsx"], { stdin: Buffer.from(source), stderr: "pipe" });
}

test("JSX formatter aligns nested props and callbacks and is idempotent", () => {
  const expected = `fn view() {
    jsx! {
        <Pane
            editor={jsx! {
                <Editor
                    state={current}
                    onSubmit={Some(Rc::new(move || {
                        submit(false);
                    }))}
                />
            }}
        />
    }
}
`;
  const input = expected.replace(/^ +/gm, " ");
  // The JSX pass takes the surrounding Rust indentation as its baseline.
  const result = format(input.replace(" jsx!", "    jsx!"));
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  expect(result.stdout.toString()).toBe(expected);
  expect(format(expected).stdout.toString()).toBe(expected);
});

// A tag that's a value, `<Comp>`, may take attributes after its spread, as
// a DOM element may (ADR 0220): laid out where they are.
test("JSX formatter lays out attributes after a tag value's spread", () => {
  const expected = `fn view(Props { r#as: Comp, id, rest }: Props) {
    jsx! {
        <Comp
            id={id}
            {...rest}
            className="mdx-heading"
        />
    }
}
`;
  const result = format(expected.replace(/^ +/gm, " ").replace(" jsx!", "    jsx!"));
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  expect(result.stdout.toString()).toBe(expected);
});

test("JSX formatter preserves literal contents, comments, other macros and skipped items", () => {
  const preserved = [
    'r#"first\n  second\nlast"#',
    '"first\n second"',
    'stringify!(\n jsx! { untouched }\n)',
    'foreign::jsx!(\n not our grammar\n)',
    '/* first\n second */',
    'macro_rules! local {\n () => { opaque };\n}',
  ];
  const skipped = `#[rustfmt::skip]
fn skipped() { jsx! {
 <p />
} }
`;
  const skippedCall = `#[rustfmt::skip]
    jsx! {
 <p />
    };`;
  const source = `fn view<'a>(s: &'a str) {
    jsx! { <div title={${preserved[0]}}>
{${preserved[1]}}
{${preserved[2]}}
{${preserved[3]}}
{${preserved[4]}}
{${preserved[5]} if s.len() < 3 { s } else { "long" }}
    </div> }
}
${skipped}`;
  const result = format(source);
  expect(result.exitCode, result.stderr.toString()).toBe(0);
  const output = result.stdout.toString();
  for (const text of [...preserved, skipped]) expect(output).toContain(text);
  expect(format(output).stdout.toString()).toBe(output);
  expect(format("// plain Rust\nfn f() {}\n").stdout.toString()).toBe("// plain Rust\nfn f() {}\n");
  expect(format("").exitCode).toBe(0);
  const statement = `fn view() {\n    ${skippedCall}\n}\n`;
  expect(format(statement).stdout.toString()).toBe(statement);
  const method = `impl View {\n${skipped}}\n`;
  expect(format(method).stdout.toString()).toBe(method);
});

test("formatter check is read-only and invalid input prevents selected-file writes", () => {
  const dir = fixture("format");
  const file = join(dir, "input.rs");
  const bad = join(dir, "invalid.rs");
  const source = "fn view() { jsx! {\n <div />\n} }\n";
  writeFileSync(file, source);
  const command = ["bun", join(root, "scripts/format.ts")];
  expect(Bun.spawnSync([...command, "--check", file]).exitCode).toBe(1);
  expect(readFileSync(file, "utf8")).toBe(source);
  for (const invalid of ["fn broken( {", "fn view() { jsx! { <div></p> } }"]) {
    writeFileSync(bad, invalid);
    expect(Bun.spawnSync([...command, file, bad]).exitCode).not.toBe(0);
    expect(readFileSync(file, "utf8")).toBe(source);
    expect(readFileSync(bad, "utf8")).toBe(invalid);
    const result = format(invalid);
    expect(result.exitCode).not.toBe(0);
    expect(result.stdout.toString()).toBe("");
  }
  run([...command, file]);
  expect(readFileSync(file, "utf8")).toBe("fn view() {\n    jsx! {\n        <div />\n    }\n}\n");
  run([...command, "--check", file]);
}, 30_000);

test("formatting preserves generated JSX and maps handlers to the formatted source", () => {
  const dir = fixture("format-map");
  const file = join(dir, "lib.rs");
  const source = `use react::{JSX, jsx};
unsafe extern "Rust" { #[link_name = "globalThis.record"] safe fn record(n: i32); }
pub fn view() -> JSX::Element {
    jsx! {
 <button
 onClick={move |_| {
 record(7);
 record(8);
 }}
 >{"Save"}</button>
    }
}
`;
  const command = [compiler, file, "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  writeFileSync(file, source);
  run(command);
  const before = readFileSync(join(dir, "lib.jsx"), "utf8");
  run(["bun", join(root, "scripts/format.ts"), file]);
  run(command);
  const after = readFileSync(join(dir, "lib.jsx"), "utf8");
  expect(after).toBe(before);
  const formatted = readFileSync(file, "utf8");
  const map = JSON.parse(readFileSync(join(dir, "lib.jsx.map"), "utf8"));
  expect(map.sourcesContent).toEqual([formatted]);
  const lines = after.split("\n");
  const line = lines.findIndex(l => l.includes("globalThis.record(7)"));
  expect(lookup(decodeMappings(map.mappings), line, lines[line].indexOf("globalThis.record(7)"))?.srcLine)
    .toBe(formatted.split("\n").findIndex(l => l.includes("record(7);")));
});

// The formatter's JSX pass is the compiler's, the one given too, as an
// installed one is qualified with (ADR 0094), not always the checkout's
// debug build, which it would build first.
test("the formatter uses the compiler it's given", () => {
  const dir = fixture("format-given");
  const mark = join(dir, "called");
  const given = join(dir, "rust-js");
  writeFileSync(given, `#!/bin/sh\ntouch "${mark}"\ncat\n`);
  chmodSync(given, 0o755);
  const file = join(dir, "input.rs");
  writeFileSync(file, "fn f() {}\n");
  const p = Bun.spawnSync(["bun", join(root, "scripts/format.ts"), "--check", file], { env: { ...process.env, RUST_JS_COMPILER: given }, stderr: "pipe" });
  expect(p.exitCode, p.stderr.toString()).toBe(0);
  expect(existsSync(mark)).toBe(true);
});

// ADR 0278: a `let mut` nothing sets again, only changed in place, is a
// `const`, as react.dev's toCommaSeparatedList has `const list = []`; one
// set again stays a `let`.
test("a variable nothing sets again is a const", async () => {
  const dir = fixture("consts-of-lets");
  writeFileSync(join(dir, "lib.rs"), `pub fn pushed() -> Vec<u32> {
    let mut list = Vec::new();
    list.push(1);
    list
}
pub fn counted() -> u32 {
    let mut n = 0;
    n += 1;
    n
}
pub fn later(flag: bool) -> u32 {
    let n;
    if flag { n = 1 } else { n = 2 }
    n
}
fn bump(n: &mut u32) {
    *n += 1;
}
pub fn handled() -> u32 {
    let mut n = 0;
    bump(&mut n);
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const list = [];\n  list.push(1);");
  expect(js).toContain("  let n = 0;\n  n = (n + 1) >>> 0;");
  expect(js).toContain("  let n;\n  if (flag) {");
  expect(js).toContain("  let n = 0;\n  const n$1 = { value: n };");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pushed(), lib.counted(), lib.later(true), lib.later(false), lib.handled()]).toEqual([[1], 1, 1, 2, 1]);
});

// ADR 0279: a callback that only passes what it's given on, `(x) => double(x)`,
// is the function itself, `xs.map(double)`, as react.dev's
// toCommaSeparatedList has `array.map(renderCallback)`, where it's Rust's, which
// takes no more than it's given: a JS function's, `parseFloat`, may read
// the index too.
test("a callback that only passes its arguments on is the function", async () => {
  const dir = fixture("eta");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "parseFloat"]
    safe fn parse_float(text: &str) -> f64;
}

fn double(x: &u32) -> u32 {
    x * 2
}

pub fn doubled(xs: &[u32]) -> Vec<u32> {
    xs.iter().map(|x| double(x)).collect()
}

pub fn rendered(xs: &[u32], f: impl Fn(&u32, usize) -> u32) -> Vec<u32> {
    xs.iter().enumerate().map(|(i, x)| f(x, i)).collect()
}

pub fn parsed(xs: &[String]) -> Vec<f64> {
    xs.iter().map(|x| parse_float(x)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return xs.map(double);");
  expect(js).toContain("return xs.map(f);");
  expect(js).toContain("return xs.map((x) => parseFloat(x));");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.doubled([1, 2]), lib.rendered([5, 6], (x: number, i: number) => x + i), lib.parsed(["1.5", "2"])]).toEqual([[2, 4], [5, 7], [1.5, 2]]);
});

// An `else` of nothing, of only comments in Rust, which rust-js doesn't
// carry, is no `else`, as react.dev's _app has one.
test("an else of nothing is left out", async () => {
  const dir = fixture("empty-else");
  writeFileSync(join(dir, "lib.rs"), `pub fn picked(flag: bool, out: &mut Vec<u32>) {
    if flag {
        out.push(1);
    } else {
        // Nothing to do.
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (flag) {\n    out.push(1);\n  }\n}");
  const out: number[] = [];
  (await import(join(dir, "lib.js"))).picked(true, out);
  expect(out).toEqual([1]);
});
