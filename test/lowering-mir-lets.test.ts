// Where a `let` from MIR is declared and how it's named: where it's first
// named, in a loop's body or branch, a method's `self` after its type, a
// copy binding or a let-else's of a variable as the variable, a held
// temporary after the call that made it, and a name given apart twice
// reclaimed. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

test("a `let` from MIR is declared where it's first named", async () => {
  const dir = fixture("mir-sunk-lets");
  writeFileSync(join(dir, "lib.rs"), `pub fn label(path: Option<&str>, crumbs: &[&str], force: bool) -> String {
    let selected = path == Some("x");
    let item;
    if path.is_none() {
        item = "none".to_string();
    } else if crumbs.len() > 1 {
        let is_crumb = crumbs.len() > 2 && crumbs[0] == "y";
        let is_expanded = force || is_crumb || selected;
        item = format!("{is_expanded}");
    } else {
        item = "leaf".to_string();
    }
    item
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function label(path, crumbs, force) {
  const selected = path === "x";
  let item;
  if (path == null) {
    item = "none";
  } else if (crumbs.length > 1) {
    const isCrumb = crumbs.length > 2 && crumbs[0] === "y";
    const isExpanded = force || isCrumb || selected;
    item = String(isExpanded);
  } else {
    item = "leaf";
  }
  return item;
}`);
});

// What \`?\` of an \`Option\` gives a \`let\`, from MIR, is held under the
// name the program gives it, \`f\`: not \`value\`.
test("a let of an Option's ? from MIR is named as the program names it", async () => {
  const dir = fixture("mir-tried-named");
  writeFileSync(join(dir, "lib.rs"), `pub fn first_even(xs: &[i32]) -> Option<i32> {
    let f = xs.iter().find(|x| *x % 2 == 0)?;
    Some(f * 10)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const f = xs.find((x) => x % 2 === 0);\n  if (f == null) {\n    return;\n  }\n  return (f * 10) | 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.firstEven([1, 4]), lib.firstEven([1])]).toEqual([40, undefined]);
});

test("a `let` a loop's body makes, from MIR, is declared in the body", async () => {
  const dir = fixture("mir-loop-lets");
  writeFileSync(join(dir, "lib.rs"), `pub fn starts(lines: &[&str], ends: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    for end in ends {
        let mut start = 0;
        for i in 0..*end {
            start += lines[i].len() + 1;
        }
        out.push(start);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const end of ends) {\n    let start = 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.starts(["ab", "c"], [1, 2])).toEqual([3, 5]);
});

// A method's `self` is named after its type, `section` of a `Section`, as
// react.dev's TopNav names `Section.asStr`'s.
test("a method's self from MIR is named after its type", async () => {
  const dir = fixture("self-name");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "home")]
    Home,
}
impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Home => "home",
        }
    }
}
pub fn label(section: Section) -> String {
    format!("og-{}", section.as_str())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  asStr(section) {\n    return section;\n  },\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.label("home")).toBe("og-home");
});

