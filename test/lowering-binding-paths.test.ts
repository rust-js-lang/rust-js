// A binding's path is JS's (ADR 1033): the global it starts from, then each
// property by its name, `console.log`, or by a quoted key, `window['$RT']`,
// which a minifier keeps, as React's Fizz runtime writes the globals the
// server's inline scripts set.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const externs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a quoted key of a binding's path is read and written by its key", async () => {
  const dir = fixture("quoted-path");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "window['$RT']"]
    safe static RT: Option<&'static js::Unknown>;
    #[link_name = "window['$RN']"]
    safe static RN: f64;
}

#[cfg_attr(rust_js, rust_js::link_name = "set window['$RT']")]
fn set_rt(time: f64) {
    let _ = time;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get window['$RT'].toFixed")]
fn rt_to_fixed() -> Option<&'static js::Unknown> {
    unreachable!()
}

pub fn mark(now: f64) -> bool {
    set_rt(now);
    js::is_number(RN) && js::is_defined(RT) && rt_to_fixed().is_some()
}

pub fn shell_time() -> Option<&'static js::Unknown> {
    RT
}

pub fn shadowed(window: f64) -> f64 {
    set_rt(window + 1.0);
    window
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('window["$RT"] = now;');
  expect(js).toContain('typeof window["$RN"] === "number"');
  expect(js).toContain('window["$RT"].toFixed != null');
  expect(js).toContain('return window["$RT"];');
  const lib = await import(join(dir, "lib.js"));
  const g = globalThis as any;
  g.window = { $RN: 1 };
  try {
    expect([lib.mark(5), lib.shellTime(), g.window.$RT]).toEqual([true, 5, 5]);
    expect([lib.shadowed(2), g.window.$RT]).toEqual([2, 3]);
  } finally {
    delete g.window;
  }
});

test("a binding's path that isn't JS's is refused, naming the quoted key", () => {
  const dir = fixture("refused-path");
  writeFileSync(join(dir, "lib.rs"), `unsafe extern "Rust" {
    #[link_name = "window[\\"$RT\\"]"]
    safe static RT: Option<&'static js::Unknown>;
    #[link_name = "window['a'b']"]
    safe static AB: Option<&'static js::Unknown>;
}

#[cfg_attr(rust_js, rust_js::link_name = "set window.$R-T")]
fn set_rt(time: f64) {
    let _ = time;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./shell.js#times.shell-time")]
fn shell_time() -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "['matchMedia']")]
fn match_media(this: &js::Unknown) {
    let _ = this;
    unreachable!()
}

pub fn mark(now: f64, on: &js::Unknown) -> bool {
    set_rt(now + shell_time());
    match_media(on);
    RT.is_some() && AB.is_some()
}
`);
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs], root, 60_000);
  expect(p.code).not.toBe(0);
  const out = p.stderr;
  expect(out).toContain("`window.$R-T` isn't a binding's path: `.$R-T` isn't a JS name's property: a key that isn't one is quoted, `['$R-T']`");
  expect(out).toContain("`window[\"$RT\"]` isn't a binding's path: `[\"$RT\"]` isn't a property: one is `.name` or a quoted key, `['name']`");
  expect(out).toContain("`window['a'b']` isn't a binding's path: `['a'b']` isn't a key: one of at least one character, without a quote or a backslash");
  expect(out).toContain("`./shell.js#times.shell-time` isn't a binding's path: `.shell-time` isn't a JS name's property");
  expect(out).toContain("`['matchMedia']` isn't a binding's path: a method's property is named");
});

