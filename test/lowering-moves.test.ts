// Where a temporary MIR made is read: in its place, where what the
// program does stays the same (src/lower/mir/tidy/moves.rs, ADR 0750).
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate, with
 * `flags`: its JS, and its module. */
async function compiled(
  name: string,
  rust: string,
  flags: string[] = [],
): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...flags, "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's addValueToProperties pushes `[prefix + '\xa0\xa0'.repeat(indent)
// + propertyName, objectName === 'Object' ? .. : objectName]`, each
// branch where it's written (ADR 0969, amended). MIR makes
// the pair's first part before the branch of its second: once that's a
// conditional, the part is read in the pair, past `properties.push`, the
// method its array has, not held, `const value$1 = ..`.
test("a call's value is read in a method's arguments, past the method", async () => {
  const { js, lib } = await compiled("read-past-a-method", `pub fn pushed(name: &str, object_name: &str, indent: usize, out: &mut Vec<(String, String)>) {
    out.push((
        "-".repeat(indent) + name,
        if object_name == "Object" {
            if indent < 3 { String::new() } else { "\\u{2026}".to_string() }
        } else {
            object_name.to_string()
        },
    ));
}
`);
  expect(js).toContain('  out.push([\n    "-".repeat(indent) + name,\n    objectName === "Object" ? (indent < 3 ? "" : "…") : objectName,\n  ]);\n');
  expect(js).not.toMatch(/const (value|_\d+)/);
  const out: [string, string][] = [];
  lib.pushed("a" as never, "Object" as never, 1 as never, out as never);
  lib.pushed("b" as never, "Map" as never, 4 as never, out as never);
  lib.pushed("c" as never, "Object" as never, 4 as never, out as never);
  expect(out).toEqual([["-a", ""], ["----b", "Map"], ["----c", "…"]]);
});

// A binding's method of a JS value, `obj.send(a, b)`, is looked up when
// it's called, after Rust has run its arguments: a getter may answer the
// lookup, and an argument may replace the method. A call's value isn't
// read past it, `const effect = ..; obj.send(effect, ..)`, as Rust runs it
// (#187); what does nothing, `flag ? 1 : 2`, is read in place still.
test("a binding's method is looked up after its arguments run", async () => {
  const { js, lib } = await compiled("read-past-a-binding-method", `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.effect")]
fn effect() -> i32 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "send")]
fn send(this: &Unknown, a: i32, b: i32) {
    unreachable!()
}

pub fn run(obj: &Unknown, flag: bool) {
    send(obj, effect(), if flag { 1 } else { 2 });
}
`);
  expect(js).toContain("  const effect = globalThis.effect();\n  obj.send(effect, flag ? 1 : 2);\n");
  const g = globalThis as Record<string, unknown>;
  // A getter answers the lookup: after the argument, as Rust runs them.
  const events: string[] = [];
  g.effect = () => {
    events.push("argument");
    return 7;
  };
  const got: number[][] = [];
  const watched = {
    get send() {
      events.push("method lookup");
      return (a: number, b: number) => got.push([a, b]);
    },
  };
  lib.run(watched as never, true as never);
  expect(events).toEqual(["argument", "method lookup"]);
  // An argument that replaces the method: the new one is called.
  const called: string[] = [];
  const replaced: Record<string, (a: number, b: number) => void> = { send: () => called.push("old") };
  g.effect = () => {
    replaced.send = () => called.push("new");
    return 1;
  };
  lib.run(replaced as never, false as never);
  expect(called).toEqual(["new"]);
  expect(got).toEqual([[7, 1]]);
});

