// An `Option`'s fallback as JS writes it, `||`: a filter by truth then a
// fallback, React's typeName as upstream reads it, and a let then its
// fallback where it's falsy. Split from lowering-options.test.ts by
// concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0635: an `Option` filtered by the truth of what it holds, then a
// fallback, is JS's `||` of values, as React writes its names: `x || y`
// gives `x` where it's truthy, else `y`, and the filtered `Option` gives `x`
// where `js::truthy(x)`, else what's after. A predicate that's anything
// else is its own test.
test("a filter by truth then a fallback is JS's `||`", async () => {
  const dir = fixture("truthy-or");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

#[rust_js::nullable]
pub fn display_name(t: &Unknown) -> Option<&'static Unknown> {
    js::get(t, "displayName")
        .filter(|v| js::truthy(Some(*v)))
        .or_else(|| js::get(t, "name"))
        .filter(|v| js::truthy(Some(*v)))
}

pub fn function_name(inner: &Unknown) -> String {
    js::get(inner, "displayName")
        .filter(|v| js::truthy(Some(*v)))
        .or_else(|| js::get(inner, "name"))
        .filter(|v| js::truthy(Some(*v)))
        .map(|v| unsafe { js::cast::<String>(Some(v)) })
        .unwrap_or_default()
}

pub fn component_name(t: Option<&Unknown>) -> Option<String> {
    t.and_then(|t| js::get(t, "displayName")).map(|v| unsafe { js::cast::<String>(Some(v)) })
}

pub fn memo_name(t: &Unknown) -> String {
    component_name(js::get(t, "type"))
        .filter(|name| js::truthy(Some(name)))
        .unwrap_or_else(|| "Memo".to_string())
}

pub fn either(a: Option<&'static Unknown>, b: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v))).or(b)
}

pub fn either_or(
    a: Option<&'static Unknown>,
    b: Option<&'static Unknown>,
    c: Option<&'static Unknown>,
) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v))).or(b).or(c)
}

pub fn given(a: Option<&'static Unknown>, b: &'static Unknown) -> &'static Unknown {
    a.filter(|&v| js::truthy(Some(v))).unwrap_or(b)
}

pub fn first(
    a: Option<&'static Unknown>,
    b: Option<&'static Unknown>,
    c: Option<&'static Unknown>,
) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v)))
        .or_else(|| b.filter(|v| js::truthy(Some(*v))))
        .or_else(|| c)
}

pub fn falsy_or(a: Option<&'static Unknown>, b: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| !js::truthy(Some(*v))).or(b)
}

#[rust_js::nullable]
pub fn kept(a: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(Some(*v)))
}

pub fn shout(a: Option<&'static str>) -> Option<String> {
    a.filter(|s| js::truthy(Some(*s))).map(|s| s.to_uppercase())
}

