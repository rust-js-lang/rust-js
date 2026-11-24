// What the compiler writes of an `Option` read through: an optional chain,
// `a?.b`, `a?.f()`, `a?.[k]`, that ends where it's `None`, of a property, a
// method's call or a key, as a getter's, a binding's and a dictionary's are
// read. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

// ADR 0030: a property chain of an option's value ends where it's `None`,
// `o?.inner.v`, one chain, as `map` does: `(o?.inner).v` would read `.v` of
// `undefined`.
test("an option's property chain ends where it's None", async () => {
  const dir = fixture("optional-chains");
  writeFileSync(join(dir, "lib.rs"), `pub struct Inner {
    pub v: u32,
}

pub struct Outer {
    pub inner: Inner,
}

pub fn deep(o: Option<&Outer>) -> Option<u32> {
    o.map(|x| x.inner.v)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return o?.inner.v;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.deep({ inner: { v: 3 } }), lib.deep(undefined)]).toEqual([3, undefined]);
});

// ADR 0030: a method of what's in an option, of a JS object, is `o?.m(x)`,
// which calls it, its arguments too, only where `o` is: as a person writes
// `document.body?.appendChild(a)`. A Rust type's method takes its value as
// an argument, `Node.add(o, x)`, which `?.` can't skip.
test("an option's method call ends where it's None", async () => {
  const dir = fixture("optional-calls");
  writeFileSync(join(dir, "lib.rs"), `pub struct Node {
    pub n: u32,
}

impl Node {
    pub fn add(&self, x: u32) -> u32 {
        self.n + x
    }
}

pub fn tested(o: Option<&js::RegExp>, x: &str) -> Option<bool> {
    o.map(|r| r.test(x))
}

pub fn ran(o: Option<&js::RegExp>, x: &str) {
    o.map(|r| r.test(x));
}

pub fn added(o: Option<&Node>, x: u32) -> Option<u32> {
    o.map(|node| node.add(x))
}

pub fn or_not(o: Option<&js::RegExp>, x: &str) -> bool {
    o.map_or(false, |r| r.test(x))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("return o?.test(x);");
  expect(js).toContain("  o?.test(x);");
  expect(js).toContain("return o != null ? Node.add(o, x) : undefined;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.tested(/a/, "a"), lib.tested(undefined, "a"), lib.ran(undefined, "a"), lib.added({ n: 1 }, 2), lib.added(undefined, 2), lib.orNot(undefined, "a")])
    .toEqual([true, undefined, undefined, 3, undefined, false]);
});

// ADR 0030: a getter's option mapped to a property of what's in it,
// `o.map(|e| e.id)`, is `o?.id`, which reads `o` once, as a `const` of it
// would, as react.dev's SocialBanner has `ref.current?.offsetHeight`.
test("a getter's option mapped to a property is an optional chain", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("getter-chain");
  writeFileSync(join(dir, "lib.rs"), `use webapi::Element;

pub fn first_id(parent: &Element) -> Option<String> {
    parent.first_element_child().map(|child| child.id())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return parent.firstElementChild?.id;");
  const { firstId } = await import(join(dir, "lib.js"));
  expect([firstId({ firstElementChild: { id: "a" } }), firstId({ firstElementChild: null })]).toEqual(["a", undefined]);
});

// A property of what may be `None`, `file.and_then(|f| js::get(f, "visible"))`,
// is `file?.visible`, and of a dictionary's, `files["/styles.css"]?.visible`,
// as react.dev's SandpackRoot reads it, and one of a variable's key,
// `files[name]?.active`: a key that's a name. Set where it's none, it's
// `counts[name] ??= ..`, as react.dev's CustomPreset counts lines. Another
// key is an index of the chain, `file?.["a-b"]` (ADR 0647), and so is one
// of a field JS gives, `text.node?.[key]`, read once (#1908).
test("a property read of what may be None is an optional chain", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("optional-property");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Unknown, dict};
pub struct File {
    pub active: Option<bool>,
}
pub fn active(files: &Dict<File>, name: &str) -> bool {
    dict::get(files, name).and_then(|f| f.active) == Some(true)
}
pub fn counted(counts: &Dict<f64>, name: &str, code: &str) -> f64 {
    if dict::get(counts, name).is_none() {
        dict::set(counts, name, code.split('\\n').count() as f64);
    }
    *dict::get(counts, name).unwrap()
}
pub fn styled(files: &Dict<&Unknown>) -> bool {
    !js::truthy(dict::get(files, "/styles.css").and_then(|f| js::get(*f, "visible")))
}
pub fn visible(file: Option<&Unknown>) -> bool {
    !js::truthy(file.and_then(|f| js::get(f, "visible")))
}
pub fn dashed(file: Option<&Unknown>) -> bool {
    js::truthy(file.and_then(|f| js::get(f, "a-b")))
}
pub struct Text {
    pub node: std::cell::Cell<Option<&'static Unknown>>,
}
pub fn keyed(text: &Text, key: &str) -> Option<&'static Unknown> {
    text.node.get().and_then(|node| js::get(node, key))
}
thread_local! {
    static CURRENT: std::cell::Cell<Option<&'static Unknown>> = const { std::cell::Cell::new(None) };
}
pub fn current_name() -> Option<&'static Unknown> {
    CURRENT.get().and_then(|v| js::get(v, "constructor")).and_then(|c| js::get(c, "name"))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return !file?.visible;");
  expect(js).toContain('return !files["/styles.css"]?.visible;');
  expect(js).toContain("return files[name]?.active === true;");
  expect(js).toContain('counts[name] ??= code.split("\\n").length;');
  expect(js).toContain('return !!file?.["a-b"];');
  // Of a field JS gives, read once (#1908): in place where one read does,
  // ReactART's `this._text?.[key]`, no `const` first.
  expect(js).toContain("return text.node?.[key];");
  // A module variable's, which no getter answers, read twice, one path,
  // `current?.constructor`, so one chain.
  expect(js).toContain("return current?.constructor?.name;");
  const { visible, dashed, styled, active, counted, keyed } = await import(join(dir, "lib.js"));
  expect([keyed({ node: { x: 4 } }, "x"), keyed({ node: undefined }, "x")]).toEqual([4, undefined]);
  expect([counted({}, "a", "x\ny"), counted({ a: 7 }, "a", "x"), active({ a: { active: true } }, "a"), active({}, "a"), styled({}), styled({ "/styles.css": { visible: true } }), visible(undefined), visible({}), visible({ visible: 1 }), dashed({ "a-b": 1 }), dashed(undefined)]).toEqual([2, 7, true, false, true, false, true, true, false, true, false]);
});