// A getter's read given to a binding's method, `send(obj, value(source))`,
// runs before the method's looked up, as Rust runs it, nested in an
// argument too, `value(source) + 1`: a read a getter may answer does
// something, `has_effects` (#188).
test("a getter's read given to a binding's method runs before its lookup", async () => {
  const { js, lib } = await compiled("getter-past-a-binding-method", `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "get value")]
fn value(this: &Unknown) -> i32 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "send")]
fn send(this: &Unknown, a: i32, b: i32) {
    unreachable!()
}

pub fn run(obj: &Unknown, source: &Unknown, flag: bool) {
    send(obj, value(source), if flag { 1 } else { 2 });
}

pub fn nested(obj: &Unknown, source: &Unknown) {
    send(obj, value(source) + 1, 2);
}
`);
  expect(js).toContain("  const value = source.value;\n  obj.send(value, flag ? 1 : 2);\n");
  expect(js).toContain("  const value = (source.value + 1) | 0;\n  obj.send(value, 2);\n");
  for (const name of ["run", "nested"] as const) {
    // What the getter does, the method's lookup sees.
    const events: string[] = [];
    let n = 0;
    const source = {
      get value() {
        events.push("argument getter");
        n = 7;
        return n;
      },
    };
    const obj = {
      get send() {
        events.push("method lookup");
        const seen = n;
        return () => events.push(`call saw ${seen}`);
      },
    };
    lib[name](obj as never, source as never, true as never);
    expect(events).toEqual(["argument getter", "method lookup", "call saw 7"]);
    // A getter that replaces the method: the new one is called.
    const called: string[] = [];
    const target: Record<string, () => void> = { send: () => called.push("old") };
    const replacing = {
      get value() {
        target.send = () => called.push("new");
        return 1;
      },
    };
    lib[name](target as never, replacing as never, false as never);
    expect(called).toEqual(["new"]);
  }
});

// An `Option` a getter gives, tested and kept, `filter` and `is_some_and`'s
// `o != null && o > 1 ? o : undefined`: read once, in a `const`, as Rust
// reads it, not at each use: a getter may give another value, or count
// its reads (`Expr::reads_same`, #188).
test("a getter's Option a combinator reads more than once is read once", async () => {
  const { js, lib } = await compiled("getter-option-read-once", `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "get count")]
fn count(this: &Unknown) -> Option<u32> {
    unreachable!()
}

pub fn big(source: &Unknown) -> Option<u32> {
    count(source).filter(|n| *n > 1)
}

pub fn has_big(source: &Unknown) -> bool {
    count(source).is_some_and(|n| n > 1)
}
`);
  expect(js).toContain("  const option = source.count;\n  return option != null && option > 1 ? option : undefined;\n");
  let reads = 0;
  const source = {
    get count() {
      reads++;
      return reads;
    },
  };
  // Rust reads 1, which `filter` drops; read again, it would see 2 and keep 3.
  expect(lib.big(source as never)).toBeUndefined();
  expect(reads).toBe(1);
  expect(lib.hasBig(source as never)).toBe(true);
  expect(reads).toBe(2);
});

// React's addObjectDiffToProperties: `if (readReactElementTypeof(nextValue)
// === REACT_ELEMENT_TYPE) { .. continue; } else { .. }`, a test MIR holds
// before the blocks its arms' jumps leave, `const _183 = ..; bb200: {
// bb291: { if (_183) ..`. Entering a block runs nothing: the test is read
// where it's tested, not held as a `const value`.
test("a temporary is read in the block after it that reads it", async () => {
  const { js, lib } = await compiled("read-in-a-block", `use js::{SymbolFor, Unknown};

const ELEMENT: SymbolFor = SymbolFor("react.element");

fn tag(value: &Unknown) -> Option<&'static Unknown> {
    js::get(value, "tag")
}

pub fn counted(values: &[&Unknown], out: &mut Vec<String>) -> usize {
    let mut other = 0;
    for &value in values {
        if matches!(SymbolFor::from_value(tag(value)), Some(ELEMENT)) {
            if js::get(value, "skip").is_some() {
                out.push("skipped".to_string());
                continue;
            }
        } else {
            let kind = js::object::to_string(Some(value)).unwrap();
            if kind == "[object Array]" {
                out.push(kind);
                continue;
            }
        }
        other += 1;
    }
    other
}
`);
  expect(js).toContain("      if (tag(value) === ELEMENT) {\n");
  expect(js).not.toMatch(/const (value\$?\d*|_\d+) = /);
  const element = Symbol.for("react.element");
  const out: string[] = [];
  const values = [{ tag: element }, { tag: element, skip: 1 }, [], {}];
  expect(lib.counted(values as never, out as never)).toBe(2);
  expect(out).toEqual(["skipped", "[object Array]"]);
});

