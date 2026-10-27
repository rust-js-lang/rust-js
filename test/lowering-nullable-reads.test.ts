// What the compiler writes of an `Option` read from a place that holds its
// `None` as `null` (ADR 0949): a `#[rust_js::nullable]` field's or
// thread-local's, which is `null` or a value already, so a nullable
// function that returns it returns it as it is, no `?? null`. Each test is
// a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// React's getSuspenseInstanceFromFiber, `return suspenseState.dehydrated;`,
// and popSearchTarget, `return searchTarget;`.
test("a nullable place's read is returned as it is", async () => {
  const dir = fixture("nullable-reads");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

use js::Unknown;

pub struct SuspenseState {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub dehydrated: Option<&'static Unknown>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn dehydrated(memoized_state: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let suspense_state: Option<&SuspenseState> = unsafe { js::cast(memoized_state) };
    if let Some(suspense_state) = suspense_state {
        return suspense_state.dehydrated;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn current() -> Option<&'static Unknown> {
    CURRENT.get()
}

pub fn set_current(value: Option<&'static Unknown>) {
    CURRENT.set(value);
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (memoizedState) {\n    return memoizedState.dehydrated;\n  }\n  return null;\n");
  expect(js).toContain("export function current() {\n  return CURRENT;\n}");
  expect(js).toContain("  CURRENT = value ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const one = {};
  expect([lib.dehydrated({ dehydrated: one }), lib.dehydrated({ dehydrated: null }), lib.dehydrated(null), lib.current()]).toEqual([one, null, null, null]);
  lib.setCurrent(undefined);
  expect(lib.current()).toBe(null);
  lib.setCurrent(one);
  expect(lib.current()).toBe(one);
});

// ADR 0957: React's popHydrationStateOnInterruptedWork, `if (fiber.tag ===
// HostComponent && fiber.stateNode != null) { nextHydratableInstance =
// fiber.stateNode; }`: a place a test shows isn't `null`, stored right
// after it, is stored as it is, no `?? null`.
test("a place a test shows present is stored as it is", async () => {
  const dir = fixture("nullable-store-after-test");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

use js::Unknown;

pub struct Fiber {
    pub tag: Cell<u8>,
    pub state_node: Cell<Option<&'static Unknown>>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static NEXT: Cell<Option<&'static Unknown>> = const { Cell::new(None) };
}

pub fn restore(fiber: &Fiber) {
    if fiber.tag.get() == 5 && fiber.state_node.get().is_some() {
        NEXT.set(fiber.state_node.get());
    }
}

pub fn restore_or(fiber: &Fiber) {
    if fiber.tag.get() == 7 || fiber.state_node.get().is_some() {
        NEXT.set(fiber.state_node.get());
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn next() -> Option<&'static Unknown> {
    NEXT.get()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (fiber.tag === 5 && fiber.stateNode != null) {\n    NEXT = fiber.stateNode;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const node = {};
  lib.restore({ tag: 5, stateNode: undefined });
  expect(lib.next()).toBe(null);
  lib.restore({ tag: 5, stateNode: node });
  expect(lib.next()).toBe(node);
  // Of an `||`, where the place may be `undefined`: `?? null` kept.
  lib.restoreOr({ tag: 7, stateNode: undefined });
  expect(lib.next()).toBe(null);
});

// ADR 0957, amended: React's getParentFragmentInstances, `let
// parentFragmentInstances = null; .. return parentFragmentInstances;`: a
// `let` made and set only to what's never `undefined` is returned as it is;
// one a closure may set keeps its `?? null`.
test("a let set only to values is returned as it is", async () => {
  const dir = fixture("nullable-let-returned");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::nullable)]
pub fn collect(items: &[&'static js::Unknown]) -> Option<Vec<&'static js::Unknown>> {
    let mut found = js::null();
    for &item in items {
        if js::truthy(js::get(item, "keep")) {
            match &mut found {
                None => found = Some(vec![item]),
                Some(found) => found.push(item),
            }
        }
    }
    found
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn first(items: &[u32]) -> Option<u32> {
    let mut seen = js::null();
    items.iter().for_each(|&n| {
        if seen.is_none() {
            seen = items.get(n as usize).copied();
        }
    });
    seen
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return found;\n");
  expect(js).toContain("  return seen ?? null;\n");
  const lib = await import(join(dir, "lib.js"));
  const a = { keep: true };
  expect([lib.collect([]), lib.collect([{}, a]), lib.first([5]), lib.first([0])]).toEqual([null, [a], null, 0]);
});
