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

// React's `Transition`, whose `types` a nested transition shares and the
// outer one is given back: a field's `RefCell<Option<_>>` only ever
// borrowed for a moment is the value it holds, its `None` `null` where it's
// written, left out where it's made, `get_mut()` and `replace` alike (ADR
// 0863); and one that's a cell of its own is refused.
test("a nullable RefCell field's None is null where it's written", async () => {
  const dir = fixture("nullable-refcell-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Transition {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,
}

pub fn start(prev: Option<&Transition>) -> Transition {
    let mut current = Transition {
        types: RefCell::new(None),
    };
    *current.types.get_mut() = match prev {
        Some(prev) => prev.types.borrow().clone(),
        None => None,
    };
    current
}

pub fn add(transition: &Transition, ty: &str) {
    let types = transition.types.borrow().clone();
    match types {
        None => {
            transition.types.replace(Some(Rc::new(RefCell::new(vec![ty.to_string()]))));
        }
        Some(types) => types.borrow_mut().push(ty.to_string()),
    }
}

pub fn clear(transition: &Transition) -> bool {
    transition.types.replace(None).is_some()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const current = {};\n  current.types = prev?.types ?? null;\n  return current;\n");
  expect(js).toContain("    transition.types = [ty];\n");
  expect(js).toContain("  transition.types = null;\n");
  expect(js).not.toContain("$borrow");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("types?: string[] | null | undefined;");
  const lib = await import(join(dir, "lib.js"));
  const outer = lib.start(null);
  expect(outer.types).toBe(null);
  lib.add(outer, "a");
  const inner = lib.start(outer);
  lib.add(inner, "b");
  expect([outer.types, inner.types === outer.types]).toEqual([["a", "b"], true]);
  expect([lib.clear(inner), inner.types, lib.clear(inner)]).toEqual([true, null, false]);
  expect(lib.start(lib.start(null)).types).toBe(null);
  const cell = fixture("nullable-refcell-cell");
  writeFileSync(join(cell, "lib.rs"), `use std::cell::RefCell;

pub struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub held: RefCell<Option<u32>>,
}

pub fn lent(slot: &Slot) -> &RefCell<Option<u32>> {
    &slot.held
}

pub fn clear(slot: &Slot) {
    *slot.held.borrow_mut() = None;
}
`);
  // One lent is the field's one cell, its `value` the property, its `None`
  // written `null`, whoever made the object (ADR 0863, amended).
  run([compiler, join(cell, "lib.rs"), "-o", join(cell, "lib.js")]);
  expect(readFileSync(join(cell, "lib.js"), "utf8")).toContain('  return $fieldCell(slot, "held", true);\n');
  const slots = await import(join(cell, "lib.js"));
  const slot = { held: 3 };
  const lent = slots.lent(slot);
  const read = lent.value;
  lent.value = undefined;
  expect([read, slot.held, slots.lent(slot) === lent]).toEqual([3, null, true]);
  // And one written through its guard, where nothing counts its borrows.
  const other: { held: number | null } = { held: 4 };
  slots.clear(other);
  expect(other.held).toBe(null);
});

// React's root's lane maps, `createLaneMap(null)`, whose items JS tests
// `=== null`: a `#[rust_js::nullable]` field of a `Vec<Option<T>>` holds
// its items' `None` as `null`, each written by index or taken, and each
// `None` given where its items are made (ADR 0586, amended).
test("a nullable field's Vec of Options holds null for each None", async () => {
  const dir = fixture("nullable-items");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

pub struct FiberRoot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub transition_lanes: RefCell<Vec<Option<u32>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub hidden_updates: Vec<Option<Vec<u32>>>,
}

pub fn create_lane_map<T: Copy>(initial: T) -> Vec<T> {
    let mut lane_map = Vec::new();
    for _ in 0..3 {
        lane_map.push(initial);
    }
    lane_map
}

pub fn create_root() -> FiberRoot {
    FiberRoot {
        transition_lanes: RefCell::new(create_lane_map(None)),
        hidden_updates: vec![Some(vec![1]), Some(vec![]), Some(vec![2, 3])],
    }
}

pub fn mark(root: &FiberRoot, index: usize, lane: u32) {
    let mut lanes = root.transition_lanes.borrow_mut();
    lanes[index] = Some(lanes[index].unwrap_or(0) | lane);
}

pub fn clear(root: &FiberRoot, index: usize) {
    if root.transition_lanes.borrow()[index].is_some() {
        root.transition_lanes.borrow_mut()[index] = None;
    }
}

pub fn take_hidden(root: &mut FiberRoot, index: usize) -> usize {
    match root.hidden_updates[index].take() {
        Some(updates) => updates.len(),
        None => 0,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    transitionLanes: createLaneMap(null),\n");
  expect(js).toContain("] = null;\n");
  expect(js).not.toContain("= undefined;");
  const lib = await import(join(dir, "lib.js"));
  const root = lib.createRoot();
  expect(root.transitionLanes).toEqual([null, null, null]);
  lib.mark(root, 1, 4);
  lib.mark(root, 1, 1);
  expect(root.transitionLanes).toEqual([null, 5, null]);
  lib.clear(root, 1);
  lib.clear(root, 2);
  expect(root.transitionLanes).toEqual([null, null, null]);
  expect([lib.takeHidden(root, 2), lib.takeHidden(root, 2), lib.takeHidden(root, 1)]).toEqual([2, 0, 0]);
  expect(root.hiddenUpdates).toEqual([[1], null, null]);
});

// React's `Fiber`, whose `child` the reconciler's other files read: a
// struct's `RefCell` field is the value it holds by its declaration, not
// by whether a body happens to borrow one of its type (ADR 0863, amended).
// One no body reads is the property, typed, as is one another field of its
// type is read beside.
test("a nullable RefCell field no body borrows is the value it holds", async () => {
  const dir = fixture("nullable-refcell-unread");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub child: RefCell<Option<Rc<Fiber>>>,
    pub sibling: RefCell<Option<Rc<Fiber>>>,
}

pub fn tag(f: &Fiber) -> u8 {
    f.tag
}

pub fn leaf(tag: u8) -> Fiber {
    Fiber {
        tag,
        child: RefCell::new(None),
        sibling: RefCell::new(None),
    }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return {\n    tag,\n    child: null,\n  };\n");
  const types = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(types).toContain("    child?: Fiber | null | undefined;\n    sibling: Fiber | null | undefined;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.tag(lib.leaf(3)), lib.leaf(3).child]).toEqual([3, null]);
  // Not one of a type whose borrow is held while code that may borrow it
  // runs, which is checked.
  const held = fixture("refcell-field-held");
  writeFileSync(join(held, "lib.rs"), `use std::cell::RefCell;

pub struct Slot {
    pub items: RefCell<Vec<u32>>,
}

pub fn during(items: &RefCell<Vec<u32>>, f: &dyn Fn()) -> usize {
    let borrowed = items.borrow_mut();
    f();
    borrowed.len()
}
`);
  run([compiler, join(held, "lib.rs"), "-o", join(held, "lib.js")]);
  expect(readFileSync(join(held, "lib.js"), "utf8")).toContain("$borrowMut(items");
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
    // Absent until set, read where it may be absent: its handle stores
    // its \`None\` as \`null\` too (#292).
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub current: Cell<Option<&'static Unknown>>,
}

pub fn make(renderer: &'static Unknown) -> Context {
    Context {
        renderer: Cell::new(Some(renderer)),
        plain: Cell::new(Some(renderer)),
        current: Cell::new(Some(renderer)),
    }
}

pub fn current_cleared(context: &Context) -> bool {
    cleared(&context.current)
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
  // An optional one's, through a helper: `null`, as it's set (#292).
  expect([lib.currentCleared(context), context.current]).toEqual([true, null]);
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

// React's \`ReactElement\`, whose \`ref\` is \`null\` where \`props.ref\` isn't there.
pub fn referred(props: &Unknown) -> Context {
    let ref_prop = js::get(props, "ref");
    let renderer = if !js::strict_equals(ref_prop, None::<&Unknown>) {
        ref_prop
    } else {
        None
    };
    Context {
        value: Cell::new(2.0),
        provider: Cell::new(None),
        renderer: Cell::new(renderer),
        label: None,
    }
}

pub struct Titled {
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub title: Option<String>,
}

// A variable moved where the struct's made, not MIR's copy of it.
pub fn untitled() -> Titled {
    let title = None;
    Titled { title }
}

pub fn titled(n: u32) -> Titled {
    let title = if n > 1 { Some(n.to_string()) } else { None };
    Titled { title }
}

pub fn titled_here(n: u32) -> Titled {
    Titled {
        title: if n > 1 { None } else { Some(n.to_string()) },
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
  // Given an \`Option\` that may be \`None\`, it's stored as a nullable field's is, \`null\` for
  // \`None\`: there, as the literal writes it, never \`undefined\` (ADR 0810, amended).
  expect(js).toContain("return {\n    value: 0,\n    provider: null,\n    label: label ?? null,\n  };");
  expect(js).toContain("return {\n    value: 2,\n    provider: null,\n    renderer: renderer ?? null,\n  };");
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
  expect([lib.labeled("a").label, lib.labeled(undefined).label, lib.unlabeled(lib.labeled(undefined))]).toEqual(["a", null, true]);
  // A \`None\` only it's given is \`null\` where it's made, as one only nullable fields read is (ADR 0781).
  expect(js).toContain("const label = null;");
  expect([lib.unnamed().label, lib.unlabeled(lib.unnamed())]).toEqual([null, true]);
  const referred = [lib.referred({ ref: "r" }), lib.referred({}), lib.referred({ ref: null })];
  expect(referred.map((o) => [o.renderer, Object.hasOwn(o, "renderer"), "label" in o, lib.has(o)])).toEqual([
    ["r", true, false, true],
    [null, true, false, false],
    [null, true, false, false],
  ]);
  expect(js).toContain("export function titled(n) {\n  const title = n > 1 ? String(n) : null;\n  return { title };\n}");
  expect([lib.untitled().title, lib.titled(1).title, lib.titled(2).title]).toEqual([null, null, "2"]);
  expect([lib.titledHere(1).title, lib.titledHere(2).title]).toEqual(["1", null]);
  // No optional field is ever \`undefined\` and there.
  for (const o of [prod, dev, lib.labeled(undefined), lib.unnamed(), ...referred, lib.untitled(), lib.titled(1), lib.titledHere(2)]) {
    expect(Object.values(o)).not.toContain(undefined);
  }
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

// React's `startTransition`: `ReactSharedInternals.T`, a nullable getter's
// `null` or a Transition, kept in a `let` and given back to its nullable
// setter, through a copy and through the field of the `Drop` guard that
// restores it, is written as it is, `ReactSharedInternals.T =
// prevTransition;` (ADR 0841, amended). A `let` set again to what may be
// `undefined`, and a guard's field set again, are coalesced still; so is
// what a trait's method marked `#[rust_js::nullable]` gives, whose impl
// may give `undefined`.
test("a nullable function's result is written to a nullable place as it is", async () => {
  const dir = fixture("nullable-result-written");
  writeFileSync(join(dir, "lib.rs"), `use std::marker::PhantomData;

use js::JsObject;

pub struct Internals(PhantomData<JsObject>);

pub struct Transition(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::link_name = "get T")]
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn t(this: &Internals) -> Option<&'static Transition> {
    let _ = this;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "set T")]
#[cfg_attr(rust_js, rust_js::nullable(transition))]
pub fn set_t(this: &Internals, transition: Option<&Transition>) {
    let _ = (this, transition);
    unreachable!()
}

pub fn restore(internals: &Internals, next: &Transition) -> bool {
    let prev_transition = t(internals);
    set_t(internals, Some(next));
    set_t(internals, prev_transition);
    prev_transition.is_some()
}

pub fn copied(internals: &Internals) -> bool {
    let prev_transition = t(internals);
    let again = prev_transition;
    set_t(internals, again);
    prev_transition.is_some()
}

pub fn replaced(internals: &Internals, other: Option<&'static Transition>, replace: bool) {
    let mut prev_transition = t(internals);
    if replace {
        prev_transition = other;
    }
    set_t(internals, prev_transition);
}

struct Restore<'a> {
    internals: &'a Internals,
    prev: Option<&'static Transition>,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        set_t(self.internals, self.prev);
    }
}

pub fn scoped(internals: &Internals, next: &Transition, scope: impl FnOnce()) {
    let prev_transition = t(internals);
    set_t(internals, Some(next));
    let _restore = Restore {
        internals,
        prev: prev_transition,
    };
    scope();
}

pub fn scoped_again(internals: &Internals, other: Option<&'static Transition>, scope: impl FnOnce()) {
    let prev_transition = t(internals);
    let mut restore = Restore {
        internals,
        prev: prev_transition,
    };
    scope();
    restore.prev = other;
}

unsafe extern "Rust" {
    #[link_name = "sharedInternals"]
    safe static SHARED_INTERNALS: &'static Internals;
}

struct Reset(Option<&'static Transition>);

impl Drop for Reset {
    fn drop(&mut self) {
        set_t(SHARED_INTERNALS, self.0);
    }
}

pub fn reset(next: &Transition, scope: impl FnOnce()) {
    let prev_transition = t(SHARED_INTERNALS);
    set_t(SHARED_INTERNALS, Some(next));
    let _reset = Reset(prev_transition);
    scope();
}

pub trait Peek {
    #[cfg_attr(rust_js, rust_js::nullable)]
    fn peek(&self) -> Option<&'static Transition>;
}

pub struct Queue(pub Vec<&'static Transition>);

impl Peek for Queue {
    fn peek(&self) -> Option<&'static Transition> {
        self.0.first().copied()
    }
}

pub fn peeked<P: Peek>(internals: &Internals, queue: &P) {
    set_t(internals, queue.peek());
}

pub fn peeked_queue(internals: &Internals, queue: &Queue) -> bool {
    let first = queue.peek();
    set_t(internals, first);
    first.is_some()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  internals.T = next;\n  internals.T = prevTransition;\n");
  expect(js).toContain("  const again = prevTransition;\n  internals.T = again;\n");
  expect(js).toContain("    prevTransition = other;\n  }\n  internals.T = prevTransition ?? null;\n}");
  expect(js).toContain("  } finally {\n    internals.T = prevTransition;\n  }");
  expect(js).toContain("    prev = other;\n  } finally {\n    internals.T = prev ?? null;\n");
  expect(js).toContain("  } finally {\n    sharedInternals.T = prevTransition;\n");
  expect(js).toContain("  internals.T = PPeek.peek(queue) ?? null;\n");
  expect(js).toContain("  internals.T = first ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const [a, b] = [{}, {}];
  const internals = { T: a as unknown };
  expect(lib.restore(internals, b)).toBe(true);
  expect(internals.T).toBe(a);
  internals.T = null;
  expect(lib.copied(internals)).toBe(false);
  expect(internals.T).toBe(null);
  lib.replaced(internals, undefined, true);
  expect(internals.T).toBe(null);
  internals.T = a;
  lib.scoped(internals, b, () => expect(internals.T).toBe(b));
  expect(internals.T).toBe(a);
  lib.scopedAgain(internals, undefined, () => {});
  expect(internals.T).toBe(null);
  (globalThis as any).sharedInternals = internals;
  lib.reset(b, () => expect(internals.T).toBe(b));
  expect(internals.T).toBe(null);
  delete (globalThis as any).sharedInternals;
  internals.T = a;
  lib.peeked(internals, [[]], lib.queuePeek);
  expect(internals.T).toBe(null);
  internals.T = a;
  expect(lib.peekedQueue(internals, [[]])).toBe(false);
  expect(internals.T).toBe(null);
});

// React's logUncaughtError, ported: `COMPONENT_NAME.set(error_info.source
// .as_deref().and_then(get_component_name_from_fiber))`. What a
// `#[rust_js::nullable]` function gives is `null` for `None`, so its
// `and_then` stores it as it is, as `map`'s `Some`: upstream's `componentName
// = errorInfo.source ? getComponentNameFromFiber(errorInfo.source) : null`,
// not `(.. : undefined) ?? null` (ADR 0841, amended). One that may give
// `undefined` is still coalesced.
test("an and_then of a nullable function is stored as it is", async () => {
  const dir = fixture("nullable-and-then");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

pub struct Fiber {
    pub id: u32,
}

pub struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub source: Option<Box<Fiber>>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn name_of(fiber: &Fiber) -> Option<String> {
    if fiber.id > 2 { Some(fiber.id.to_string()) } else { None }
}

fn maybe(fiber: &Fiber) -> Option<String> {
    if fiber.id > 2 { Some(fiber.id.to_string()) } else { None }
}

pub fn logged(info: &Info) -> Option<String> {
    NAME.set(info.source.as_deref().and_then(name_of));
    NAME.with_borrow(Clone::clone)
}

pub fn maybe_logged(info: &Info) -> Option<String> {
    NAME.set(info.source.as_deref().and_then(maybe));
    NAME.with_borrow(Clone::clone)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  name = info.source != null ? nameOf(info.source) : null;");
  expect(js).toContain("  name = (info.source != null ? maybe(info.source) : undefined) ?? null;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.logged({ source: { id: 3 } } as never), lib.logged({ source: { id: 1 } } as never), lib.logged({ source: null } as never)]).toEqual(["3", null, null]);
  expect([lib.maybeLogged({ source: { id: 1 } } as never), lib.maybeLogged({ source: { id: 4 } } as never)]).toEqual([null, "4"]);
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

// One of the crate's functions is never `undefined` (`Known::Item`): a
// nullable place stores it as it is, `h.done = noop`, where it's set and
// where an `Option` of it is mapped to it (ADR 0693, amended).
test("a function a nullable place is given is stored as it is", async () => {
  const dir = fixture("nullable-function-item");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub done: Cell<Option<fn() -> u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub picked: Option<fn() -> u32>,
}

fn noop() -> u32 {
    7
}

pub fn make() -> Holder {
    Holder {
        done: Cell::new(None),
        picked: None,
    }
}

pub fn finish(h: &Holder) {
    h.done.set(Some(noop));
}

pub fn pick(h: &mut Holder, given: Option<u32>) {
    h.picked = given.map(|_| noop as fn() -> u32);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  h.done = noop;\n");
  expect(js).toContain("  h.picked = given != null ? noop : null;\n");
  const lib = await import(join(dir, "lib.js"));
  const h = lib.make();
  lib.finish(h);
  lib.pick(h, null);
  expect([h.done(), h.picked]).toEqual([7, null]);
  lib.pick(h, 0);
  expect(h.picked()).toBe(7);
});

// React's `findFirstSuspended` keeps a fiber's \`return\` in a variable and
// stores it in another's, \`sibling.return = node.return\`; ReactFiberRoot
// takes the root's transition types, \`const claimed = ..; return
// claimed\`. What's read of a nullable field is \`null\` or the value, never
// \`undefined\`: a variable holding it is stored, and returned by a
// nullable function, as it is, no \`?? null\` (ADR 0620, amended).
test("a nullable field's value a variable holds is stored as it is", async () => {
  const dir = fixture("nullable-held");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub name: String,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
}

pub struct Root {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub transition_types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,
}

pub fn fiber(name: &str, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { name: name.to_string(), r#return: RefCell::new(parent) })
}

pub fn adopt(node: &Fiber, sibling: &Fiber) {
    let return_fiber = node.r#return.borrow().clone();
    sibling.r#return.replace(return_fiber);
}

pub fn root(types: bool) -> Root {
    let types = types.then(|| Rc::new(RefCell::new(vec!["a".to_string()])));
    Root { transition_types: RefCell::new(types) }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn claim(root: &Root) -> Option<Rc<RefCell<Vec<String>>>> {
    let claimed = root.transition_types.take();
    claimed
}

/// An optional one, absent until it's set, may be \`undefined\`.
pub struct Props {
    #[cfg_attr(rust_js, rust_js::nullable)]
    #[cfg_attr(rust_js, rust_js::optional)]
    pub banner: RefCell<Option<String>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub shown: RefCell<Option<String>>,
}

pub fn props() -> Props {
    Props { banner: RefCell::new(None), shown: RefCell::new(Some("x".to_string())) }
}

pub fn show(props: &Props) {
    let banner = props.banner.borrow().clone();
    props.shown.replace(banner);
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const returnFiber = node.return;\n  sibling.return = returnFiber;\n");
  expect(js).toContain("  const claimed = root.transitionTypes;\n  root.transitionTypes = null;\n  return claimed;\n");
  const lib = await import(join(dir, "lib.js"));
  const parent = lib.fiber("parent", null);
  const [child, orphan, other] = [lib.fiber("child", parent), lib.fiber("orphan", null), lib.fiber("other", parent)];
  lib.adopt(child, other);
  lib.adopt(orphan, child);
  expect([other.return === parent, child.return]).toEqual([true, null]);
  const [some, none] = [lib.root(true), lib.root(false)];
  expect([lib.claim(some), lib.claim(some), lib.claim(none)]).toEqual([["a"], null, null]);
  expect(js).toContain("  props.shown = banner ?? null;\n");
  const props = lib.props();
  lib.show(props);
  expect(props.shown).toBe(null);
});

// React's \`findFirstSuspended\` takes a fiber's sibling once its loop's
// test says it's there, \`while (node.sibling === null) { .. } node.sibling\`:
// its port's \`node.sibling.borrow().clone().unwrap()\` reads the field
// again, nothing between, so it's the field, no \`$unwrap\`; where a call
// comes between, which may change it, the \`$unwrap\` stays (ADR 0620,
// amended).
test("an unwrap of a field a test says is there is the field", async () => {
  const dir = fixture("unwrap-tested-field");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub name: String,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub sibling: RefCell<Option<Rc<Fiber>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
}

pub fn fiber(name: &str, sibling: Option<Rc<Fiber>>, parent: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { name: name.to_string(), sibling: RefCell::new(sibling), r#return: RefCell::new(parent) })
}

pub fn next(mut node: Rc<Fiber>) -> Option<String> {
    while node.sibling.borrow().is_none() {
        let parent = node.r#return.borrow().clone()?;
        node = parent;
    }
    let sibling = node.sibling.borrow().clone().unwrap();
    Some(sibling.name.clone())
}

pub fn changed(node: &Fiber, clear: &dyn Fn(&Fiber)) -> String {
    if node.sibling.borrow().is_some() {
        clear(node);
        let sibling = node.sibling.borrow().clone().unwrap();
        return sibling.name.clone();
    }
    String::new()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const sibling = node.sibling;\n");
  expect(js).toContain("const sibling = $unwrap(node.sibling);\n");
  const lib = await import(join(dir, "lib.js"));
  const uncle = lib.fiber("uncle", null, null);
  const parent = lib.fiber("parent", uncle, null);
  expect([lib.next(lib.fiber("child", null, parent)), lib.next(lib.fiber("lone", null, null))]).toEqual(["uncle", undefined]);
  expect(() => lib.changed(parent, (n: { sibling: unknown }) => (n.sibling = null))).toThrow("on a `None` value");
});

// A nullable function's `return ref_prop;` where `js::strict_equals(ref_prop,
// None)` holds: `ref_prop` is `undefined` there, its `None` `null` as the
// function gives it, `refProp ?? null`. It was `return refProp;`, giving
// `undefined`: the test against the `None` given, `refProp === undefined`,
// was read as one against a literal, which says it's there (ADR 0620,
// amended).
test("a nullable result tested equal to a None given is null", async () => {
  const dir = fixture("nullable-tested-none");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn ref_or(props: Option<&'static js::Unknown>, other: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    let ref_prop = js::get(props, "ref");
    if js::strict_equals(ref_prop, None::<&js::Unknown>) {
        return ref_prop;
    }
    other
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (refProp === undefined) {\n    return refProp ?? null;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.refOr({} as never, 4 as never), lib.refOr({ ref: 3 } as never, 4 as never)]).toEqual([null, 4]);
});

// A generic struct's field of a type parameter's value, React's
// `StackCursor<T>`'s `current: RefCell<T>` and `ContextDependency<T>`'s
// `memoizedValue: Cell<T>`, `#[rust_js::nullable]`: where `T` is an
// `Option`, its `None` is `null` where the struct is made and where the
// cell is set or replaced, as React's JS writes it; where `T` is anything
// else, or a type parameter's value in generic code, what's given as it
// is (ADR 0586, amended).
test("a generic struct's nullable field is null for None where it's an Option", async () => {
  const dir = fixture("nullable-generic-fields");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};

pub struct StackCursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current: RefCell<T>,
}

pub struct ContextDependency<T: 'static> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub memoized_value: Cell<T>,
}

pub fn create_cursor<T>(default_value: T) -> StackCursor<T> {
    StackCursor {
        current: RefCell::new(default_value),
    }
}

pub fn empty() -> StackCursor<Option<u8>> {
    StackCursor {
        current: RefCell::new(None),
    }
}

pub fn reset(cursor: &StackCursor<Option<u8>>) -> Option<u8> {
    cursor.current.replace(None)
}

pub fn made_of(value: Option<u32>) -> ContextDependency<Option<u32>> {
    ContextDependency {
        memoized_value: Cell::new(value),
    }
}

pub fn cleared(dependency: &ContextDependency<Option<u32>>) {
    dependency.memoized_value.set(None);
}

pub fn numbers() -> ContextDependency<u32> {
    ContextDependency {
        memoized_value: Cell::new(1),
    }
}

pub fn read(dependency: &ContextDependency<Option<u32>>) -> Option<u32> {
    dependency.memoized_value.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function createCursor(defaultValue) {\n  return {\n    current: defaultValue,\n  };\n}");
  expect(js).toContain("  return {\n    current: null,\n  };\n");
  expect(js).toContain("  cursor.current = null;\n");
  expect(js).toContain("    memoizedValue: value ?? null,\n");
  expect(js).toContain("  dependency.memoizedValue = null;\n");
  expect(js).toContain("    memoizedValue: 1,\n");
  const lib = await import(join(dir, "lib.js"));
  const cursor = lib.empty();
  expect(cursor).toEqual({ current: null });
  cursor.current = 4;
  expect([lib.reset(cursor), cursor.current]).toEqual([4, null]);
  const dependency = lib.madeOf(undefined);
  expect([dependency.memoizedValue, lib.read(lib.madeOf(2))]).toEqual([null, 2]);
  lib.cleared(dependency);
  expect([dependency.memoizedValue, lib.read(dependency), lib.numbers().memoizedValue]).toEqual([null, null, 1]);
});

// React's unsafe_markUpdateLaneFromFiberToRoot ends `const root = ..; return
// root;` of what a nullable function gave, a variable's, held while
// more runs: `null` for `None` already, so returned and stored as it is,
// not `root ?? null` (ADR 0620, amended).
test("a variable of what a nullable function gave is returned and stored as it is", async () => {
  const dir = fixture("nullable-call-kept");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Root {
    pub tag: u32,
}

pub struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub root: RefCell<Option<Rc<Root>>>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn lookup(roots: &[Rc<Root>], tag: u32) -> Option<Rc<Root>> {
    roots.iter().find(|r| r.tag == tag).cloned()
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn found(roots: &[Rc<Root>], tag: u32, seen: &mut u32) -> Option<Rc<Root>> {
    let root = lookup(roots, tag);
    *seen += 1;
    root
}

pub fn held(holder: &Holder, roots: &[Rc<Root>], tag: u32) {
    let root = lookup(roots, tag);
    holder.root.replace(root);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  seen.value = (seen.value + 1) >>> 0;\n  return root;\n");
  expect(js).toContain("  const root = lookup(roots, tag);\n  holder.root = root;\n");
  const lib = await import(join(dir, "lib.js"));
  const roots = [{ tag: 1 }, { tag: 2 }];
  const seen = { value: 0 };
  const holder = { root: roots[0] as unknown };
  lib.held(holder, roots, 3);
  expect([lib.found(roots, 2, seen), lib.found(roots, 5, seen), holder.root]).toEqual([roots[1], null, null]);
});