// React's enqueueTask (shared/enqueueTask.js) sets `channel.port1.onmessage
// = () => { callback(); }`. A library's `Box<dyn FnMut>` drops what the
// functions it and its libraries make hold, as a program's does (ADR 0626,
// amended): none here, of every callback the closure holds, so nothing
// holds the closure past the call, `const value = () => ..`, and it's
// written where it's given (ADR 0800).
test("a closure given to a setter of a getter's object is written where it's given", async () => {
  const { js, lib } = await compiled(
    "closure-to-a-setter",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "get port1")]
fn port1(this: &Unknown) -> &'static Unknown {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set onmessage")]
#[cfg_attr(rust_js, rust_js::nullable(value))]
fn set_onmessage(this: &Unknown, value: Option<Box<dyn FnMut(&Unknown)>>) {
    unreachable!()
}

pub fn by_ref(channel: &Unknown, callback: &'static dyn Fn()) {
    set_onmessage(port1(channel), Some(Box::new(move |_| callback())));
}

pub fn by_fn(channel: &Unknown, callback: fn()) {
    set_onmessage(port1(channel), Some(Box::new(move |_| callback())));
}

pub fn by_number(channel: &Unknown, n: u32) {
    set_onmessage(port1(channel), Some(Box::new(move |_| println!("{n}"))));
}

pub fn by_nothing(channel: &Unknown) {
    set_onmessage(port1(channel), Some(Box::new(|_| println!("none"))));
}
`,
    ["--library", "--manifest", join(fixture("closure-to-a-setter-manifest"), "manifest.json")],
  );
  for (const [name, body] of [
    ["byRef(channel, callback)", "callback();"],
    ["byFn(channel, callback)", "callback();"],
    ["byNumber(channel, n)", "console.log(`${n}`);"],
    ["byNothing(channel)", 'console.log("none");'],
  ]) {
    expect(js).toContain(`export function ${name} {\n  channel.port1.onmessage = () => {\n    ${body}\n  };\n}\n`);
  }
  expect(js).not.toMatch(/const (value|_\d+)/);
  // The getter runs, then the closure is given, as Rust runs them.
  const events: string[] = [];
  const port: { onmessage?: () => void } = {};
  const channel = {
    get port1() {
      events.push("port1");
      return port;
    },
  };
  lib.byRef(channel as never, (() => events.push("callback")) as never);
  port.onmessage?.();
  expect(events).toEqual(["port1", "callback"]);
});

// Under a drop guard each call is in its own `try`, and the closure a
// temporary MIR made: what does nothing, of variables nothing changes, is
// read where it's given past what the assignment's target does, the
// getter, as it would be read there anywhere (#393).
test("a closure given to a setter under a drop guard is written where it's given", async () => {
  const { js, lib } = await compiled(
    "closure-to-a-setter-guarded",
    `use js::Unknown;

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "get port1")]
fn port1(this: &Unknown) -> &'static Unknown {
    let _ = this;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set onmessage")]
#[cfg_attr(rust_js, rust_js::nullable(value))]
fn set_onmessage(this: &Unknown, value: Option<Box<dyn FnMut(&Unknown)>>) {
    let _ = (this, value);
    unreachable!()
}

pub fn guarded(channel: &Unknown, callback: &'static dyn Fn()) {
    let _guard = Loud(1);
    set_onmessage(port1(channel), Some(Box::new(move |_| callback())));
}
`,
  );
  expect(js).toContain("  try {\n    channel.port1.onmessage = () => {\n      callback();\n    };\n  } finally {\n");
  expect(js).not.toMatch(/function value|const (value|_\d+)/);
  // The getter runs, then the closure is given, then the guard drops, as
  // Rust runs them.
  const events: string[] = [];
  const port: { onmessage?: () => void } = {};
  const channel = {
    get port1() {
      events.push("port1");
      return port;
    },
  };
  const log = console.log;
  console.log = (text: string) => events.push(text);
  try {
    lib.guarded(channel as never, (() => events.push("callback")) as never);
  } finally {
    console.log = log;
  }
  port.onmessage?.();
  expect(events).toEqual(["port1", "drop 1", "callback"]);
});

// An import's member is one JS's modules never change: `tags.link`, made
// before a branch whose arm calls, is read where it's given, past the
// branch, not held, `const link = tags.link` (`Body::plain_names`).
test("an import's member is read past a branch that calls", async () => {
  const dir = fixture("import-past-a-branch");
  writeFileSync(join(dir, "tags.js"), "export const tags = { link: 'link', quote: 'quote' };\n");
  writeFileSync(
    join(dir, "names.js"),
    "export const said = [];\nexport const log = (s) => said.push(s);\nexport const side = () => 'side';\nexport const both = (a, b) => `${a} ${b}`;\n",
  );
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::JsObject;
use std::marker::PhantomData;

pub struct Tag(PhantomData<JsObject>);

mod tags {
    use super::Tag;

    unsafe extern "Rust" {
        #[link_name = "./tags.js#tags.link"]
        pub safe static link: &'static Tag;
        #[link_name = "./tags.js#tags.quote"]
        pub safe static quote: &'static Tag;
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "./names.js#side")]
fn side() -> &'static Tag {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./names.js#log")]
#[allow(unused_variables)]
fn log(s: &str) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./names.js#both")]
#[allow(unused_variables)]
fn both(a: Tags, b: Tags) -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Tags {
    One(&'static Tag),
    Many(Vec<&'static Tag>),
}

pub fn chosen(flag: bool) -> String {
    both(
        Tags::One(tags::link),
        if flag {
            log("side");
            Tags::One(side())
        } else {
            Tags::One(tags::quote)
        },
    )
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return both(tags.link, tmp);\n");
  expect(js).not.toContain("= tags.link");
  const lib = await import(join(dir, "lib.js"));
  const names = await import(join(dir, "names.js"));
  expect([lib.chosen(true), lib.chosen(false)]).toEqual(["link side", "link quote"]);
  expect(names.said).toEqual(["side"]);
});

// A struct `js::cast` gives, a JS object the program didn't build, whose
// properties a getter may answer: React DOM's restoreViewTransitionName,
// `let margin = style_prop.margin; if margin.is_some() { .. margin .. }`,
// reads `styleProp.margin` once, as Rust does, in its `const`. One the
// program builds is plain data, read in place (rd204, ADR 1022).
test("a field of a struct JS made is read as often as Rust reads it", async () => {
  const { js, lib } = await compiled("js-made-field-read-once", `use js::Unknown;

