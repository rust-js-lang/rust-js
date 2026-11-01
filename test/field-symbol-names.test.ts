// A struct's field named by a symbol, `#[rust_js::name(SYMBOL)]` (ADR 1051,
// the user's decision, rd480): React's Flight reply server keys a resolved
// model's response by a symbol of its module's own, `{id,
// [RESPONSE_SYMBOL]: response}`, and its async iterable's method by
// `Symbol.asyncIterator`, `{[ASYNC_ITERATOR]() { .. }}`. Its literal's key,
// its reads and writes, `x[SYMBOL]`, a closure's method, its declaration,
// the use of the symbol rustc sees, and what's refused. The js crate's
// symbols have no native form, so these run in Node, not the corpus.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string, declarations = false) => [
  ...(declarations ? ["--declarations"] : []),
  join(dir, "lib.rs"),
  "-o",
  join(dir, "lib.js"),
  "--",
  "--extern",
  `js=${join(target, "libjs.rmeta")}`,
  "--extern",
  `react=${join(target, "libreact.rmeta")}`,
  "-L",
  target,
];

/** `rust` compiled against the js crate: its JS, its declarations, and its
 * module, loaded. */
async function compiled(name: string, rust: string): Promise<{ js: string; dts: string; lib: any }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir, true)], root, 600_000);
  expect(ran.code, ran.stderr).toBe(0);
  return {
    js: readFileSync(join(dir, "lib.js"), "utf8"),
    dts: readFileSync(join(dir, "lib.d.ts"), "utf8"),
    lib: await import(join(dir, "lib.js")),
  };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 600_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

const reason = `use std::rc::Rc;

use js::Symbol;

thread_local! {
    static RESPONSE_SYMBOL: &'static Symbol = js::symbol::new(None);
}

pub struct Response {
    pub prefix: String,
}

pub struct ModelReason {
    pub id: f64,
    #[cfg_attr(rust_js, rust_js::name(RESPONSE_SYMBOL))]
    pub response: Rc<Response>,
}
`;

// The Flight reply server's resolved model's reason: made, its key the
// module's own symbol, read and written by it, and declared by it, a
// \`unique symbol\` the declarations declare where the module holds it.
test("a field a symbol names is that symbol's key, made, read, written and declared", async () => {
  const { js, dts, lib } = await compiled(
    "field-symbol-reason",
    `${reason}
pub fn reason(id: f64, prefix: &str) -> ModelReason {
    ModelReason {
        id,
        response: Rc::new(Response {
            prefix: prefix.to_owned(),
        }),
    }
}

pub fn prefix(reason: &ModelReason) -> String {
    reason.response.prefix.clone()
}

pub fn moved(reason: &mut ModelReason, to: &str) {
    reason.response = Rc::new(Response { prefix: to.to_owned() });
}
`,
  );
  expect(js).toContain("const RESPONSE_SYMBOL = Symbol();");
  expect(js).toContain("    [RESPONSE_SYMBOL]: {\n      prefix,\n    },\n");
  expect(js).toContain("return reason[RESPONSE_SYMBOL].prefix;");
  expect(js).toContain("reason[RESPONSE_SYMBOL] = { prefix: to };");
  // Nothing of the use rustc sees: it writes no JS.
  expect(js).not.toContain("names_field");
  expect(js).not.toContain("const _");
  const made = lib.reason(1, "a");
  expect(Object.keys(made)).toEqual(["id"]);
  const [symbol] = Object.getOwnPropertySymbols(made);
  expect(typeof symbol).toBe("symbol");
  expect([made[symbol].prefix, lib.prefix(made)]).toEqual(["a", "a"]);
  lib.moved(made, "b");
  expect([lib.prefix(made), Object.getOwnPropertySymbols(made).length]).toEqual(["b", 1]);
  expect(dts).toContain("declare const RESPONSE_SYMBOL: unique symbol;");
  expect(dts).toContain("    [RESPONSE_SYMBOL]: Response;\n");
});

