// ADR 0942: whether a binding's global is a function is `js::is_function`'s,
// `typeof X === "function"`, of the binding its type names, a static, or a
// path of bindings, as `js::is_defined` asks whether it's there (ADRs 0597,
// 0624, 0645). React asks `typeof Object.preventExtensions === 'function'`
// and `typeof setTimeout === 'function'`. A path's global that's
// ECMAScript's own, every host's, isn't asked of, as `Math` isn't (ADR 0887).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("js::is_function is whether a binding's global is a function", async () => {
  const dir = fixture("is-function");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsCallable")]
pub fn callable(n: f64) -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsKit.hook")]
pub fn kit_hook(n: f64) -> f64 {
    unreachable!()
}

unsafe extern "Rust" {
    #[link_name = "rustJsValue"]
    pub safe static VALUE: &'static js::JsObject;
}

/// React's \`typeof Object.preventExtensions === 'function'\`.
pub fn can_prevent_extensions() -> bool {
    js::is_function(js::object::prevent_extensions::<js::Unknown>)
}

pub fn has_prevent_extensions() -> bool {
    js::is_defined(js::object::prevent_extensions::<js::Unknown>)
}

pub fn called(n: f64) -> Option<f64> {
    js::is_function(callable).then(|| callable(n))
}

pub fn valued() -> bool {
    js::is_function(VALUE)
}

pub fn hooked() -> bool {
    let hook = kit_hook;
    js::is_function(hook)
}

// Where the host may lack the global, the program asks so first.
pub fn guarded() -> bool {
    js::is_defined(kit_hook) && js::is_function(kit_hook)
}

pub fn tagged() -> bool {
    js::is_function(js::symbol::to_string_tag)
}

// ECMAScript's own global, asked as itself: the program asked of it.
pub fn dated() -> bool {
    js::is_defined(js::date::new)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function canPreventExtensions() {\n  return typeof Object.preventExtensions === "function";\n}');
  expect(js).toContain('export function hasPreventExtensions() {\n  return typeof Object.preventExtensions !== "undefined";\n}');
  expect(js).toContain('typeof rustJsCallable === "function"');
  expect(js).toContain('return typeof rustJsValue === "function";');
  // Only the member is asked, as upstream's `typeof performance.getEntriesByType
  // === 'function'` is: it throws where the host hasn't the global before it.
  expect(js).toContain('return typeof rustJsKit.hook === "function";');
  expect(js).toContain('typeof rustJsKit !== "undefined" &&\n    typeof rustJsKit.hook !== "undefined" &&\n    typeof rustJsKit.hook === "function"\n');
  expect(js).toContain('return typeof Symbol.toStringTag === "function";');
  expect(js).toContain('export function dated() {\n  return typeof Date !== "undefined";\n}');
  const global = globalThis as Record<string, unknown>;
  const lib = await import(join(dir, "lib.js"));
  expect([lib.canPreventExtensions(), lib.called(1), lib.valued(), lib.tagged(), lib.dated()]).toEqual([true, undefined, false, false, true]);
  expect(() => lib.hooked()).toThrow(ReferenceError);
  expect(lib.guarded()).toBe(false);
  global.rustJsCallable = 3;
  global.rustJsValue = {};
  global.rustJsKit = { hook: "no" };
  expect([lib.called(1), lib.valued(), lib.hooked()]).toEqual([undefined, false, false]);
  global.rustJsCallable = (n: number) => n + 1;
  global.rustJsValue = () => {};
  global.rustJsKit = { hook: () => 0 };
  expect([lib.called(1), lib.valued(), lib.hooked(), lib.guarded()]).toEqual([2, true, true, true]);
  const prevent = Object.preventExtensions;
  try {
    (Object as unknown as Record<string, unknown>).preventExtensions = 1;
    expect([lib.canPreventExtensions(), lib.hasPreventExtensions()]).toEqual([false, true]);
  } finally {
    Object.preventExtensions = prevent;
  }
});