pub struct StyleProp {
    pub margin: Option<&'static Unknown>,
    pub padding: Option<&'static Unknown>,
}

unsafe impl js::Defined for StyleProp {}

#[cfg_attr(rust_js, rust_js::link_name = "set margin")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
#[allow(unused_variables)]
fn set_margin(this: &Unknown, value: Option<&Unknown>) {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set padding")]
#[cfg_attr(rust_js, rust_js::runs_nothing)]
#[allow(unused_variables)]
fn set_padding(this: &Unknown, value: Option<&Unknown>) {
    unreachable!()
}

fn apply(style: &Unknown, style_prop: &StyleProp) {
    let margin = style_prop.margin;
    if margin.is_some() {
        set_margin(style, margin);
    } else {
        let padding = style_prop.padding;
        set_padding(style, padding);
    }
}

pub fn restore(style: &Unknown, props: &Unknown) {
    // Safety: a style prop is an object of these fields.
    let style_prop: Option<&StyleProp> = unsafe { js::cast(js::get(props, "style")) };
    if let Some(style_prop) = style_prop {
        apply(style, style_prop);
    }
}

pub struct Kept {
    pub margin: Option<&'static Unknown>,
    pub padding: Option<&'static Unknown>,
}

// What the program stored, given back, React's state: its own object.
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.stored")]
#[cfg_attr(rust_js, rust_js::gives_back)]
fn stored() -> &'static Kept {
    unreachable!()
}

pub fn kept(style: &Unknown) {
    let kept = stored();
    let margin = kept.margin;
    if margin.is_some() {
        set_margin(style, margin);
    } else {
        let padding = kept.padding;
        set_padding(style, padding);
    }
}

struct Own {
    margin: Option<&'static Unknown>,
    padding: Option<&'static Unknown>,
}

fn own_margin(style: &Unknown, own: &Own) {
    let margin = own.margin;
    if margin.is_some() {
        set_margin(style, margin);
    } else {
        let padding = own.padding;
        set_padding(style, padding);
    }
}

pub fn own(style: &Unknown, margin: Option<&'static Unknown>) {
    own_margin(style, &Own { margin, padding: None });
}
`);
  expect(js).toContain("  const margin = styleProp.margin;\n  if (margin != null) {\n    style.margin = margin;\n");
  // The program's own struct's field is read in place, and so is one a
  // binding gives back (\`#[rust_js::gives_back]\`).
  expect(js).toContain("  if (own.margin != null) {\n    style.margin = own.margin;\n");
  expect(js).toContain("  if (kept.margin != null) {\n    style.margin = kept.margin;\n");
  let reads = 0;
  const styleProp = {
    get margin() {
      reads++;
      return reads === 1 ? "4px" : "changed";
    },
  };
  const style: Record<string, unknown> = {};
  lib.restore(style as never, { style: styleProp } as never);
  expect([style.margin, reads]).toEqual(["4px", 1]);
  lib.own(style as never, "1px" as never);
  expect(style.margin).toBe("1px");
});
