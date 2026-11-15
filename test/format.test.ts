import { beforeAll, expect, test } from "bun:test";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { decodeMappings, lookup } from "./sourcemap";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

/** `rust-js fmt --stdin` of `source`, rustfmt's and the JSX pass's (#1901). */
function format(source: string) {
  return Bun.spawnSync([compiler, "fmt", "--stdin"], { stdin: Buffer.from(source), stderr: "pipe", cwd: root });
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

// A statement's line that continues it, a let chain's `&&`, a method
// chain's `.` or what `=` gives, is 4 in from it, as rustfmt lays it out;
// a block that opens a line, after a condition of several, is at its
// statement's (react.dev's Console).
test("JSX formatter indents a statement's continued lines", () => {
  const expected = `fn view(data: Vec<u32>) {
    jsx! {
        <div>
            {data.iter().map(|&msg| {
                let children;
                if let Some(value) = msg.checked_sub(1)
                    && value > 2
                {
                    children = value;
                } else {
                    children = 0;
                }
                let total: u32 = data
                    .iter()
                    .sum();
                let shown =
                    format!("{children}-{total}");
                jsx! { <span>{shown}</span> }
            })}
        </div>
    }
}
`;
  const result = format(expected.replace(/^ +/gm, " ").replace(" jsx! {\n <div>", "    jsx! {\n <div>"));
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
  // As rustfmt indents it, a skipped item's first line, its own.
  const method = `impl View {\n${skipped.replace(/^(#|fn)/gm, "    $1")}}\n`;
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
}, underLoad(30_000));

test("formatting preserves generated JSX and maps handlers to the formatted source", () => {
  const dir = fixture("format-map");
  const file = join(dir, "lib.rs");
  const source = `use react::{JSX, jsx};
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.record")] fn record(n: i32) { unreachable!() }
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

// `rust-js fmt`, the one way (#1901): `--format-jsx` is refused naming it;
// `--check` writes nothing, prints a diff and exits 1; a `jsx!` body rustfmt
// moved, a multi-line string's second line with it, is as written, its
// literal byte for byte; and with no paths, each target's module tree, as
// `cargo fmt` finds it, a file no target declares left as it is.
test("rust-js fmt is the one formatter: its check, its literals and its workspace", () => {
  const refused = Bun.spawnSync([compiler, "--format-jsx"], { stdin: Buffer.from(""), stderr: "pipe" });
  expect([refused.exitCode === 0, refused.stderr.toString()]).toEqual([false, expect.stringContaining("is `rust-js fmt` now")]);
  const moved = 'fn view() {\njsx! { <div>\n{"first\n second"}\n</div> }\n}\n';
  const out = format(moved).stdout.toString();
  expect(out).toBe('fn view() {\n    jsx! { <div>\n        {"first\n second"}\n    </div> }\n}\n');
  const checked = Bun.spawnSync([compiler, "fmt", "--stdin", "--check"], { stdin: Buffer.from(moved), stdout: "pipe", cwd: root });
  expect([checked.exitCode, checked.stdout.toString()]).toEqual([1, expect.stringContaining("--- <stdin>\n+++ <stdin>\n@@ -1,")]);
  const clean = Bun.spawnSync([compiler, "fmt", "--stdin", "--check"], { stdin: Buffer.from(out), stdout: "pipe", cwd: root });
  expect([clean.exitCode, clean.stdout.toString()]).toEqual([0, ""]);
  const files = fixture("fmt-files");
  writeFileSync(join(files, "moved.rs"), moved);
  expect(Bun.spawnSync([compiler, "fmt", join(files, "moved.rs")], { stderr: "pipe" }).exitCode).toBe(0);
  expect(readFileSync(join(files, "moved.rs"), "utf8")).toBe(out);
  const dir = fixture("fmt-workspace");
  mkdirSync(join(dir, "src", "a"), { recursive: true });
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "fmt_workspace"\nversion = "0.1.0"\nedition = "2024"\n\n[workspace]\n');
  writeFileSync(join(dir, "src", "lib.rs"), "mod a;\n// mod loose;\nconst S: &str = \"mod loose;\";\nfn  lib() {}\n");
  writeFileSync(join(dir, "src", "a.rs"), "mod b;\nfn  a() {}\n");
  writeFileSync(join(dir, "src", "a", "b.rs"), "fn  b() {}\n");
  writeFileSync(join(dir, "src", "loose.rs"), "fn  loose() {}\n");
  const listed = Bun.spawnSync([compiler, "fmt", "--check"], { cwd: dir, stdout: "pipe", stderr: "pipe" });
  const diffs = [...listed.stdout.toString().matchAll(/^--- (.+)$/gm)].map(([, path]) => path!.slice(dir.length + 1)).sort();
  expect([listed.exitCode, diffs]).toEqual([1, ["src/a.rs", "src/a/b.rs", "src/lib.rs"]]);
  expect(Bun.spawnSync([compiler, "fmt"], { cwd: dir, stderr: "pipe" }).exitCode).toBe(0);
  expect([readFileSync(join(dir, "src", "a", "b.rs"), "utf8"), readFileSync(join(dir, "src", "loose.rs"), "utf8")]).toEqual(["fn b() {}\n", "fn  loose() {}\n"]);
}, underLoad(60_000));
