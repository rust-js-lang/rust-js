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

// Each body's calls are its own facts, read once (#191): a closure's
// `set(None)` writes `null` as its function's does, two calls in one body
// are each theirs, and a cell that isn't nullable beside them keeps
// `undefined`.
test("a nullable Cell field's None is null in a closure as in its function, each call its own", async () => {
  const dir = fixture("nullable-cell-bodies");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Context {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub renderer: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub previous: Cell<Option<&'static Unknown>>,
    pub plain: Cell<Option<&'static Unknown>>,
}

pub fn make() -> Context {
    Context {
        renderer: Cell::new(None),
        previous: Cell::new(None),
        plain: Cell::new(None),
    }
}

pub fn swap(context: &Context, renderer: &'static Unknown) {
    context.previous.set(context.renderer.take());
    context.renderer.set(Some(renderer));
    context.plain.set(None);
}

pub fn reset_later(context: &Context) -> impl Fn() + '_ {
    move || {
        context.renderer.set(None);
        context.previous.set(None);
        context.plain.set(None);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    context.renderer = null;\n    context.previous = null;\n    context.plain = undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  const context = lib.make();
  lib.swap(context, "dom");
  expect([context.renderer, context.previous, context.plain]).toEqual(["dom", null, undefined]);
  lib.swap(context, "art");
  expect([context.renderer, context.previous]).toEqual(["art", "dom"]);
  lib.resetLater(context)();
  expect([context.renderer, context.previous, context.plain]).toEqual([null, null, undefined]);
});

// The storage decides, not how it's reached (#191): a nullable field's
// `Cell` reached through a local reference, or given to a helper as a
// `&Cell`, stores its `None` as `null`, as it does written directly; the
// same helper given a `Cell` that isn't nullable stores `undefined`. Each
// returns what it held, and what's given runs once.
test("a nullable Cell field's None is null through a reference and a helper, as its storage says", async () => {
  const dir = fixture("nullable-cell-references");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Context {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub renderer: Cell<Option<&'static Unknown>>,
    pub plain: Cell<Option<&'static Unknown>>,
}

pub fn make(renderer: &'static Unknown) -> Context {
    Context {
        renderer: Cell::new(Some(renderer)),
        plain: Cell::new(Some(renderer)),
    }
}

fn cleared(cell: &Cell<Option<&'static Unknown>>) -> bool {
    cell.replace(None).is_some()
}

fn emptied(cell: &Cell<Option<&'static Unknown>>) -> bool {
    cell.take().is_some()
}

fn reset(cell: &Cell<Option<&'static Unknown>>) {
    cell.set(None);
}

pub fn through_helpers(context: &Context) -> (bool, bool) {
    (cleared(&context.renderer), cleared(&context.plain))
}

pub fn emptied_both(context: &Context) -> (bool, bool) {
    (emptied(&context.renderer), emptied(&context.plain))
}

pub fn reset_both(context: &Context) {
    reset(&context.renderer);
    reset(&context.plain);
}

pub fn through_alias(context: &Context, next: &'static Unknown) -> bool {
    let renderer = &context.renderer;
    renderer.set(None);
    let taken = renderer.take();
    let old = renderer.replace(Some(next));
    taken.is_none() && old.is_none()
}

pub fn alias_reset(context: &Context) {
    let renderer = &context.renderer;
    let plain = &context.plain;
    renderer.set(None);
    plain.set(None);
}

pub fn alias_take(context: &Context) -> bool {
    let renderer = &context.renderer;
    renderer.take().is_some()
}

pub fn copied_reset(context: &Context) {
    let renderer = &context.renderer;
    let copied = renderer;
    copied.set(None);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // The handle on the nullable field stores as the field does; the plain one as it's given.
  expect(js).toContain("      set value(value) {\n        context.renderer = value ?? null;\n      },");
  expect(js).toContain("      set value(value) {\n        context.plain = value;\n      },");
  expect(js).toContain("export function aliasReset(context) {\n  context.renderer = null;\n  context.plain = undefined;\n}");
  const lib = await import(join(dir, "lib.js"));
  const context = lib.make("dom");
  expect(lib.throughHelpers(context)).toEqual([true, true]);
  expect([context.renderer, context.plain]).toEqual([null, undefined]);
  expect(lib.throughHelpers(context)).toEqual([false, false]);
  Object.assign(context, { renderer: "dom", plain: "dom" });
  expect(lib.emptiedBoth(context)).toEqual([true, true]);
  expect([context.renderer, context.plain]).toEqual([null, undefined]);
  Object.assign(context, { renderer: "dom", plain: "dom" });
  lib.resetBoth(context);
  expect([context.renderer, context.plain]).toEqual([null, undefined]);
  expect(lib.throughAlias(context, "art")).toBe(true);
  expect(context.renderer).toBe("art");
  lib.aliasReset(context);
  expect([context.renderer, context.plain]).toEqual([null, undefined]);
  context.renderer = "dom";
  lib.copiedReset(context);
  expect(context.renderer).toBe(null);
  context.renderer = "dom";
  expect([lib.aliasTake(context), context.renderer]).toEqual([true, null]);
});
