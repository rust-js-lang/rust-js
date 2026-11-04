// A binding marked `#[rust_js::require]`, read through `require(module)`
// where it's read, not imported (ADR 0305). Each test is a program of its
// own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0305: a binding marked `#[rust_js::require]` is read through
// `require(module)`, where it's read, as react.dev's runESLint reads
// `require('eslint-plugin-react-hooks').rules`: nothing imports it.
test("a required binding is read through require", async () => {
  const dir = fixture("required");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_upper_case_globals)]
#[cfg_attr(rust_js, rust_js::link_name = "get node:path#sep")]
#[rust_js::require]
fn sep() -> String {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get node:path#delimiter")]
fn delimiter() -> String {
    unreachable!()
}
thread_local! {
    static separator: String = sep();
}
pub fn joined(a: &str, b: &str) -> String {
    separator.with(|s| format!("{a}{s}{b}"))
}
pub fn listed() -> String {
    delimiter()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const separator = require("node:path").sep;');
  expect(js).toContain('import { delimiter } from "node:path";');
  expect(js).not.toContain("import { sep");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.joined("a", "b"), lib.listed()]).toEqual(["a/b", ":"]);
});

// #572: the module's namespace, `#*` (ADR 0256), read through `require`, is
// what `require` gives, the module itself, as `import * as` binds it, not
// its property `"*"`, `undefined`: React's ReactDOMRootFB, `const
// ReactFiberErrorDialogWWW = require('ReactFiberErrorDialog');`, then
// `ReactFiberErrorDialogWWW.showErrorDialog`. A path past it reads the
// module's properties, `require("ReactFiberErrorDialog").options.title`;
// its `.d.ts` is its type's `#[rust_js::types]`. Run in Node, of a
// CommonJS module.
test("a required binding of a whole module is the module", () => {
  const dir = fixture("required-namespace");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

use js::JsObject;

#[cfg_attr(rust_js, rust_js::types = "{ showErrorDialog(message: string): boolean }")]
pub struct ReactFiberErrorDialog(PhantomData<JsObject>);

impl ReactFiberErrorDialog {
    #[cfg_attr(rust_js, rust_js::link_name = "showErrorDialog")]
    pub fn show_error_dialog(&self, message: &str) -> bool {
        unreachable!()
    }
}

#[cfg_attr(rust_js, rust_js::link_name = "get ReactFiberErrorDialog#*")]
#[cfg_attr(rust_js, rust_js::require)]
fn react_fiber_error_dialog() -> &'static ReactFiberErrorDialog {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "get ReactFiberErrorDialog#*.options.title")]
#[cfg_attr(rust_js, rust_js::require)]
fn title() -> String {
    unreachable!()
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::name = "ReactFiberErrorDialogWWW")]
    static REACT_FIBER_ERROR_DIALOG_WWW: &'static ReactFiberErrorDialog = react_fiber_error_dialog();
}

pub fn dialog() -> &'static ReactFiberErrorDialog {
    REACT_FIBER_ERROR_DIALOG_WWW.with(|dialog| *dialog)
}

pub fn shown(message: &str) -> bool {
    dialog().show_error_dialog(message)
}

pub fn titled() -> String {
    title()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const ReactFiberErrorDialogWWW = require("ReactFiberErrorDialog");');
  expect(js).toContain('return require("ReactFiberErrorDialog").options.title;');
  expect(js).not.toContain('["*"]');
  expect(js).not.toMatch(/^import /m);
  const types = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(types).toContain("export function dialog(): {\n    showErrorDialog(message: string): boolean;\n};");
  expect(types).toContain("export function shown(message: string): boolean;");
  expect(types).toContain("export function titled(): string;");
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  mkdirSync(join(dir, "node_modules", "ReactFiberErrorDialog"), { recursive: true });
  writeFileSync(
    join(dir, "node_modules", "ReactFiberErrorDialog", "index.js"),
    "module.exports = { showErrorDialog: (message) => message === 'boom', options: { title: 'www' } };\n",
  );
  // A CommonJS module's own `require`, of the module's file.
  writeFileSync(join(dir, "run.js"), `import { createRequire } from "node:module";
globalThis.require = createRequire(new URL("./lib.js", import.meta.url));
const lib = await import("./lib.js");
console.log(JSON.stringify([lib.dialog() === require("ReactFiberErrorDialog"), lib.shown("boom"), lib.shown("ok"), lib.titled()]));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(JSON.parse(p.stdout.toString())).toEqual([true, true, false, "www"]);
});

// A binding's relative module is relative to the crate root's file (ADR
// 0028), and a module below the root imports it from its own file,
// `../helper.cjs` from `outer/inner.js`. Its `require` named it from the
// root's file, `require("./helper.cjs")`, a file that isn't there, so the
// call threw (#618, P0): it names it as the import does.
test("a required binding in a module below the root requires its file from its own", () => {
  const dir = fixture("required-nested");
  mkdirSync(join(dir, "outer"), { recursive: true });
  writeFileSync(join(dir, "helper.cjs"), "module.exports = { twice: (n) => n * 2 };\n");
  writeFileSync(join(dir, "lib.rs"), "pub mod outer;\n");
  writeFileSync(join(dir, "outer.rs"), "pub mod inner;\n");
  writeFileSync(join(dir, "outer", "inner.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./helper.cjs#twice")]
#[cfg_attr(rust_js, rust_js::require)]
fn twice(_n: f64) -> f64 {
    unreachable!()
}

pub fn doubled(n: f64) -> f64 {
    twice(n)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "outer", "inner.js"), "utf8")).toContain('return require("../helper.cjs").twice(n);');
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  // A CommonJS module's own `require`, of the module's file.
  writeFileSync(join(dir, "run.js"), `import { createRequire } from "node:module";
globalThis.require = createRequire(new URL("./outer/inner.js", import.meta.url));
console.log((await import("./outer/inner.js")).doubled(4));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  expect(p.stdout.toString().trim()).toBe("8");
});
