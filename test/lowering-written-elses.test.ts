// What tidy keeps of an `else` and an `if` the program wrote, past the
// passes that make its test or its chain again: a written `if` in a
// match's last arm stays an `if` of its own, and a test made again of the
// same truth keeps its written `else` (ADRs 0237, 0879, amended). Each test
// is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's markRef: `} else { if (typeof ref !== 'function' && ..) { throw
// .. } if (current === null || current.ref !== ref) { .. } }`, a match's
// last arm, its `else` (ADR 0237, amended), of two `if`s the program wrote
// apart: not `} else if (..) { throw .. } else if (..) { .. }`, the check
// made a link of the chain (`chained_checks`, which takes only a check MIR
// made, an `else if .. else { panic!() }`'s).
test("a written if that throws in a match's last arm stays an if of its own", async () => {
  const { js, lib } = await compiled(
    "written-check-in-last-arm",
    `use std::cell::Cell;

pub struct Fiber {
    pub flags: Cell<i32>,
    pub r#ref: Cell<Option<u32>>,
}

pub fn mark_ref(current: Option<&Fiber>, work_in_progress: &Fiber) {
    match work_in_progress.r#ref.get() {
        None => {
            if current.is_some_and(|current| current.r#ref.get().is_some()) {
                work_in_progress.flags.set(work_in_progress.flags.get() | 1);
            }
        }
        Some(r#ref) => {
            if r#ref > 9 {
                panic!("Expected ref to be a function.");
            }
            if current.is_none_or(|current| current.r#ref.get() != Some(r#ref)) {
                work_in_progress.flags.set(work_in_progress.flags.get() | 2);
            }
        }
    }
}
`,
  );
  expect(js).toContain(
    '  } else {\n    if (ref > 9) {\n      throw new Error("Expected ref to be a function.");\n    }\n    if (current == null || current.ref !== ref) {\n',
  );
  const fiber = (ref: number | undefined) => ({ flags: 0, ref });
  const marked = (current: { ref: number | undefined } | undefined, ref: number | undefined) => {
    const wip = fiber(ref);
    lib.markRef(current as never, wip as never);
    return wip.flags;
  };
  expect([marked(undefined, undefined), marked(fiber(1), undefined), marked(undefined, 3), marked(fiber(3), 3), marked(fiber(4), 3)]).toEqual([0, 1, 2, 0, 2]);
  expect(() => marked(undefined, 10)).toThrow("Expected ref to be a function.");
});

// React's updateHostRoot: `if (workInProgress.flags & ForceClientRender) {
// return ..; } else if (nextChildren !== prevChildren) { ..; return ..; }
// else { .. }`. Its port's `(flags & FORCE_CLIENT_RENDER) != 0`, made the
// truth test `flags & 4` (`bit_tests`), lost what it said of its `if`, its
// written `else`, and printed `} if (next !== prev) {`.
test("a bit test made again keeps its if's written else", async () => {
  const { js, lib } = await compiled(
    "bit-test-keeps-its-else",
    `use std::cell::Cell;

pub struct Fiber {
    pub flags: Cell<i32>,
    pub child: Cell<u32>,
}

fn mount(f: &Fiber, n: u32) -> u32 {
    f.child.set(n);
    n
}

pub fn update_host_root(dehydrated: bool, wip: &Fiber, next: u32, prev: u32) -> u32 {
    if dehydrated {
        if (wip.flags.get() & 4) != 0 {
            // Forced.
            return mount(wip, next + 100);
        } else if next != prev {
            return mount(wip, next);
        } else {
            // Start hydrating.
            wip.child.set(prev + 10);
        }
    } else {
        if next == prev {
            return 7;
        }
        mount(wip, next);
    }
    wip.child.get()
}
`,
  );
  expect(js).toContain("    if (wip.flags & 4) {\n      // Forced.\n      return mount(wip, (next + 100) >>> 0);\n    } else if (next !== prev) {\n");
  const run = (dehydrated: boolean, flags: number, next: number, prev: number) =>
    lib.updateHostRoot(dehydrated as never, { flags, child: 0 } as never, next as never, prev as never);
  expect([run(true, 4, 1, 1), run(true, 0, 1, 2), run(true, 0, 1, 1), run(false, 0, 1, 1), run(false, 0, 1, 2)]).toEqual([101, 1, 11, 7, 1]);
});