// What isn't a global's binding is refused, naming `js::is_function`.
test("js::is_function of what isn't a global's binding is refused", () => {
  const dir = fixture("is-function-refused");
  const refused = (code: string) => {
    writeFileSync(join(dir, "lib.rs"), code);
    const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 60_000);
    return [p.code === 0, p.stderr];
  };
  const message = expect.stringContaining("`js::is_function` of what isn't a JS global's binding");
  expect(refused("pub fn ask() -> bool {\n    js::is_function(|| 1)\n}\n")).toEqual([false, message]);
  expect(refused("fn helper() {}\n\npub fn ask() -> bool {\n    helper();\n    js::is_function(helper)\n}\n")).toEqual([false, message]);
  expect(refused("pub fn ask() -> bool {\n    js::is_function(&js::symbol::iterator)\n}\n")).toEqual([
    false,
    expect.stringContaining("given the static itself, `js::is_function(webapi::performance)`"),
  ]);
});

// ADR 0964: React's test selectors make their selector types symbols where
// the host has `Symbol.for`, `if (typeof Symbol === 'function' &&
// Symbol.for) { const symbolFor = Symbol.for; .. symbolFor('selector.component') }`.
// `js::SymbolFor` itself is `Symbol.for`, asked as a binding is, held and
// called as it is; a `typeof` an earlier test of the `&&` says isn't asked
// again.
test("js::SymbolFor is Symbol.for, asked whether it's a function and called where it's held", async () => {
  const dir = fixture("is-function-symbol-for");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::{SymbolFor, Unknown};

thread_local! {
    static COMPONENT_TYPE: Cell<&'static Unknown> = Cell::new(js::unknown_of(0b000));
}

js::on_load! {
    if js::is_function(js::symbol::new) && js::is_function(SymbolFor) {
        let symbol_for = SymbolFor;
        COMPONENT_TYPE.set(js::unknown_of(symbol_for("selector.component")));
    }
}

pub fn component_type() -> &'static Unknown {
    COMPONENT_TYPE.get()
}

pub fn has_symbol_for() -> bool {
    js::is_function(SymbolFor)
}

pub fn defines_symbol_for() -> bool {
    js::is_defined(SymbolFor)
}

pub fn symbol_defines_for() -> bool {
    js::is_function(js::symbol::new) && js::is_defined(SymbolFor)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('if (typeof Symbol === "function" && typeof Symbol.for === "function") {\n  const symbolFor = Symbol.for;\n  COMPONENT_TYPE = symbolFor("selector.component");\n}');
  expect(js).toContain('export function hasSymbolFor() {\n  return typeof Symbol.for === "function";\n}');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.componentType()).toBe(Symbol.for("selector.component"));
  expect(js).toContain('export function definesSymbolFor() {\n  return typeof Symbol !== "undefined" && typeof Symbol.for !== "undefined";\n}');
  // What `typeof Symbol === "function"` says, a later `typeof Symbol !==
  // "undefined"` needn't (ADR 0964).
  expect(js).toContain('export function symbolDefinesFor() {\n  return typeof Symbol === "function" && typeof Symbol.for !== "undefined";\n}');
  expect([lib.hasSymbolFor(), lib.definesSymbolFor(), lib.symbolDefinesFor()]).toEqual([true, true, true]);
});

