// A `#[rust_js::nullable]` field's `None` is `null`, of an `Option` and of
// a `Cell` of one alike (ADRs 0275, 0772): where the struct is made, its
// comment kept, and where the cell is set, replaced or taken. Each test is
// a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, installRuntime, run, target } from "./support";

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

// React's lazy(): `const owner = null; .. owner: owner`. A `None` bound to a
// variable that only nullable fields and binding parameters read is
// `null` where it's made, and read as it is (ADR 0781).
test("a None only nullable fields and parameters read is null where it's made", async () => {
  const dir = fixture("nullable-none-made");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Info {
    pub name: Cell<&'static str>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub owner: Option<&'static Unknown>,
}

#[cfg_attr(rust_js, rust_js::link_name = "Object.is")]
#[cfg_attr(rust_js, rust_js::nullable(a))]
#[allow(unused_variables)]
fn same(a: Option<&Unknown>, b: f64) -> bool {
    unreachable!()
}

pub fn leaked() -> &'static Info {
    let owner = None;
    let info: &'static Info = Box::leak(Box::new(Info {
        name: Cell::new("lazy"),
        owner,
    }));
    info
}

pub fn twice() -> (Info, Info) {
    let owner = None;
    (
        Info { name: Cell::new("a"), owner },
        Info { name: Cell::new("b"), owner },
    )
}

pub fn given() -> bool {
    let owner = None;
    same(owner, 1.0)
}

pub fn also_tested() -> (Info, bool) {
    let owner = None;
    (Info { name: Cell::new("c"), owner }, owner.is_none())
}