pub fn named(a: Option<&'static Unknown>) -> Option<&'static Unknown> {
    a.filter(|v| js::truthy(js::get(v, "name")))
}

#[rust_js::nullable]
pub fn label(a: Option<&'static str>) -> Option<&'static str> {
    a.filter(|s| js::truthy(Some(*s))).or(Some("Memo"))
}

fn logged(log: &mut Vec<u32>, n: u32) -> Option<u32> {
    log.push(n);
    if n > 1 { Some(n) } else { None }
}

pub fn ordered() -> Vec<u32> {
    let mut log = Vec::new();
    let found = logged(&mut log, 1).or(logged(&mut log, 2));
    log.push(found.unwrap_or(0));
    log
}

// React's typeName: the chain held, then given through a cast, the
// identity, then its fallback.
pub fn type_name(value: &Unknown) -> String {
    let r#type = js::get(value, "tag")
        .filter(|v| js::truthy(Some(*v)))
        .or_else(|| js::get(value, "name"))
        .filter(|v| js::truthy(Some(*v)));
    unsafe { js::cast::<Option<String>>(r#type) }.unwrap_or_else(|| "Object".to_string())
}

// One read again after its fallback keeps its \`undefined\` there.
pub fn tag_shown(value: &Unknown) -> (String, bool) {
    let tag = js::get(value, "tag").filter(|v| js::truthy(Some(*v)));
    let shown = unsafe { js::cast::<Option<String>>(tag) }.unwrap_or_else(|| "Object".to_string());
    (shown, tag.is_some())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function displayName(t) {\n  return t.displayName || t.name || null;\n}");
  expect(js).toContain('export function functionName(inner) {\n  return inner.displayName || inner.name || "";\n}');
  expect(js).toContain('export function memoName(t) {\n  return componentName(t.type) || "Memo";\n}');
  expect(js).toContain("export function either(a, b) {\n  return a || b;\n}");
  expect(js).toContain("export function eitherOr(a, b, c) {\n  return (a || b) ?? c;\n}");
  expect(js).toContain("export function given(a, b) {\n  return a || b;\n}");
  expect(js).toContain("export function first(a, b, c) {\n  return a || b || c;\n}");
  expect(js).toContain("export function kept(a) {\n  return a || null;\n}");
  expect(js).toContain('export function label(a) {\n  return a || "Memo";\n}');
  expect(js).toContain('export function typeName(value) {\n  const type = value.tag || value.name || "Object";\n  return type;\n}');
  expect(js).toContain("export function shout(a) {\n  return (a ? $toUppercase(a) : undefined) ?? undefined;\n}");
  // A predicate that isn't the truth of what it holds is its own test.
  expect(js).toContain("export function falsyOr(a, b) {\n  return a != null && !a ? a : b;\n}");
  expect(js).toContain("export function named(a) {\n  return a != null && a.name ? a : undefined;\n}");
  const lib = await import(join(dir, "lib.js"));
  const object = { name: "object" };
  const values = ["", 0, undefined, null, "n", object];
  for (const x of values) {
    for (const y of values) {
      expect(lib.displayName({ displayName: x, name: y })).toEqual(x || y || null);
      expect(lib.functionName({ displayName: x, name: y })).toEqual(x || y || "");
      expect(lib.memoName({ type: { displayName: x } })).toEqual(x || "Memo");
      expect(lib.typeName({ tag: x, name: y })).toEqual(x || y || "Object");
      expect(lib.tagShown({ tag: x })).toEqual([x || "Object", !!x]);
      expect(lib.either(x ?? undefined, y ?? undefined)).toEqual(x || (y ?? undefined));
      expect(lib.eitherOr(x ?? undefined, y ?? undefined, object)).toEqual(x || (y ?? object));
      expect(lib.given(x ?? undefined, y)).toEqual(x || y);
      expect(lib.first(x ?? undefined, y ?? undefined, object)).toEqual(x || y || object);
      expect(lib.falsyOr(x ?? undefined, y ?? undefined)).toEqual(x != null && !x ? x : (y ?? undefined));
      expect(lib.kept(x ?? undefined)).toEqual(x || null);
      expect(lib.named(x ?? undefined)).toEqual(x != null && (x as { name?: string }).name ? x : undefined);
    }
  }
  expect(lib.memoName({})).toEqual("Memo");
  expect(["", "ab", undefined].map(lib.shout)).toEqual([undefined, "AB", undefined]);
  expect([lib.label(""), lib.label("x"), lib.label(undefined)]).toEqual(["Memo", "x", "Memo"]);
  // `or`'s `b` is worked out after its `Option`, as Rust does both first.
  expect(lib.ordered()).toEqual([1, 2, 2]);
});

// ADR 0647: React's typeName, as upstream writes it: `js::is_defined` of a
// static reads nothing before it tests, so a host without `Symbol` gets
// the constructor's name; a `filter` by a test that ignores the value, then
// an `and_then`, kept by its truth, is `c && value?.[k]`; and `flatten()` of
// `bool::then` of an `Option` is `c ? x : undefined`.
test("React's typeName reads as upstream and asks before it reads", async () => {
  const dir = fixture("type-name");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

pub fn type_name(value: Option<&Unknown>) -> String {
    // toStringTag is needed for namespaced types like Temporal.Instant
    let has_to_string_tag = js::is_defined(js::symbol::to_string_tag);
    let r#type = value
        .filter(|_| has_to_string_tag)
        .and_then(|value| js::get(value, js::symbol::to_string_tag))
        .filter(|tag| js::truthy(Some(*tag)))
        .or_else(|| value.and_then(|value| js::get(value, "constructor")).and_then(|c| js::get(c, "name")))
        .filter(|name| js::truthy(Some(*name)));
    unsafe { js::cast::<Option<String>>(r#type) }.unwrap_or_else(|| "Object".to_string())
}

pub fn named(value: Option<&Unknown>, c: bool) -> Option<&'static Unknown> {
    value.filter(|_| c).and_then(|value| js::get(value, "name"))
}

pub fn tag(value: Option<&Unknown>) -> Option<&'static Unknown> {
    value.and_then(|value| js::get(value, js::symbol::to_string_tag))
}

pub fn flattened(value: Option<&Unknown>, c: bool) -> Option<&'static Unknown> {
    c.then(|| value.and_then(|value| js::get(value, "name"))).flatten()
}

fn counted(calls: &std::cell::Cell<u32>) -> bool {
    calls.set(calls.get() + 1);
    true
}

// A test that does something runs only where there's a value, as Rust's.
pub fn counted_name(value: Option<&Unknown>) -> (String, u32) {
    let calls = std::cell::Cell::new(0);
    let name = value
        .filter(|_| counted(&calls))
        .and_then(|value| js::get(value, "name"))
        .filter(|name| js::truthy(Some(*name)));
    (unsafe { js::cast::<Option<String>>(name) }.unwrap_or_else(|| "none".to_string()), calls.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  const hasToStringTag = typeof Symbol !== "undefined" && typeof Symbol.toStringTag !== "undefined";\n  const type =\n    (hasToStringTag && value?.[Symbol.toStringTag]) || value?.constructor?.name || "Object";\n  return type;\n');
  expect(js).not.toContain("const toStringTag");
  // \`and_then\` of what \`value?.constructor\` gives is one chain: the
  // same path tested and read (\`same_path\` of an optional member).
  expect(js).toContain("value?.constructor?.name");
  expect(js).toContain("return value != null && c ? value.name : undefined;");
  expect(js).toContain("return value?.[Symbol.toStringTag];");
  expect(js).toContain("return c ? value?.name : undefined;");
  const lib = await import(join(dir, "lib.js"));
  class Instant {
    get [Symbol.toStringTag]() {
      return "Temporal.Instant";
    }
  }
  expect([lib.typeName(new Instant()), lib.typeName(new Map()), lib.typeName({}), lib.typeName(Object.create(null)), lib.typeName(undefined)]).toEqual(["Temporal.Instant", "Map", "Object", "Object", "Object"]);
  expect([lib.named({ name: "n" }, true), lib.named({ name: "n" }, false), lib.named(undefined, true)]).toEqual(["n", undefined, undefined]);
  expect([lib.flattened({ name: "n" }, true), lib.flattened({ name: "n" }, false), lib.flattened(undefined, true)]).toEqual(["n", undefined, undefined]);
  expect([lib.countedName({ name: "n" }), lib.countedName({ name: "" }), lib.countedName(undefined)]).toEqual([["n", 1], ["none", 1], ["none", 0]]);
  // A host with no `Symbol`: asked first, never read.
  const symbol = globalThis.Symbol;
  let named: unknown;
  try {
    // @ts-expect-error: a host without it.
    delete globalThis.Symbol;
    named = lib.typeName(new Map());
  } finally {
    globalThis.Symbol = symbol;
  }
  expect(named).toBe("Map");
});

// ADR 0651: a `let`, then its fallback where JS's `!x` finds it falsy, is
// one `||`, as React's DefaultPrepareStackTraceV8 writes `const name =
// error.name || 'Error';`. Its JS was
//
//   let name = error.name;
//   if (!name) {
//     name = 'Error';
//   }
//
// Its fallback where it's none, `x ??= c`, is `??`. Not a `c` that reads
// `x`, nor a test that isn't the truth, a number's `=== 0`, which NaN
// fails, a string's `=== "x"`; and a `let` still, set after.
test("a let then its fallback where it's falsy is one ||", async () => {
  const dir = fixture("declared-fallback");
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;

pub fn heading(error: &JsError) -> String {
    let mut name = error.name();
    if name.is_empty() {
        name = "Error".to_string();
    }
    let message = error.message();
    name + ": " + &message
}

pub fn shouted(s: &str) -> String {
    let mut label = s.to_string();
    if label.is_empty() {
        label = "none".to_string();
    }
    label.push('!');
    label
}

pub fn asked(t: &str) -> String {
    let mut s = t.to_string();
    if s.is_empty() {
        s = format!("{s}?");
    }
    s
}

pub fn nonzero(x: f64) -> f64 {
    let mut f = x;
    if f == 0.0 {
        f = 1.0;
    }
    f
}

pub fn renamed(t: &str) -> String {
    let mut name = t.to_string();
    if name == "x" {
        name = "y".to_string();
    }
    name
}

pub fn given(o: Option<u32>) -> Option<u32> {
    let mut v = o;
    if v.is_none() {
        v = Some(3);
    }
    v
}

pub fn either(o: Option<String>) -> String {
    let mut n = o.unwrap_or_default();
    if n.is_empty() {
        n = "x".to_string();
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function heading(error) {
  const name = error.name || "Error";
  const message = error.message;
  return name + ": " + message;
}`);
  expect(js).toContain('  let label = s || "none";\n  label += "!";\n');
  expect(js).toContain('  let s = t;\n  if (s === "") {\n    s = `${s}?`;\n  }\n');
  expect(js).toContain("  let f = x;\n  if (f === 0) {\n    f = 1;\n  }\n");
  expect(js).toContain('  let name = t;\n  if (name === "x") {\n');
  expect(js).toContain("  const v = o ?? 3;\n");
  expect(js).toContain('  const n = (o ?? "") || "x";\n');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.heading({ name: "", message: "m" }), lib.heading({ name: "TypeError", message: "m" })]).toEqual([
    "Error: m",
    "TypeError: m",
  ]);
  expect([lib.shouted(""), lib.shouted("a"), lib.asked(""), lib.asked("a")]).toEqual(["none!", "a!", "?", "a"]);
  expect([lib.nonzero(0), lib.nonzero(Number.NaN), lib.nonzero(2)]).toEqual([1, Number.NaN, 2]);
  expect([lib.renamed("x"), lib.renamed("")]).toEqual(["y", ""]);
  expect([lib.given(undefined), lib.given(0), lib.either(undefined), lib.either(""), lib.either("a")]).toEqual([
    3,
    0,
    "x",
    "x",
    "a",
  ]);
});
