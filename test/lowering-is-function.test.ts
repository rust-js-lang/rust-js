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
  // Each part before the last is asked whether it's there.
  expect(js).toContain('return typeof rustJsKit !== "undefined" && typeof rustJsKit.hook === "function";');
  expect(js).toContain('return typeof Symbol !== "undefined" && typeof Symbol.toStringTag === "function";');
  expect(js).toContain('export function dated() {\n  return typeof Date !== "undefined";\n}');
  const global = globalThis as Record<string, unknown>;
  const lib = await import(join(dir, "lib.js"));
  expect([lib.canPreventExtensions(), lib.called(1), lib.valued(), lib.hooked(), lib.tagged(), lib.dated()]).toEqual([true, undefined, false, false, false, true]);
  global.rustJsCallable = 3;
  global.rustJsValue = {};
  global.rustJsKit = { hook: "no" };
  expect([lib.called(1), lib.valued(), lib.hooked()]).toEqual([undefined, false, false]);
  global.rustJsCallable = (n: number) => n + 1;
  global.rustJsValue = () => {};
  global.rustJsKit = { hook: () => 0 };
  expect([lib.called(1), lib.valued(), lib.hooked()]).toEqual([2, true, true]);
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