pub fn copied() -> (Info, Option<&'static Unknown>) {
    let owner = None;
    let other = owner;
    (Info { name: Cell::new("d"), owner }, other)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function leaked() {
  const owner = null;
  const info = {
    name: "lazy",
    owner,
  };`);
  expect(js).toContain(`export function twice() {
  const owner = null;`);
  expect(js).toContain(`export function given() {
  const owner = null;
  return Object.is(owner, 1);`);
  // Read otherwise, it's \`undefined\`, as every \`None\` is, and \`?? null\` where a field takes it.
  expect(js).toContain(`owner: owner ?? null`);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.leaked().owner, lib.twice()[1].owner, lib.given(), lib.alsoTested()[0].owner, lib.alsoTested()[1]]).toEqual([null, null, false, null, true]);
  expect(lib.copied()[1]).toBeUndefined();
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

pub fn closure_reset(context: &Context) {
    let renderer = &context.renderer;
    let reset = || renderer.set(None);
    reset();
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
  // A closure's, through the variable it captures (#192).
  context.renderer = "dom";
  lib.closureReset(context);
  expect(context.renderer).toBe(null);
});

// React's `createContext`, whose DEV-only `_currentRenderer` isn't in its
// literal and is set to `null` in DEV, Flow's `_currentRenderer?: Object |
// null`: a nullable field marked `#[rust_js::optional]` is left out where
// its struct is made with `None`, and `null` where it's set to `None`; read,
// absent, `null` or set, its `None` is `None` (ADR 0810).
test("an optional nullable field is left out where it's made and null where it's set", async () => {
  const dir = fixture("nullable-optional-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::Unknown;

pub struct Context {
    pub value: Cell<f64>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub provider: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub renderer: Cell<Option<&'static Unknown>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub label: Option<&'static str>,
}

pub struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub name: Option<&'static str>,
}

pub fn make(value: f64, dev: bool) -> Context {
    let context = Context {
        value: Cell::new(value),
        provider: Cell::new(None),
        renderer: Cell::new(None),
        label: None,
    };
    if dev {
        context.renderer.set(None);
    }
    context
}

pub fn labeled(label: Option<&'static str>) -> Context {
    Context {
        value: Cell::new(0.0),
        provider: Cell::new(None),
        renderer: Cell::new(None),
        label,
    }
}

pub fn unnamed() -> Context {
    let label = None;
    Context {
        value: Cell::new(1.0),
        provider: Cell::new(None),
        renderer: Cell::new(None),
        label,
    }
}

pub fn info(context: &Context) -> Info {
    Info { name: context.label }
}

pub fn set(context: &Context, renderer: &'static Unknown) {
    context.renderer.set(Some(renderer));
}

pub fn has(context: &Context) -> bool {
    context.renderer.get().is_some()
}

pub fn same(a: &Context, b: &Context) -> bool {
    a.renderer.get() == b.renderer.get()
}

pub fn which(context: &Context) -> u32 {
    match context.renderer.get() {
        Some(_) => 1,
        None => 0,
    }
}

pub fn unlabeled(context: &Context) -> bool {
    context.label.is_none()
}

pub fn unlabel(context: &mut Context) {
    context.label = None;
}

pub fn relabel(context: &mut Context, label: Option<&'static str>) {
    context.label = label;
}

pub fn label(context: &mut Context, label: &'static str) {
    context.label = Some(label);
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const context = {\n    value,\n    provider: null,\n  };\n  if (dev) {\n    context.renderer = null;\n  }");
  // Given an \`Option\` that may be \`None\`, as it is: \`undefined\`, as a key that isn't there reads.
  expect(js).toContain("return {\n    value: 0,\n    provider: null,\n    label,\n  };");
  // A nullable field given one's value, which may be \`undefined\`, makes it \`null\`.
  expect(js).toContain("return { name: context.label ?? null };");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("    renderer?: unknown | null | undefined;\n    label?: string | null | undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  const prod = lib.make(1, false);
  const dev = lib.make(1, true);
  expect(["renderer" in prod, "label" in prod, dev.renderer]).toEqual([false, false, null]);
  expect([lib.has(prod), lib.has(dev), lib.which(prod), lib.which(dev), lib.same(prod, dev), lib.unlabeled(prod)]).toEqual([false, false, 0, 0, true, true]);
  expect(lib.info(prod).name).toBe(null);
  lib.set(dev, "dom");
  expect([lib.has(dev), lib.which(dev), lib.same(prod, dev)]).toEqual([true, 1, false]);
  expect([lib.labeled("a").label, "label" in lib.labeled(undefined), lib.unlabeled(lib.labeled(undefined))]).toEqual(["a", true, true]);
  // A \`None\` only it's given isn't \`null\` where it's made, as one only nullable fields read is (ADR 0781).
  expect([lib.unnamed().label, lib.unlabeled(lib.unnamed())]).toEqual([undefined, true]);
  // Assigned, as its cell is set: `None` is `null`, an `Option` `?? null`, a `Some` its value.
  expect(js).toContain("export function unlabel(context) {\n  context.label = null;\n}");
  expect(js).toContain("export function relabel(context, label) {\n  context.label = label ?? null;\n}");
  expect(js).toContain("export function label(context, label) {\n  context.label = label;\n}");
  lib.label(prod, "b");
  expect([prod.label, lib.unlabeled(prod)]).toEqual(["b", false]);
  lib.unlabel(prod);
  expect([prod.label, lib.unlabeled(prod)]).toEqual([null, true]);
  lib.relabel(prod, undefined);
  expect([prod.label, lib.unlabeled(prod)]).toEqual([null, true]);
});

// React's `createContext`, whose `Consumer` is set to an object made there,
// and `lazy`, whose DEV-only `_debugTask` is `console.createTask`'s where
// there's one: what's never `undefined`, an object made there, a `Some`'s
// value, is stored as it is, through a handle too, and a `map`'s `None` is
// `null` where it's made; what may be `undefined` is `?? null` (ADR 0841).
test("a nullable field is written what's never undefined as it is", async () => {
  const dir = fixture("nullable-defined");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

use js::SymbolFor;
use webapi::console;

pub const REACT_CONSUMER_TYPE: SymbolFor = SymbolFor("react.consumer");

pub struct Context {
    #[cfg_attr(rust_js, rust_js::name = "Provider")]
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub provider: Cell<Option<&'static Context>>,
    #[cfg_attr(rust_js, rust_js::name = "Consumer")]
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub consumer: Cell<Option<&'static Consumer>>,
}

pub struct Consumer {
    #[cfg_attr(rust_js, rust_js::name = "$$typeof")]
    pub r#typeof: SymbolFor<'static>,
    #[cfg_attr(rust_js, rust_js::name = "_context")]
    pub context: &'static Context,
}

pub fn create_context() -> &'static Context {
    let context: &'static Context = Box::leak(Box::new(Context {
        provider: Cell::new(None),
        consumer: Cell::new(None),
    }));
    context.provider.set(Some(context));
    context.consumer.set(Some(Box::leak(Box::new(Consumer {
        r#typeof: REACT_CONSUMER_TYPE,
        context,
    }))));
    context
}

fn consumer_of(context: &'static Context) -> &'static Consumer {
    Box::leak(Box::new(Consumer {
        r#typeof: REACT_CONSUMER_TYPE,
        context,
    }))
}

pub fn reconsumed(context: &'static Context) {
    context.consumer.set(Some(consumer_of(context)));
}

pub fn given(context: &'static Context, consumer: Option<&'static Consumer>) {
    context.consumer.set(consumer);
}

pub struct Lazy {
    #[cfg_attr(rust_js, rust_js::name = "_debugTask")]
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub debug_task: Option<&'static webapi::ConsoleTask>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub label: Option<f64>,
}

pub fn lazy(n: Option<f64>) -> Lazy {
    Lazy {
        debug_task: console::create_task().map(|create_task| create_task("lazy()")),
        label: n.map(|n| n + 1.0),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Across lines, as its literal is, and React's (ADR 0835).
  expect(js).toContain("  context.Provider = context;\n  context.Consumer = {\n    $$typeof: REACT_CONSUMER_TYPE,\n    _context: context,\n  };\n");
  expect(js).toContain("  context.Consumer = consumerOf(context);\n");
  expect(js).toContain("  context.Consumer = consumer ?? null;\n");
  expect(js).toContain('_debugTask: createTask ? createTask("lazy()") : null');
  expect(js).toContain("label: n != null ? n + 1 : null");
  expect(js).not.toContain("undefined) ?? null");
  const lib = await import(join(dir, "lib.js"));
  const context = lib.createContext();
  expect([context.Provider, context.Consumer._context]).toEqual([context, context]);
  expect(context.Consumer.$$typeof).toBe(Symbol.for("react.consumer"));
  lib.given(context, undefined);
  expect(context.Consumer).toBe(null);
  lib.reconsumed(context);
  expect(context.Consumer._context).toBe(context);
  expect(lib.lazy(undefined).label).toBe(null);
  expect(lib.lazy(1).label).toBe(2);
  const task = { run: (f: () => unknown) => f() };
  const original = (console as any).createTask;
  (console as any).createTask = undefined;
  expect(lib.lazy(1)._debugTask).toBe(null);
  (console as any).createTask = (name: string) => (name === "lazy()" ? task : undefined);
  expect(lib.lazy(1)._debugTask).toBe(task);
  (console as any).createTask = original;
});

// ADR 0861: a function's own parameter it names `#[rust_js::nullable(..)]`
// is `T | null`, a `None` given it or set in its body `null`, as React's
// `mapIntoArray` sets `children = null` and calls the callback with it,
// whose test says `toHaveBeenCalledWith(null, 5)`. What's given it is as
// it is, the body reading `null` and `undefined` alike.
test("a function's own nullable parameter set to None is null", async () => {
  const dir = fixture("nullable-own-param");
  writeFileSync(join(dir, "lib.rs"), `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable(children))]
fn walk(mut children: Option<&'static Unknown>, callback: &dyn Fn(Option<&'static Unknown>)) {
    let r#type = js::type_of(children);
    if r#type == "undefined" || r#type == "boolean" {
        // All of the above are perceived as null.
        children = None;
    }
    callback(children);
}

fn untouched(mut children: Option<&'static Unknown>, callback: &dyn Fn(Option<&'static Unknown>)) {
    if js::type_of(children) == "boolean" {
        children = None;
    }
    callback(children);
}

fn given(children: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if js::type_of(children) == "number" { children } else { None }
}

pub fn each(children: Option<&'static Unknown>, callback: &dyn Fn(Option<&'static Unknown>)) {
    walk(children, callback);
    walk(None, callback);
    untouched(children, callback);
    walk(given(children), callback);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    // All of the above are perceived as null.\n    children = null;\n");
  expect(js).toContain("  walk(children, callback);\n  walk(null, callback);\n");
  // A call's \`Option\` as it is, which the body reads either way.
  expect(js).toContain("  walk(given(children), callback);\n");
  // A parameter not marked keeps `undefined`, as every `None` is.
  expect(js).toContain("    children = undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  const calls = (children: unknown) => {
    const given: unknown[] = [];
    lib.each(children, (child: unknown) => given.push(child));
    return given;
  };
  expect(calls(undefined)).toStrictEqual([null, null, undefined, null]);
  expect(calls(true)).toStrictEqual([null, null, undefined, null]);
  expect(calls(null)).toStrictEqual([null, null, null, null]);
  expect(calls(5)).toStrictEqual([5, null, 5, 5]);
});

// React's `ReactSharedInternalsClient`, a library: its DEV-only `actQueue`,
// an optional nullable `Cell` of `RendererTask`s, each a `Box<dyn FnMut>`,
// whose drop glue made its `Cell::new(None)` a temporary of its own, read
// `{}.value` where the literal was made (ADR 0810, amended). It's left out,
// as the other optional fields are. Its `set(None)` drops what it held:
// a library's consumer may make a `RendererTask` of a closure holding what
// has a destructor (ADR 0521), so the drop stays, `$dispose` of each.
test("an optional nullable field of what has drop glue is left out where it's made", async () => {
  const dir = fixture("nullable-optional-drops");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct SharedStateClient {
    #[cfg_attr(rust_js, rust_js::name = "H")]
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub h: Cell<Option<&'static str>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub act_queue: Cell<Option<Vec<RendererTask>>>,
    pub async_transitions: Cell<Option<f64>>,
}

#[repr(transparent)]
pub struct RendererTask(pub Box<dyn FnMut(bool) -> Option<RendererTask>>);

pub fn make(dev: bool) -> SharedStateClient {
    let internals = SharedStateClient {
        h: Cell::new(None),
        act_queue: Cell::new(None),
        async_transitions: Cell::new(None),
    };
    if dev {
        internals.act_queue.set(None);
        internals.async_transitions.set(Some(0.0));
    }
    internals
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "manifest.json")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const internals = {\n    H: null,\n  };");
  expect(js).not.toContain("{}.value");
  expect(js).toContain("$dispose(item);");
  installRuntime(dir);
  const lib = await import(join(dir, "lib.js"));
  const prod = lib.make(false);
  const dev = lib.make(true);
  expect(["actQueue" in prod, "asyncTransitions" in prod, dev.actQueue, dev.asyncTransitions]).toEqual([false, false, null, 0]);
});