// React's findHostInstance: `const hostFiber = findCurrentHostFiber(fiber);
// if (hostFiber === null) { return null; } return
// getPublicInstance(hostFiber.stateNode);`. Its port's `let host_fiber =
// host_fiber?;`, clippy's question_mark, of an `Option<Rc<T>>`, whose drop
// MIR joins the two ways after, printed the guard turned, `if (hostFiber !=
// null) { return ..; } return null;`: a `?`'s way out is a guard's, as a
// `let … else`'s `else` is, first (q1, ADR 0879, amended).
test("a ? of an Option<Rc> is its guard first", async () => {
  const { js, lib } = await compiled(
    "question-guard-first",
    `use std::cell::Cell;
use std::rc::Rc;

pub struct Fiber {
    pub state: Cell<u32>,
}

fn current(fiber: &Rc<Fiber>) -> Option<Rc<Fiber>> {
    (fiber.state.get() > 0).then(|| Rc::clone(fiber))
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn public_instance(state: u32) -> Option<u32> {
    (state > 1).then_some(state * 2)
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn instance(fiber: &Rc<Fiber>) -> Option<u32> {
    let host_fiber = current(fiber);
    let host_fiber = host_fiber?;
    public_instance(host_fiber.state.get())
}
`,
  );
  expect(js).toContain("  const hostFiber = current(fiber);\n  if (hostFiber == null) {\n    return null;\n  }\n  return");
  expect([lib.instance({ state: 0 } as never), lib.instance({ state: 4 } as never)]).toEqual([null, 8]);
  expect(lib.instance({ state: 1 } as never)).toBe(null);
});

// react-art's appendInitialChild: `if (typeof child === 'string') { //
// comment throw new Error(..); } child.inject(parentInstance);`. A match's
// arm that never goes on, a `panic!` as a `return` or a `break` alone, is a
// guard's way out, as `let .. else` writes one: no `else` after it (ADR
// 0237, amended). A `match` of two arms that each go on keeps its `else`.
test("a match arm that panics is a guard, no else after it", async () => {
  const { js, lib } = await compiled(
    "match-arm-panics",
    `pub enum Child {
    Text(String),
    Instance(Vec<u32>),
}

pub fn append_initial_child(parent: &mut Vec<u32>, child: Child) {
    match child {
        Child::Text(_) => {
            // Noop for string children of Text
            panic!("Text children should already be flattened.");
        }
        Child::Instance(child) => {
            parent.extend(child);
        }
    }
}

pub fn first(v: &[u32]) -> u32 {
    match v.first() {
        None => panic!("empty"),
        Some(&x) => x + 1,
    }
}

pub fn logged(v: &[u32], out: &mut Vec<u32>) -> u32 {
    match v.first() {
        None => {
            out.push(0);
            panic!("empty")
        }
        Some(&x) => x,
    }
}

pub fn kept(v: &[u32], out: &mut Vec<u32>) {
    match v.first() {
        None => out.push(0),
        Some(&x) => out.push(x),
    }
}
`,
  );
  expect(js).toContain(
    '  if (child.TAG === "Text") {\n    // Noop for string children of Text\n    throw new Error("Text children should already be flattened.");\n  }\n  for (let i = 0; i < child._0.length; i++) {\n    parent.push(child._0[i]);\n  }\n}\n',
  );
  expect(js).toContain('  if (x == null) {\n    throw new Error("empty");\n  }\n  return (x + 1) >>> 0;\n');
  expect(js).toContain("  if (x == null) {\n    out.push(0);\n  } else {\n    out.push(x);\n  }\n");
  // An arm of more than the panic, as of more than a jump, keeps its `else`.
  expect(js).toContain('    out.push(0);\n    throw new Error("empty");\n  } else {\n');
  const parent: number[] = [];
  lib.appendInitialChild(parent as never, { TAG: "Instance", _0: [1, 2] } as never);
  expect(parent).toEqual([1, 2]);
  expect(() => lib.appendInitialChild(parent as never, { TAG: "Text", _0: "a" } as never)).toThrow("Text children should already be flattened.");
  expect([lib.first([4] as never), (() => { try { return lib.first([] as never); } catch (e) { return (e as Error).message; } })()]).toEqual([5, "empty"]);
});