// A `#[rust_js::name]` on a `pub use` of a binding names its export, as on
// one of the crate's item, renamed or not: react-art's `export const Path =
// Mode.Path;` and `export {Transform}` of `art/core/transform`'s default
// (ADRs 0622, 0240, amended).
test("a pub use of a binding is exported by its name attribute", async () => {
  const dir = fixture("named-binding-reexports");
  writeFileSync(join(dir, "lib.rs"), `pub mod api;

mod bound {
    unsafe extern "Rust" {
        #[link_name = "./transform.js#default"]
        pub safe static TRANSFORM: &'static js::Unknown;
        #[link_name = "Math.PI"]
        pub safe static PI: f64;
        #[link_name = "Math.E"]
        pub safe static E: f64;
    }
}
`);
  writeFileSync(join(dir, "api.rs"), `#[cfg_attr(rust_js, rust_js::name = "Transform")]
pub use crate::bound::TRANSFORM;
#[cfg_attr(rust_js, rust_js::name = "Pi")]
pub use crate::bound::PI;
#[cfg_attr(rust_js, rust_js::name = "Euler")]
pub use crate::bound::E as EULER;
`);
  writeFileSync(join(dir, "transform.js"), "export default function Transform() {}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const api = readFileSync(join(dir, "api.js"), "utf8");
  expect(api).toContain("export const Pi = Math.PI;\nexport const Euler = Math.E;\n");
  expect(api).toMatch(/export \{ default as Transform \} from "\.\.?\/transform\.js";/);
  const loaded = await import(join(dir, "api.js"));
  expect([loaded.Pi, loaded.Euler, loaded.Transform.name]).toEqual([Math.PI, Math.E, "Transform"]);
});

// A private `use` that renames a global binding is the module's `const` of
// it, read by that name, as upstream writes React's `const slice =
// Array.prototype.slice;` and `const isArrayImpl = Array.isArray;`, read as
// the module loads (the user's decision, ADR 0622, amended). One by its own
// name, or `as _`, is read in place, as before.
test("a private use renaming a global binding is the module's const", async () => {
  const dir = fixture("renamed-globals");
  writeFileSync(join(dir, "lib.rs"), `use js::array::SLICE as slice;
use js::array::is_array as is_array_impl;
use js::object::keys;
use js::{Unknown, function};

pub fn copy(list: &Unknown) -> Option<&'static Unknown> {
    function::call(slice, Some(list), ())
}

pub fn both(value: Option<&Unknown>) -> bool {
    is_array_impl(value) && is_array_impl(value)
}

pub fn names(value: &js::JsObject) -> Vec<String> {
    keys(value)
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const slice = Array.prototype.slice;\nconst isArrayImpl = Array.isArray;\n");
  // rd517: a private `const` is no part of the module's type.
  const declared = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declared).not.toContain("slice");
  expect(declared).not.toContain("isArrayImpl");
  expect(declared).toContain("export function copy(list: unknown)");
  expect(js).toContain("  return slice.call(list);\n");
  expect(js).toContain("  return isArrayImpl(value) && isArrayImpl(value);\n");
  expect(js).toContain("  return Object.keys(value);\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.copy({ length: 2, 0: "a", 1: "b" }), lib.both([1]), lib.both("x"), lib.names({ a: 1 })]).toEqual([["a", "b"], true, false, ["a"]]);
});

// rd510: React's Flight reply client's `const FunctionBind =
// Function.prototype.bind;`, its `FunctionBind.apply(this, arguments)` of
// a server reference's `bind`, by the `use`'s name attribute.
test("Function.prototype.bind is a value a private use names", async () => {
  const dir = fixture("function-bind-value");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::name = "FunctionBind")]
use js::function::BIND as FUNCTION_BIND;
use js::{Unknown, function};

pub fn bound(f: &Unknown, this_arg: &Unknown) -> Option<&'static Unknown> {
    function::call(FUNCTION_BIND, Some(f), (this_arg,))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...externs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const FunctionBind = Function.prototype.bind;\n");
  expect(js).toContain("  return FunctionBind.call(f, thisArg);\n");
  const lib = await import(join(dir, "lib.js"));
  const g = lib.bound(function (this: { n: number }) { return this.n; }, { n: 3 });
  expect(g()).toBe(3);
});

// A binding crate's static of a module's default export, art's
// `#[rust_js::name = "FastNoSideEffects"] FAST_NO_SIDE_EFFECTS`, is
// imported by the name it gives, `import FastNoSideEffects from
// "art/modes/fast-noSideEffects"`, as react-art's upstream (rd422);
// one that gives none, by its own.
test("a default import is named as the static that binds it names it", async () => {
  const dir = fixture("default-static-name");
  for (const [name, fast] of [["art-fast", true], ["art-slow", false]] as const) {
    mkdirSync(join(dir, "node_modules", name), { recursive: true });
    writeFileSync(join(dir, "node_modules", name, "package.json"), `{ "name": "${name}", "type": "module", "main": "index.js" }\n`);
    writeFileSync(join(dir, "node_modules", name, "index.js"), `export default { fast: ${fast} };\n`);
  }
  mkdirSync(join(dir, "dep"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), `unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "FastNoSideEffects")]
    #[link_name = "art-fast#default"]
    pub safe static FAST_NO_SIDE_EFFECTS: &'static js::JsObject;
    #[link_name = "art-slow#default"]
    pub safe static SLOW: &'static js::JsObject;
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...externs.slice(1)]);
  mkdirSync(join(dir, "app"), { recursive: true });
  writeFileSync(join(dir, "app", "lib.rs"), `pub fn modes() -> (&'static js::JsObject, &'static js::JsObject) {
    (dep::FAST_NO_SIDE_EFFECTS, dep::SLOW)
}
`);
  run([compiler, join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...externs.slice(1)]);
  const js = readFileSync(join(dir, "app", "lib.js"), "utf8");
  expect(js).toContain('import FastNoSideEffects from "art-fast";');
  expect(js).toContain('import SLOW from "art-slow";');
  expect(js).toContain("return [FastNoSideEffects, SLOW];");
  const lib = await import(join(dir, "app", "lib.js"));
  expect(lib.modes()).toEqual([{ fast: true }, { fast: false }]);
});

// React's ReactSharedInternalsServer declares the binding it makes its
// default and checks it as the module loads, `if (!ReactSharedInternalsServer)`:
// a module that holds a binding reads it by its \`const\` there, in a
// \`js::on_load!\` and a function alike, not its path again (rd659, ADR
// 0633, amended); so does one whose \`use\` it makes its default, \`assign\`.
test("a module that holds a binding reads it by its const", async () => {
  const dir = fixture("held-bindings-at-home");
  writeFileSync(join(dir, "lib.rs"), `pub mod assign;
pub mod server;
`);
  writeFileSync(join(dir, "server.rs"), `pub struct Internals(core::marker::PhantomData<js::JsObject>);

unsafe extern "Rust" {
    #[cfg_attr(rust_js, rust_js::name = "ReactSharedInternalsServer")]
    #[link_name = "react#*.__SERVER_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE"]
    pub safe static REACT_SHARED_INTERNALS_SERVER: Option<&'static Internals>;
}

js::on_load! {
    if REACT_SHARED_INTERNALS_SERVER.is_none() {
        panic!("The \\"react-server\\" condition must be enabled.");
    }
}

pub fn internals() -> Option<&'static Internals> {
    REACT_SHARED_INTERNALS_SERVER
}

js::export_default!(REACT_SHARED_INTERNALS_SERVER);
`);
  writeFileSync(join(dir, "assign.rs"), `use js::Unknown;

pub(crate) use js::object::assign;

pub fn merged<'a>(target: &'a Unknown, source: &Unknown) -> &'a Unknown {
    assign(target, (source,))
}

js::export_default!(assign);
`);
  mkdirSync(join(dir, "node_modules/react"), { recursive: true });
  writeFileSync(join(dir, "node_modules/react/package.json"), '{ "name": "react", "type": "module", "main": "index.js" }\n');
  writeFileSync(join(dir, "node_modules/react/index.js"), "export const __SERVER_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE = { A: null };\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const server = readFileSync(join(dir, "server.js"), "utf8");
  expect(server).toContain("const ReactSharedInternalsServer =\n  React.__SERVER_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;\n\nif (ReactSharedInternalsServer == null) {");
  expect(server).toContain("  return ReactSharedInternalsServer;\n");
  expect(server.match(/__SERVER_INTERNALS/g)).toHaveLength(1);
  const assign = readFileSync(join(dir, "assign.js"), "utf8");
  expect(assign).toContain("const assign = Object.assign;");
  expect(assign).toContain("  return assign(target, source);\n");
  const lib = await import(join(dir, "server.js"));
  const react = await import(join(dir, "node_modules/react/index.js"));
  expect(lib.internals()).toBe(react.__SERVER_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE);
  expect(lib.default).toBe(lib.internals());
  expect((await import(join(dir, "assign.js"))).merged({ a: 1 }, { b: 2 })).toEqual({ a: 1, b: 2 });
});