// React's ReactFiberPerformanceTrack: `const supportsUserTiming =
// enableProfilerTimer && typeof console !== 'undefined' && // ..
// typeof console.timeStamp === 'function'`. A thread-local `bool`'s value
// a `with` lends is a `bool`, as a `bool` constant's read is, so the chain
// is its `&&`s, no `!!(..)` that kept ADR 0964's later `typeof console`;
// and a comment before `js::is_function(..)` is before its test, one
// before `js::is_defined(..)` before the first of its tests.
test("a thread-local bool of another and is_function's chain read as written", async () => {
  const dir = fixture("is-function-chain");
  writeFileSync(join(dir, "lib.rs"), `thread_local! {
    pub static ENABLE_PROFILER_TIMER: bool = true;
}

#[cfg_attr(rust_js, rust_js::link_name = "console.timeStamp")]
pub fn time_stamp(_label: &str) {
    unreachable!()
}

thread_local! {
    static SUPPORTS_USER_TIMING: bool = ENABLE_PROFILER_TIMER.with(|&enabled| enabled)
        // Some hosts have no console
        && js::is_defined(time_stamp)
        // In DEV we also rely on performance.measure
        && js::is_function(time_stamp);
}

pub fn supports() -> bool {
    SUPPORTS_USER_TIMING.with(|&supports| supports)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`const SUPPORTS_USER_TIMING =
  ENABLE_PROFILER_TIMER &&
  // Some hosts have no console
  typeof console !== "undefined" &&
  typeof console.timeStamp !== "undefined" &&
  // In DEV we also rely on performance.measure
  typeof console.timeStamp === "function";`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.supports()).toBe(true);
});

// React's SchedulerMock runs a task's callback only `if (typeof callback ===
// 'function')`, and its test "gracefully handles scheduled tasks that are
// not a function" schedules `null`, `undefined`, `{}` and `42`. A value of
// a function's type that JS gives may be anything, so `js::is_function` of
// one asks it, `typeof callback === "function"`: a `fn`, a `dyn Fn` lent
// or boxed, a type parameter's `impl Fn`, a transparent struct of one, or
// an `Option` of any of them. A closure the program makes, or one of its
// functions, is always one, and stays refused.
test("js::is_function of a value of a function's type is whether JS gave a function", async () => {
  const dir = fixture("is-function-value");
  writeFileSync(join(dir, "lib.rs"), `#[repr(transparent)]
pub struct Callback(Box<dyn Fn(bool) -> Option<f64>>);

/// SchedulerMock's workLoop: \`if (typeof callback === 'function')\`.
pub fn run(callback: Option<Callback>, did_timeout: bool) -> Option<f64> {
    if let Some(Callback(callback)) = callback
        && js::is_function(&callback)
    {
        return callback(did_timeout);
    }
    None
}

pub fn pointer(f: fn(f64) -> f64) -> bool {
    js::is_function(f)
}

pub fn lent(f: &dyn Fn()) -> bool {
    js::is_function(f)
}

pub fn generic(f: impl Fn() + Copy) -> bool {
    js::is_function(f)
}

pub fn maybe(callback: &Option<Callback>) -> bool {
    js::is_function(callback)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('if (typeof callback === "function") {\n    return callback(didTimeout);\n  }');
  expect(js).toContain('export function pointer(f) {\n  return typeof f === "function";\n}');
  expect(js).toContain('export function lent(f) {\n  return typeof f === "function";\n}');
  expect(js).toContain('export function generic(f) {\n  return typeof f === "function";\n}');
  expect(js).toContain('export function maybe(callback) {\n  return typeof callback === "function";\n}');
  const lib = await import(join(dir, "lib.js"));
  const given = [null, undefined, {}, 42, "f", (t: boolean) => (t ? 1 : 2)];
  expect(given.map((v) => lib.run(v, true))).toEqual([undefined, undefined, undefined, undefined, undefined, 1]);
  expect(given.map((v) => [lib.pointer(v), lib.lent(v), lib.generic(v), lib.maybe(v)])).toEqual(
    given.map((v) => Array(4).fill(typeof v === "function")),
  );
});

// React's ReactDOMSelection feature-tests a method of a value it's given,
// `if (!selection.extend && start > end)`, as IE 11 has no `extend`. A
// path's first step may be a value, its next a method of it,
// `js::is_function((selection, Selection::extend))`: `typeof
// selection.extend === "function"`, which the user accepted for
// upstream's truth test (ADR 0942, amended). `js::is_defined` of one asks
// each member, never the value, which is no global.
test("js::is_function of a path from a value asks its member", async () => {
  const withWeb = ["--", "--edition", "2024", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("is-function-value-path");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{Node, Selection};

pub fn set_offsets(selection: &Selection, node: &Node, start: f64, end: f64) -> bool {
    if !js::is_function((selection, Selection::extend)) && start > end {
        return false;
    }
    selection.extend(node, Some(end));
    true
}

pub fn has_extend(selection: &Selection) -> bool {
    js::is_defined((selection, Selection::extend))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('if (typeof selection.extend !== "function" && start > end) {');
  expect(js).toContain('return typeof selection.extend !== "undefined";');
  const lib = await import(join(dir, "lib.js"));
  const extended: unknown[] = [];
  const modern = { extend: (node: unknown, offset: unknown) => void extended.push([node, offset]) };
  expect([lib.setOffsets(modern, "n", 5, 2), lib.setOffsets({}, "n", 5, 2), lib.hasExtend(modern), lib.hasExtend({})]).toEqual([true, false, true, false]);
  expect(extended).toEqual([["n", 2]]);
});
