// A `#[rust_js::nullable]` field's `None` is `null`, of an `Option` and of
// a `Cell` of one alike (ADRs 0275, 0772): where the struct is made, its
// comment kept, and where the cell is set, replaced or taken. Each test is
// a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's `ReactSharedInternalsClient`, whose `H` a renderer sets, and
// `createContext`'s `_currentRenderer`, set to `null` again in DEV.
test("a nullable Cell field's None is null where it's made and where it's written", async () => {
  const dir = fixture("nullable-cell-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Context {
    pub value: Cell<f64>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub renderer: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub label: Option<&'static str>,
}

pub fn make(value: f64) -> Context {
    Context {
        value: Cell::new(value),
        // These are circular
        renderer: Cell::new(None),
        // Not yet named
        label: None,
    }
}

pub fn reset(context: &Context) {
    context.renderer.set(None);
}

pub fn set(context: &Context, renderer: &'static Unknown) {
    context.renderer.set(Some(renderer));
}

pub fn given(context: &Context, renderer: Option<&'static Unknown>) {
    context.renderer.set(renderer);
}

pub fn taken(context: &Context) -> bool {
    context.renderer.take().is_some()
}

pub fn replaced(context: &Context, renderer: &'static Unknown) -> bool {
    context.renderer.replace(Some(renderer)).is_some()
}

pub fn has(context: &Context) -> bool {
    context.renderer.get().is_some()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    value,\n    // These are circular\n    renderer: null,\n    // Not yet named\n    label: null,\n");
  expect(js).toContain("export function reset(context) {\n  context.renderer = null;\n}");
  expect(js).toContain("export function set(context, renderer) {\n  context.renderer = renderer;\n}");
  expect(js).toContain("export function given(context, renderer) {\n  context.renderer = renderer ?? null;\n}");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("    renderer?: unknown | null | undefined;\n    label?: string | null | undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  const context = lib.make(1);
  expect([context.renderer, lib.has(context)]).toEqual([null, false]);
  lib.set(context, "dom");
  expect([context.renderer, lib.has(context)]).toEqual(["dom", true]);
  expect([lib.taken(context), context.renderer]).toEqual([true, null]);
  expect([lib.replaced(context, "art"), context.renderer]).toEqual([false, "art"]);
  lib.given(context, undefined);
  expect([context.renderer, lib.has(context)]).toEqual([null, false]);
  lib.set(context, "dom");
  lib.reset(context);
  expect(context.renderer).toBe(null);
});