// The async iterable's method, a closure keyed by JS's own symbol, a method
// as a closure field is (ADR 0882), called by it; declared by its global.
test("a closure a symbol names is its method, and JS's own symbol its global", async () => {
  const { js, dts, lib } = await compiled(
    "field-symbol-iterable",
    `use std::cell::Cell;
use std::rc::Rc;

use js::Unknown;

pub struct Iterable {
    #[cfg_attr(rust_js, rust_js::name(js::symbol::async_iterator))]
    pub async_iterator: Box<dyn Fn() -> f64>,
}

unsafe impl js::Defined for Iterable {}

pub fn iterable(start: f64) -> Iterable {
    let next = Rc::new(Cell::new(start));
    Iterable {
        async_iterator: Box::new(move || {
            let at = next.get();
            next.set(at + 1.0);
            at
        }),
    }
}

pub fn step(iterable: &Iterable) -> f64 {
    (iterable.async_iterator)()
}

pub fn as_value(iterable: Iterable) -> &'static Unknown {
    js::unknown_of(iterable)
}
`,
  );
  expect(js).toContain("    [Symbol.asyncIterator]() {\n");
  expect(js).toContain("return iterable[Symbol.asyncIterator]();");
  const made = lib.asValue(lib.iterable(3));
  expect([made[Symbol.asyncIterator](), lib.step(made), Object.keys(made)]).toEqual([3, 4, []]);
  expect(dts).toContain("    [Symbol.asyncIterator]: () => number;\n");
});

// A registry's symbol, a `SymbolFor` constant, names one too; and one a
// literal at the module's load reads is made before it, as the use rustc
// sees orders them.
test("a registry's symbol names a field, made before a literal its module's load makes", async () => {
  const { js, dts, lib } = await compiled(
    "field-symbol-registry",
    `pub struct Tagged {
    #[cfg_attr(rust_js, rust_js::name(TAG))]
    pub tag: u32,
}

thread_local! {
    pub static MADE: Tagged = Tagged { tag: 7 };
}

pub const TAG: js::SymbolFor = js::SymbolFor("rust-js.test.tag");

pub const ORIGIN: Tagged = Tagged { tag: 3 };

pub fn tag() -> u32 {
    MADE.with(|made| made.tag)
}

pub fn origin() -> u32 {
    ORIGIN.tag
}
`,
  );
  expect(js).toContain('export const TAG = Symbol.for("rust-js.test.tag");');
  expect(js).toContain("export const made = { [TAG]: 7 };");
  expect(js.indexOf("export const TAG")).toBeLessThan(js.indexOf("export const made"));
  expect([lib.tag(), lib.origin()]).toEqual([7, 3]);
  expect(lib.made[Symbol.for("rust-js.test.tag")]).toBe(7);
  expect(dts).toContain("export const TAG: unique symbol;");
});

// Two fields of one struct, each its own symbol's; an update of one,
// `..reason`, its base's other spread, and a derived \`Default\`'s.
test("each field is its own symbol's, made by an update and a derived Default", async () => {
  const { js, lib } = await compiled(
    "field-symbol-update",
    `use js::Symbol;

thread_local! {
    static FIRST: &'static Symbol = js::symbol::new(Some(&"first"));
    static SECOND: &'static Symbol = js::symbol::new(Some(&"second"));
}

#[derive(Default)]
pub struct Pair {
    #[cfg_attr(rust_js, rust_js::name(FIRST))]
    pub first: u32,
    #[cfg_attr(rust_js, rust_js::name(SECOND))]
    pub second: u32,
    pub plain: u32,
}

pub fn pair(first: u32, second: u32) -> Pair {
    Pair { first, second, plain: 0 }
}

pub fn updated(pair: Pair) -> Pair {
    Pair { second: 9, ..pair }
}

pub fn default_pair() -> Pair {
    Pair::default()
}

pub fn sum(pair: &Pair) -> u32 {
    pair.first * 10 + pair.second + pair.plain
}

pub fn parts(pair: Pair) -> u32 {
    let Pair { first, second, .. } = pair;
    first * 10 + second
}

pub fn given_parts(Pair { first, second, .. }: Pair) -> u32 {
    first * 10 + second
}

pub fn from_default(first: u32) -> Pair {
    Pair {
        first,
        ..Default::default()
    }
}
`,
  );
  expect(js).toContain("[FIRST]: first,");
  expect(js).toContain("[SECOND]: second,");
  const made = lib.pair(1, 2);
  const keys = Object.getOwnPropertySymbols(made).map((symbol) => symbol.description);
  expect([keys, lib.sum(made)]).toEqual([["first", "second"], 12]);
  const updated = lib.updated(made);
  expect([lib.sum(updated), lib.sum(lib.defaultPair()), Object.getOwnPropertySymbols(lib.defaultPair()).length]).toEqual([19, 0, 2]);
  // Taken apart by a pattern, read field by field; and made of a default's.
  expect([lib.parts(lib.pair(3, 4)), lib.givenParts(lib.pair(5, 6)), lib.sum(lib.fromDefault(5))]).toEqual([34, 56, 50]);
});

