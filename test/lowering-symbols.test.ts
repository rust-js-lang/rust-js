// What the compiler writes of the registry's symbols, `js::SymbolFor`: a
// `const` of one, a value of any shape read as one, and a `match` of them
// (ADR 0634). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0634: React tags its elements with the registry's symbols and
// switches on them, `switch (type.$$typeof) { case REACT_MEMO_TYPE: .. }`.
// A `const` of one is `Symbol.for(key)`, and a `match` of a value read as
// one, only against them, is a `switch` of the value itself: no other
// value is one of them, a symbol of its own or none.
test("a match of the registry's symbols is a switch of the value read", async () => {
  const dir = fixture("registered-symbols");
  writeFileSync(join(dir, "lib.rs"), `use js::{SymbolFor, Unknown};

pub const REACT_FRAGMENT_TYPE: SymbolFor = SymbolFor("react.fragment");
pub const REACT_PROFILER_TYPE: SymbolFor = SymbolFor("react.profiler");
pub const REACT_PORTAL_TYPE: SymbolFor = SymbolFor("react.portal");
pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");
pub const REACT_LAZY_TYPE: SymbolFor = SymbolFor("react.lazy");

pub fn name(ty: Option<&Unknown>) -> Option<&'static str> {
    match SymbolFor::from_value(ty) {
        Some(REACT_FRAGMENT_TYPE) => Some("Fragment"),
        Some(REACT_PROFILER_TYPE) => Some("Profiler"),
        _ => None,
    }
}

pub fn tagged(ty: &Unknown) -> &'static str {
    match SymbolFor::from_value(js::get(ty, "$$typeof")) {
        Some(REACT_PORTAL_TYPE) => "Portal",
        Some(REACT_MEMO_TYPE) => "Memo",
        Some(REACT_LAZY_TYPE) => "Lazy",
        _ => "Other",
    }
}

pub fn is_lazy(ty: &Unknown) -> bool {
    matches!(SymbolFor::from_value(js::get(ty, "$$typeof")), Some(REACT_LAZY_TYPE))
}

pub fn wraps(ty: &Unknown) -> bool {
    if let Some(REACT_LAZY_TYPE | REACT_MEMO_TYPE) = SymbolFor::from_value(Some(ty)) {
        return true;
    }
    false
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export const REACT_FRAGMENT_TYPE = Symbol.for("react.fragment");\n');
  expect(js).toContain(
    'export function name(ty) {\n  switch (ty) {\n    case REACT_FRAGMENT_TYPE:\n      return "Fragment";\n    case REACT_PROFILER_TYPE:\n      return "Profiler";\n',
  );
  expect(js).toContain(
    'export function tagged(ty) {\n  switch (ty.$$typeof) {\n    case REACT_PORTAL_TYPE:\n      return "Portal";\n    case REACT_MEMO_TYPE:\n      return "Memo";\n    case REACT_LAZY_TYPE:\n      return "Lazy";\n    default:\n      return "Other";\n  }\n}',
  );
  expect(js).toContain("export function isLazy(ty) {\n  return ty.$$typeof === REACT_LAZY_TYPE;\n}");
  expect(js).toContain("  if (ty === REACT_LAZY_TYPE || ty === REACT_MEMO_TYPE) {\n");
  expect(js).not.toContain("$symbolFor");
  const lib = await import(join(dir, "lib.js"));
  const values = [Symbol.for("react.fragment"), Symbol.for("react.profiler"), Symbol("react.fragment"), "react.fragment", 5, undefined, {}];
  expect(values.map((v) => lib.name(v))).toEqual(["Fragment", "Profiler", undefined, undefined, undefined, undefined, undefined]);
  const tags = [Symbol.for("react.memo"), Symbol.for("react.lazy"), Symbol("react.lazy"), "react.lazy", undefined];
  expect(tags.map((t) => [lib.tagged({ $$typeof: t }), lib.isLazy({ $$typeof: t })])).toEqual([
    ["Memo", false],
    ["Lazy", true],
    ["Other", false],
    ["Other", false],
    ["Other", false],
  ]);
  expect([Symbol.for("react.memo"), Symbol.for("react.lazy"), Symbol("react.memo"), 1].map((v) => lib.wraps(v))).toEqual([true, true, false, false]);
});

// ADR 0634: a registered symbol's value is the symbol, `Symbol.for(key)`
// where it's made, its key `Symbol.keyFor(symbol)`, and two equal by
// `===`, as their keys are. A value read as one whose `Some` or `None` the
// program tells, `None =>`, a binding, is the value where it's one,
// `$symbolFor(value)`, else `undefined`.
test("the registry's symbol is Symbol.for its key, and a read of one it tells is checked", async () => {
  const dir = fixture("registered-symbol-values");
  writeFileSync(join(dir, "lib.rs"), `use js::{SymbolFor, Unknown};

pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");

pub fn made(key: &str) -> SymbolFor<'_> {
    SymbolFor(key)
}

pub fn all(keys: &[&'static str]) -> Vec<SymbolFor<'static>> {
    keys.iter().copied().map(SymbolFor).collect()
}

pub fn key(symbol: SymbolFor<'static>) -> &'static str {
    symbol.0
}

pub fn same(a: SymbolFor, b: SymbolFor) -> bool {
    a == b
}

pub fn key_of(value: Option<&Unknown>) -> Option<&'static str> {
    SymbolFor::from_value(value).map(|symbol| symbol.0)
}

pub fn kind(value: Option<&Unknown>) -> &'static str {
    match SymbolFor::from_value(value) {
        Some(REACT_MEMO_TYPE) => "memo",
        None => "none",
        Some(_) => "other",
    }
}

pub fn named(value: Option<&Unknown>) -> &'static str {
    match SymbolFor::from_value(value) {
        Some(REACT_MEMO_TYPE) => "memo",
        Some(symbol) => symbol.0,
        _ => "none",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function made(key) {\n  return Symbol.for(key);\n}");
  expect(js).toContain("export function key(symbol) {\n  return Symbol.keyFor(symbol);\n}");
  expect(js).toContain("export function same(a, b) {\n  return a === b;\n}");
  expect(js).toContain("$symbolFor(value)");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.all(["a", "b"])).toEqual([Symbol.for("a"), Symbol.for("b")]);
  expect([lib.made("app") === Symbol.for("app"), lib.key(Symbol.for("app")), lib.same(Symbol.for("a"), lib.made("a")), lib.same(Symbol.for("a"), Symbol.for("b"))]).toEqual([
    true,
    "app",
    true,
    false,
  ]);
  const values = [Symbol.for("react.memo"), Symbol.for("react.lazy"), Symbol("react.memo"), "react.memo", 5, undefined];
  expect(values.map((v) => lib.keyOf(v))).toEqual(["react.memo", "react.lazy", undefined, undefined, undefined, undefined]);
  expect(values.map((v) => lib.kind(v))).toEqual(["memo", "other", "none", "none", "none", "none"]);
  expect(values.map((v) => lib.named(v))).toEqual(["memo", "react.lazy", "none", "none", "none", "none"]);
});

// ADR 0771: `from_value(x) == Some(C)` says what `matches!(from_value(x),
// Some(C))` does, and is what it is, `x === C`, `!==` of `!=`, `C` by its
// name; a value read as one and kept is still checked, and its `==` of a
// `Some` strict, but one a `let` holds only to compare is the value. An
// object that turns into the symbol isn't it.
test("an == of a value read as the registry's symbol and a Some is the matches! form", async () => {
  const dir = fixture("registered-symbols-equal");
  writeFileSync(join(dir, "lib.rs"), `use js::{SymbolFor, Unknown};

pub const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");
pub const REACT_LAZY_TYPE: SymbolFor = SymbolFor("react.lazy");

pub fn eq(x: Option<&Unknown>) -> bool {
    SymbolFor::from_value(x) == Some(REACT_MEMO_TYPE)
}

pub fn ne(x: Option<&Unknown>) -> bool {
    SymbolFor::from_value(x) != Some(REACT_MEMO_TYPE)
}

pub fn flipped(x: Option<&Unknown>) -> bool {
    Some(REACT_LAZY_TYPE) == SymbolFor::from_value(x)
}

pub fn matched(x: Option<&Unknown>) -> bool {
    matches!(SymbolFor::from_value(x), Some(REACT_MEMO_TYPE))
}

pub fn literal(x: Option<&Unknown>) -> bool {
    SymbolFor::from_value(x) == Some(SymbolFor("react.lazy"))
}

pub fn kept(x: Option<&Unknown>) -> Option<SymbolFor<'static>> {
    let s = SymbolFor::from_value(x);
    if s == Some(REACT_MEMO_TYPE) {
        return None;
    }
    s
}

pub fn tagged(x: Option<&Unknown>) -> u32 {
    let tag = SymbolFor::from_value(x);
    if tag == Some(REACT_MEMO_TYPE) {
        1
    } else if tag != Some(REACT_LAZY_TYPE) {
        2
    } else {
        3
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A `let` of the read that's only compared, React's mountLazyComponent's
  // `$$typeof`: the value, as of the read in place (ADR 0771, amended).
  const tagged = js.slice(js.indexOf("export function tagged"));
  expect(tagged).toContain("tag === REACT_MEMO_TYPE");
  expect(tagged).toContain("tag !== REACT_LAZY_TYPE");
  expect(tagged).not.toContain("$symbolFor");
  expect(js).toContain("export function eq(x) {\n  return x === REACT_MEMO_TYPE;\n}");
  expect(js).toContain("export function ne(x) {\n  return x !== REACT_MEMO_TYPE;\n}");
  expect(js).toContain("export function flipped(x) {\n  return REACT_LAZY_TYPE === x;\n}");
  expect(js).toContain("export function matched(x) {\n  return x === REACT_MEMO_TYPE;\n}");
  expect(js).toContain('export function literal(x) {\n  return x === Symbol.for("react.lazy");\n}');
  expect(js).toContain("  const s = $symbolFor(x);\n  if (s === REACT_MEMO_TYPE) {\n");
  const lib = await import(join(dir, "lib.js"));
  const memo = Symbol.for("react.memo");
  const values = [memo, Symbol.for("react.lazy"), Symbol("react.memo"), "react.memo", { [Symbol.toPrimitive]: () => memo }, undefined];
  expect(values.map(lib.tagged)).toEqual([1, 3, 2, 2, 2, 2]);
  expect(values.map((v) => [lib.eq(v), lib.ne(v), lib.flipped(v), lib.matched(v), lib.literal(v), lib.kept(v)])).toEqual([
    [true, false, false, true, false, undefined],
    [false, true, true, false, true, Symbol.for("react.lazy")],
    [false, true, false, false, false, undefined],
    [false, true, false, false, false, undefined],
    [false, true, false, false, false, undefined],
    [false, true, false, false, false, undefined],
  ]);
});

// ADR 0634: the registry's symbol is made one way, `js::SymbolFor(key)`;
// `js::symbol::r#for` is gone, an error that says so.
// ADR 0634, amended: a guarded arm's pattern, which MIR reads the key of
// for its guard, leaves no `Symbol.keyFor(ty);` of what nothing reads: the
// registry's symbol and its key do nothing, of a value its type says is one.
test("a guarded arm of a registered symbol reads no key it doesn't use", async () => {
  const dir = fixture("registered-symbols-guarded");
  writeFileSync(join(dir, "lib.rs"), `use js::{SymbolFor, Unknown};

pub const A: SymbolFor = SymbolFor("a");
pub const B: SymbolFor = SymbolFor("b");

pub fn name(ty: Option<&Unknown>, flag: bool) -> u32 {
    match SymbolFor::from_value(ty) {
        Some(A) => 1,
        Some(B) if flag => 2,
        _ => 3,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("keyFor");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.name(Symbol.for("a"), false), lib.name(Symbol.for("b"), true), lib.name(Symbol.for("b"), false), lib.name(1, true)]).toEqual([1, 2, 3, 3]);
});

test("js::symbol::r#for is gone, an error naming js::SymbolFor", () => {
  const dir = fixture("symbol-for-gone");
  writeFileSync(join(dir, "lib.rs"), `pub fn app() -> &'static js::Symbol {
    js::symbol::r#for("app")
}
`);
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()], root, 60_000);
  expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining("`js::symbol::r#for` is gone: the registry's symbol of a key is `js::SymbolFor(key)`")]);
});
