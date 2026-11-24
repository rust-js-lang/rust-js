// If-let chains and or chains from MIR as written: arms before a panic in
// written order, a binding kept in its arm an else if, a value's truth part
// by part, a getter's bool one, and a call giving a bool one test. Split
// from lowering-branches.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// An `if let .. else if let .. else { panic!() }` chain is its arms in
// the order written, not the panic's first in a labeled block, as
// react.dev's TerminalBlock reads its message.
test("an if-let chain ending in a panic keeps its arms in written order", async () => {
  const dir = fixture("if-let-chain-panic");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Text(String),
    Element(Option<String>),
    Other,
}
pub fn message(kind: Kind) -> String {
    let message: String;
    if let Kind::Text(text) = &kind {
        message = text.to_string();
    } else if let Kind::Element(child) = &kind
        && let Some(text) = child
    {
        message = text.to_string();
    } else {
        panic!("Expected a plain string.");
    }
    message
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (kind.TAG === "Text") {\n    message = kind._0;\n  } else if (kind.TAG === "Element" && kind._0 != null) {\n    message = kind._0;\n  } else {\n    throw');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.message({ TAG: "Text", _0: "a" }), lib.message({ TAG: "Element", _0: "b" })]).toEqual(["a", "b"]);
  expect(() => lib.message({ TAG: "Element", _0: undefined })).toThrow("plain string");
});

// A binding a let chain makes in its condition, kept a `const` in its
// `then`, read twice there, is the `then`'s: the chain is still an `else
// if`, no label to reach its `else` (ADR 0659). Read once, first, it's the
// place the test read (ADR 0866, amended).
test("an if-let chain's binding kept in its arm keeps the chain an else if", async () => {
  const dir = fixture("if-let-chain-binding");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Text(String),
    Element(Vec<String>),
    Other,
}
pub fn message(kind: Kind) -> String {
    let message: String;
    if let Kind::Text(text) = &kind {
        message = text.to_string();
    } else if let Kind::Element(child) = &kind
        && let Some(text) = child.first()
        && text.len() > 1
    {
        message = text.clone() + text;
    } else {
        panic!("Expected a plain string.");
    }
    message
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  if (kind.TAG === "Text") {
    message = kind._0;
  } else if (kind.TAG === "Element" && kind._0[0] != null && $byteLen(kind._0[0]) > 1) {
    const text = kind._0[0];
    message = text + text;
  } else {
    throw $panic("Expected a plain string.", "lib.rs:16:9");
  }`);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.message({ TAG: "Text", _0: "a" }), lib.message({ TAG: "Element", _0: ["bc"] })]).toEqual(["a", "bcbc"]);
  expect(() => lib.message({ TAG: "Element", _0: ["b"] })).toThrow("plain string");
  expect(() => lib.message({ TAG: "Other" })).toThrow("plain string");
});

// An `||` chain whose part is an `Option`'s `is_some()`, of an object:
// `error != null || ..`, a `bool` part by part, not `!(error == null &&
// ..)`, as react.dev's Preview hides its content; one only tested too.
test("an or chain of a value's truth from MIR is a bool part by part", async () => {
  const dir = fixture("or-chain-truth");
  writeFileSync(join(dir, "lib.rs"), `pub struct Err {
    pub title: String,
}
pub fn hidden(error: Option<&Err>, height: Option<f64>, ready: bool) -> bool {
    error.is_some() || height.is_none_or(|h| h == 0.0) || !ready
}
pub fn kept(error: Option<&Err>, height: Option<f64>, ready: bool) -> String {
    let hide = error.is_some() || height.is_none_or(|h| h == 0.0) || !ready;
    if hide { "a".to_string() } else { "b".to_string() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return error != null || height == null || height === 0 || !ready;");
  expect(js).toContain("const hide = error != null || height == null || height === 0 || !ready;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.hidden({ title: "t" }, 3, true), lib.hidden(undefined, 3, true), lib.kept({ title: "t" }, 3, true), lib.kept(undefined, 3, true)]).toEqual([true, false, "a", "b"]);
});

// An `else if` chain's last arm before a panic is an `else if`, not a
// check in the `else`, `if (x !== "language-css") throw`, as react.dev's
// createFileMap picks a code block's file.
test("an else-if chain's arm before a panic stays an else if", async () => {
  const dir = fixture("else-if-panic");
  writeFileSync(join(dir, "lib.rs"), `pub fn path(meta: Option<&str>, class_name: &str) -> String {
    let file_path;
    if let Some(meta) = meta {
        file_path = format!("/{meta}");
    } else if class_name == "language-js" {
        file_path = "/App.js".to_string();
    } else if class_name == "language-css" {
        file_path = "/styles.css".to_string();
    } else {
        panic!("Code block is missing a filename: {class_name}");
    }
    file_path
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  } else if (className === "language-css") {\n    filePath = "/styles.css";\n  } else {\n    throw');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.path(undefined, "language-css"), lib.path("a.js", "x")]).toEqual(["/styles.css", "/a.js"]);
  expect(() => lib.path(undefined, "x")).toThrow("missing a filename");
});

// An `||` chain whose first test is a getter's `bool`, `element.isContentEditable`,
// is one chain returned, as react.dev's Search tells whether a key goes to
// what's being edited.
test("an or chain of a getter's bool is returned as one", async () => {
  const dir = fixture("getter-or-chain");
  writeFileSync(join(dir, "lib.rs"), `use react::webapi::HTMLElement;
pub fn editing(element: &HTMLElement) -> bool {
    let tag_name = element.tag_name();
    element.is_content_editable() || tag_name == "INPUT" || tag_name == "SELECT" || tag_name == "TEXTAREA"
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return (\n    element.isContentEditable ||\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.editing({ isContentEditable: false, tagName: "SELECT" }), lib.editing({ isContentEditable: false, tagName: "B" })]).toEqual([true, false]);
});

// A call of a function that gives a `bool`, `isItem(item)`, is one, so
// it's no `!!isItem(item)` in a test, as react.dev's CodeDiagram keeps its
// images; of a `dyn Fn` or a fn pointer that gives one too, `keep(item)`,
// which no function item says it is.
test("a call giving a bool is one in a test", async () => {
  const dir = fixture("bool-call");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub kind: String,
}
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.isItem")]
fn is_item(x: &Item) -> bool {
    unreachable!()
}
pub fn images(items: Vec<&Item>) -> Vec<&Item> {
    items.into_iter().filter(|item| is_item(item) && item.kind == "img").collect()
}
pub fn kept<'a>(items: Vec<&'a Item>, keep: &dyn Fn(&Item) -> bool) -> Vec<&'a Item> {
    items.into_iter().filter(|item| keep(item) && item.kind == "img").collect()
}
pub fn picked<'a>(items: Vec<&'a Item>, pick: fn(&Item) -> bool) -> Vec<&'a Item> {
    items.into_iter().filter(|item| pick(item) && item.kind == "img").collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('items.filter((item) => globalThis.isItem(item) && item.kind === "img")');
  expect(js).toContain('items.filter((item) => keep(item) && item.kind === "img")');
  expect(js).toContain('items.filter((item) => pick(item) && item.kind === "img")');
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).isItem = (x: { kind: string }) => x.kind !== "";
  const items = [{ kind: "img" }, { kind: "p" }, { kind: "" }];
  expect(lib.images(items)).toEqual([{ kind: "img" }]);
  expect([lib.kept(items, (x: { kind: string }) => x.kind !== ""), lib.picked(items, () => false)]).toEqual([[{ kind: "img" }], []]);
});