// What can't name its field by a symbol is refused, saying what to write.
test("a symbol name where it can't be one is refused, saying what to write", () => {
  const symbol = "thread_local! { static S: &'static js::Symbol = js::symbol::new(None); }\n";
  const cases: [string, string, string][] = [
    ["item", `${symbol}#[cfg_attr(rust_js, rust_js::name(S))]\npub fn f() {}\n`, "names a struct's field alone"],
    ["two", `${symbol}pub struct A { #[cfg_attr(rust_js, rust_js::name(S, S))] pub a: u32 }\n`, "names its field by one symbol"],
    ["variant", `${symbol}pub enum E { V { #[cfg_attr(rust_js, rust_js::name(S))] a: u32 } }\n`, "names a struct's field, not an enum variant's"],
    ["generic", `${symbol}pub struct A<T> { #[cfg_attr(rust_js, rust_js::name(S))] pub a: T }\n`, "of no type or const parameter"],
    ["in-a-body", `${symbol}pub fn f() -> u32 { struct A { #[cfg_attr(rust_js, rust_js::name(S))] a: u32 } A { a: 1 }.a }\n`, "a struct written in a module"],
    ["derived", `${symbol}#[derive(Clone)]\npub struct A { #[cfg_attr(rust_js, rust_js::name(S))] pub a: u32 }\n`, "derives or implements `Clone`"],
    ["implemented", `${symbol}pub struct A { #[cfg_attr(rust_js, rust_js::name(S))] pub a: u32 }\nimpl PartialEq for A { fn eq(&self, o: &Self) -> bool { self.a == o.a } }\n`, "derives or implements `PartialEq`"],
    ["debug", `${symbol}#[derive(Debug)]\npub struct A { #[cfg_attr(rust_js, rust_js::name(S))] pub a: u32 }\n`, "derives or implements `Debug`"],
    ["hash", `${symbol}#[derive(Hash, PartialEq, Eq)]\npub struct A { #[cfg_attr(rust_js, rust_js::name(S))] pub a: u32 }\n`, "derives or implements"],
    ["flatten", `${symbol}pub struct I { pub b: u32 }\npub struct A { #[cfg_attr(rust_js, rust_js::flatten, rust_js::name(S))] pub a: I }\n`, "names no flattened field"],
    ["test-member", `${symbol}pub struct A { #[cfg_attr(rust_js, rust_js::test, rust_js::name(S))] pub a: Vec<u32> }\n`, "names no `#[rust_js::test]` member"],
    ["props", `${symbol}use react::{JSX, jsx};\npub struct CardProps { #[cfg_attr(rust_js, rust_js::name(S))] pub title: u32 }\npub fn Card(CardProps { title }: CardProps) -> JSX::Element { jsx! { <p>{title.to_string()}</p> } }\n`, "names no component's props field"],
    ["class", `${symbol}#[cfg_attr(rust_js, rust_js::class)]\npub struct A { #[cfg_attr(rust_js, rust_js::name(S))] pub a: u32 }\n#[cfg_attr(rust_js, rust_js::class)]\nimpl A { pub fn new() -> Self { A { a: 1 } } }\n`, "names no field of a class"],
  ];
  for (const [name, rust, said] of cases) {
    expect(refused(`field-symbol-refused-${name}`, rust), name).toContain(said);
  }
  // What it names is rustc's to resolve and check: a symbol, or rustc's
  // own error, of no attribute's.
  expect(refused("field-symbol-not-a-symbol", `pub const N: u32 = 1;\npub struct A { #[cfg_attr(rust_js, rust_js::name(N))] pub a: u32 }\n`)).toContain(
    "`u32` is no symbol a field may be named by",
  );
  expect(refused("field-symbol-unresolved", `pub struct A { #[cfg_attr(rust_js, rust_js::name(MISSING))] pub a: u32 }\n`)).toContain(
    "cannot find value `MISSING` in this scope",
  );
});
