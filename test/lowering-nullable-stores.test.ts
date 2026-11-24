// What a nullable place is given, stored as it is: a function's own
// nullable parameter set to `None`, an optional field of what has drop
// glue, a function a nullable place is given, a variable's value, an
// unwrap a test shows there, a result tested equal to a `None`, and a
// generic struct's field. Split from lowering-nullable.test.ts by concept
// (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, installStd, run, target } from "./support";
import { withoutBoundary } from "./without-boundary";

beforeAll(buildReact, 600_000);

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
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
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
// an optional nullable `Cell` of `RendererTask`s, each a `Box<dyn FnMut>`.
// Its `Cell::new(None)` was once a temporary of its own, read `{}.value`
// where the literal was made (ADR 0810, amended), when a library's `dyn
// Fn` was taken to hold what a consumer's destructor drops. Now it drops
// what the library's own make hold, nothing, and a consumer whose
// `RendererTask` would is refused (ADR 0626, amended): it's left out, as
// the other optional fields are, and its `set(None)` drops nothing.
test("an optional nullable field of what has drop glue is left out where it's made", async () => {
  const dir = fixture("nullable-optional-drops");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

pub struct SharedStateClient {
    #[cfg_attr(rust_js, rust_js::name = "H")]
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub h: Cell<Option<&'static str>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub act_queue: Cell<Option<Vec<RendererTask>>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
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
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("const internals = { H: null };");
  expect(js).not.toContain("{}.value");
  expect(js).not.toContain("$dispose");
  installStd(dir);
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
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
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

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn take_banner(props: &Props) -> Option<String> {
    props.banner.take()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  // The program's name for it, not the field's own (ADR 0866, amended).
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
  expect(js).toContain("  props.shown = props.banner ?? null;\n");
  const props = lib.props();
  lib.show(props);
  expect(props.shown).toBe(null);
  // Its `take()` may give `undefined` too, so a nullable function returns
  // it `?? null`, React's claimQueuedTransitionTypes (#539).
  expect(js).toContain("  props.banner = null;\n  return old ?? null;\n");
  expect(lib.takeBanner({})).toBe(null);
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
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("const sibling = node.sibling;\n");
  expect(js).toContain("const sibling = $unwrap(node.sibling, \"lib.rs:28:53\");\n");
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
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
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
  const js = withoutBoundary(readFileSync(join(dir, "lib.js"), "utf8"));
  expect(js).toContain("export function createCursor(defaultValue) {\n  return { current: defaultValue };\n}");
  expect(js).toContain("  return { current: null };\n");
  expect(js).toContain("  cursor.current = null;\n");
  expect(js).toContain("  return { memoizedValue: value ?? null };\n");
  expect(js).toContain("  dependency.memoizedValue = null;\n");
  expect(js).toContain("  return { memoizedValue: 1 };\n");
  const lib = await import(join(dir, "lib.js"));
  const cursor = lib.empty();
  expect(cursor).toEqual({ current: null });
  cursor.current = 4;
  expect([lib.reset(cursor), cursor.current]).toEqual([4, null]);
  const dependency = lib.madeOf(undefined);
  expect([dependency.memoizedValue, lib.read(lib.madeOf(2))]).toEqual([null, 2]);
  lib.cleared(dependency);
  expect([dependency.memoizedValue, lib.read(dependency), lib.numbers().memoizedValue]).toEqual([null, undefined, 1]);
});