// A pattern's binding of a variable, `if let Some(error) = raw`, a copy of
// it MIR makes where the variable is borrowed, `const error = raw;`: the
// variable itself where the block sets neither again, as react.dev's
// Preview renames a runtime error.
test("a copy binding of a variable is the variable from MIR", async () => {
  const dir = fixture("copy-binding");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct Error {
    pub title: Cell<Option<&'static str>>,
}
fn debounced(e: Option<&'static Error>) -> Option<&'static Error> {
    e
}
pub fn fixed(given: Option<&'static Error>, lint: Option<&'static Error>) -> Option<&'static str> {
    let mut raw = given;
    let first = &raw;
    if first.is_none_or(|e| e.title.get() == Some("Runtime Exception")) && lint.is_some() {
        raw = lint;
    }
    if let Some(error) = raw
        && error.title.get() == Some("Runtime Exception")
    {
        error.title.set(Some("Runtime Error"));
    }
    let error = debounced(raw);
    error.and_then(|e| e.title.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (raw != null && raw.title === "Runtime Exception") {\n    raw.title = "Runtime Error";\n  }');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.fixed({ title: "Runtime Exception" }, undefined), lib.fixed(undefined, { title: "x" })]).toEqual(["Runtime Error", "x"]);
});

// A temporary MIR holds of what a pattern takes apart, read more than once,
// is named after the call that made it, `find`, as rust-analyzer suggests
// (ADR 0569), not `_4`, a name no other in the function has, as react.dev's
// CodeBlock reads its regular expressions' matches.
test("a held temporary is named after the call that made it", async () => {
  const dir = fixture("temporary-name");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "globalThis.find")] #[allow(unused_variables)] fn find(text: &str) -> Option<(String, String)> { unreachable!() }
pub fn lines(meta: &str) -> String {
    let Some((_, lines)) = find(meta) else {
        return String::new();
    };
    lines.to_uppercase()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const find = globalThis.find(meta);\n  if (find == null) {");
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).find = (text: string) => (text ? [text, "ab"] : undefined);
  expect([lib.lines("x"), lib.lines("")]).toEqual(["AB", ""]);
});

// A `let` Rust declares in a loop's body is declared there,
// `const contentBoxSize = ..` where a branch of each turn makes it, not one
// `let` before the loop, as react.dev's NavigationBar measures its tabs.
test("a let a loop's branch makes is declared in the loop", async () => {
  const dir = fixture("loop-branch-let");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;
pub struct BoxSize {
    pub inline_size: f64,
}
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ContentBoxSize {
    Array(&'static [&'static BoxSize]),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Single(&'static BoxSize),
}
#[cfg_attr(rust_js, rust_js::link_name = "get contentBoxSize")]
#[allow(unused_variables)]
fn contentBoxSize(this: &Unknown) -> Option<ContentBoxSize> {
    unreachable!()
}
pub fn widths(entries: &[&Unknown], log: impl Fn(f64)) {
    for entry in entries {
        if let Some(size) = contentBoxSize(entry) {
            let contentBoxSize = match size {
                ContentBoxSize::Array(sizes) => unsafe { *sizes.get_unchecked(0) },
                ContentBoxSize::Single(size) => size,
            };
            let width = contentBoxSize.inline_size;
            log(width);
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("      const contentBoxSize = Array.isArray(size) ? size[0] : size;");
  const lib = await import(join(dir, "mir.js"));
  const widths: number[] = [];
  lib.widths([{ contentBoxSize: [{ inlineSize: 1 }] }, {}, { contentBoxSize: { inlineSize: 2 } }], (w: number) => widths.push(w));
  expect(widths).toEqual([1, 2]);
});

// What a `let`-`else` binds of a variable, `let Some(current) = &mut
// current else { continue; }`, is that variable, read as it is, not a
// copy, `const current$1 = current`, as react.dev's llmsTxt adds pages to
// its current section.
test("a let-else's binding of a variable is the variable", async () => {
  const dir = fixture("let-else-copy");
  writeFileSync(join(dir, "lib.rs"), `pub struct Section {
    pub pages: Vec<u32>,
}
pub fn grouped(items: &[u32]) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<Section> = None;
    for &item in items {
        if item == 0 {
            if let Some(section) = current {
                sections.push(section);
            }
            current = Some(Section { pages: Vec::new() });
            continue;
        }
        let Some(current) = &mut current else {
            continue;
        };
        current.pages.push(item);
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    current.pages.push(item);");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.grouped([1, 0, 2, 3, 0, 4])).toEqual([{ pages: [2, 3] }, { pages: [4] }]);
});

// A name given apart from one given apart already, `caption$1`'s, is the
// first's next, `caption$2`, which is the name itself where nothing it
// would shadow is read (ADR 0352), not `caption$1$1`, as react.dev's
// MDXComponents' IllustrationBlock names its captions from MIR.
test("a name given apart twice is reclaimed", async () => {
  const dir = fixture("fresh-twice");
  writeFileSync(join(dir, "lib.rs"), `pub struct Info {
    pub caption: Option<&'static str>,
}
pub fn shown(infos: &[Info]) -> Vec<String> {
    infos
        .iter()
        .map(|info| info.caption.filter(|caption| !caption.is_empty()).map(|caption| format!("<{caption}>")).unwrap_or_default())
        .collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("const caption = info.caption || undefined;");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.shown([{ caption: "a" }, { caption: "" }, {}])).toEqual(["<a>", "", ""]);
});

// A variable a loop's turn declares, `let y = match x { 0 => continue, ..
// }`, is declared in the loop's body from MIR: each arm assigns it or
// leaves the turn before it's read.
test("a let each turn assigns from MIR is the loop body's", async () => {
  const dir = fixture("turn-let");
  writeFileSync(join(dir, "lib.rs"), `pub fn named(v: &[u32]) -> Vec<String> {
    let mut out = Vec::new();
    for &x in v {
        let y = match x {
            0 => continue,
            1 => "one".to_string(),
            _ => format!("{x}"),
        };
        out.push(y);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("  for (const x of v) {\n    let y;\n");
  expect((await import(join(dir, "mir.js"))).named([0, 1, 7])).toEqual(["one", "7"]);
});
