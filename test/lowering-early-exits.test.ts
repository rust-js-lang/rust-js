// What the compiler writes of an early exit the program wrote, as it wrote
// it: a loop's `break` in the `if` it's in, not an `else` of the rest
// ending in a `continue`, nor an early `continue` the program never wrote
// (ADR 0730). Each test is a program of its own, compiled and run.

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

// React's addObjectDiffToProperties (shared/ReactPerformanceTrackProperties.js):
// `if (checked > OBJECT_WIDTH_LIMIT) { ..; break; }` then the rest of the
// body. MIR has the rest as the `if`'s `else`, ending in a `continue`, and
// the `break` after it; and the rest's own `if (!(key in next))` is as
// written, not `if (key in next) { continue; }`.
test("a loop's early break is in the if the program wrote it in", async () => {
  const { js, lib } = await compiled("early-breaks-in-then", `const LIMIT: usize = 3;

pub fn checked(keys: &[String], next: &[String], log: &mut Vec<String>) -> bool {
    let mut equal = true;
    for (checked, key) in keys.iter().enumerate() {
        if checked > LIMIT {
            log.push("too many".to_string());
            equal = false;
            break;
        }

        if !next.contains(key) {
            log.push(key.clone());
            equal = false;
        }
    }

    let mut next_checked = 0;
    for key in next {
        if next_checked > LIMIT {
            log.push("too many".to_string());
            equal = false;
            break;
        }
        if keys.contains(key) {
            log.push(key.clone());
            if key.is_empty() {
                continue;
            }
        } else {
            equal = false;
        }
        next_checked += 1;
    }
    equal
}
`);
  expect(js).toContain('    if (checked > LIMIT) {\n      log.push("too many");\n      equal = false;\n      break;\n    }\n');
  expect(js).toContain("    if (!next.includes(key)) {\n      log.push(key);\n      equal = false;\n    }\n  }\n");
  expect(js).toContain('    if (nextChecked > LIMIT) {\n      log.push("too many");\n      equal = false;\n      break;\n    }\n    if (keys.includes(key)) {\n');
  // Its count is bounded by the check before it, so needs no wrap (ADR 0720).
  expect(js).toContain("    nextChecked++;\n  }\n");
  expect(js).not.toContain("} else {\n      if (");
  const results = [
    [["a", "b"], ["a", "c"]],
    [["a", "b", "c", "d", "e", "f"], ["a", "", "b", "c", "d", "e", "f"]],
  ].map(([keys, next]) => {
    const log: string[] = [];
    return [lib.checked(keys as never, next as never, log as never), log];
  });
  expect(results).toEqual([
    [false, ["b", "a"]],
    [false, ["too many", "a", "b", "c", "too many"]],
  ]);
});

// ADR 0745: React's addObjectDiffToProperties
// (shared/ReactPerformanceTrackProperties.js): each `continue` the program
// wrote in its chain of `if`s, `if (indent === 0 && key === "children") {
// ..; continue; }`, is a `continue` where it's written; an `if` it ends,
// `if (!nested) { .. } else if (..) { .. } continue;`, keeps its `else`;
// and no block MIR made is left, `bb294: { .. break bb294; .. } continue;`.
test("a continue the program wrote in a chain is where it wrote it", async () => {
  const { js, lib } = await compiled("continues-in-chains", `use js::{Kind, Unknown, function};

fn diff_nested(prev: &Unknown, next: &Unknown, out: &mut Vec<String>) -> bool {
    js::get(prev, "a") == js::get(next, "a") || {
        out.push("nested".to_string());
        false
    }
}

pub fn diff(prev: &Unknown, next: &Unknown, out: &mut Vec<String>, indent: usize) -> bool {
    let mut equal = true;
    let mut checked = 0;
    for key in js::keys_in(next) {
        if checked > 100 {
            equal = false;
            break;
        }

        if js::has(prev, &key) {
            let prev_value = js::get(prev, &key);
            let next_value = js::get(next, &key);
            if prev_value != next_value {
                if indent == 0 && key == "children" {
                    // The top level children prop.
                    out.push(key.clone() + " children");
                    equal = false;
                    continue;
                }
                if indent >= 3 {
                    // Deep: the two values.
                } else if let Some(prev_object) = prev_value
                    && let Some(next_object) = next_value
                    && let Kind::Object(_) = js::classify(prev_value)
                    && let Kind::Object(_) = js::classify(next_value)
                {
                    if js::has(next_object, "type") {
                        if js::get(prev_object, "type") == js::get(next_object, "type")
                            && js::get(prev_object, "key") == js::get(next_object, "key")
                        {
                            // Same element.
                            out.push(key.clone() + " element");
                            equal = false;
                            continue;
                        }
                    } else {
                        let before = out.len();
                        if !diff_nested(prev_object, next_object, out) {
                            equal = false;
                        } else if before == out.len() {
                            // Deeply equal.
                            out.push(key.clone() + " deeply equal");
                        }
                        continue;
                    }
                } else if let Kind::Function(prev_function) = js::classify(prev_value)
                    && let Kind::Function(next_function) = js::classify(next_value)
                    && function::name(prev_function) == function::name(next_function)
                {
                    // Same function.
                    out.push(key.clone() + " function");
                    continue;
                }

                // Otherwise, the change.
                out.push(key.clone() + " changed");
                equal = false;
            }
        } else {
            out.push(key + " added");
            equal = false;
        }

        checked += 1;
    }
    equal
}
`);
  expect(js).not.toMatch(/\bbb\d+/);
  expect(js).toContain("        if (indent >= 3) {\n          // Deep: the two values.\n        } else if (\n");
  expect(js).toContain('        if (indent === 0 && key === "children") {\n          // The top level children prop.\n          out.push(key + " children");\n          equal = false;\n          continue;\n        }\n');
  expect(js).toContain("            if (!diffNested(prevValue, nextValue, out)) {\n              equal = false;\n            } else if (before === out.length) {\n");
  expect(js).toContain('              out.push(key + " deeply equal");\n            }\n            continue;\n          }\n        } else if (\n');
  expect(js).toContain('          // Same function.\n          out.push(key + " function");\n          continue;\n        }\n\n        // Otherwise, the change.\n');
  const f = () => 1;
  const g = () => 2;
  const runs = [
    [{ children: 1, x: 1 }, { children: 2, x: 1, y: 2 }, 0],
    [{ e: { type: "div", key: 1 }, o: { a: 1 }, p: { a: 1 } }, { e: { type: "div", key: 1 }, o: { a: 2 }, p: { a: 1 } }, 0],
    [{ f, children: 1 }, { f: g, children: 2 }, 1],
    [{ o: { a: 1 } }, { o: { a: 2 } }, 3],
  ].map(([prev, next, indent]) => {
    const out: string[] = [];
    return [lib.diff(prev as never, next as never, out as never, indent as never), out];
  });
  expect(runs).toEqual([
    [false, ["children children", "y added"]],
    [false, ["e element", "nested", "p deeply equal"]],
    [false, ["f function", "children changed"]],
    [false, ["o changed"]],
  ]);
});

// React's enqueueTask (shared/enqueueTask.js), as react-in-rust ports it:
// its last statement an `if let` with no `else`, in the function
// `js::export_default!` holds. MIR's `return` a macro's function ends with
// is where the macro is called, before its arm, so the arm is made the
// `if`'s `else` and a `return;` its `then`: once each branch's `return;`
// the program didn't write is gone, the emptied `then` is no branch, `if
// (enqueueTaskImpl) { .. }`, not `if (!enqueueTaskImpl) {} else { .. }`.
test("an if let a macro's function ends with is no empty then and an else", async () => {
  const { js, lib } = await compiled("macro-tail-if-let", `use std::cell::Cell;

type EnqueueTask = fn(&'static dyn Fn());

thread_local! {
    static ENQUEUE_TASK_IMPL: Cell<Option<EnqueueTask>> = const { Cell::new(None) };
}

pub fn set_enqueue_task_impl(f: EnqueueTask) {
    ENQUEUE_TASK_IMPL.set(Some(f));
}

js::export_default! {
    fn enqueue_task(task: &'static dyn Fn()) {
        if let Some(enqueue_task_impl) = ENQUEUE_TASK_IMPL.get() {
            enqueue_task_impl(task);
        }
    }
}
`);
  expect(js).toContain("  if (enqueueTaskImpl != null) {\n    enqueueTaskImpl(task);\n  }\n}\n");
  expect(js).not.toMatch(/\{\n\s*\} else \{/);
  const enqueueTask = lib.default as (task: () => void) => void;
  const ran: string[] = [];
  const task = () => ran.push("task");
  enqueueTask(task);
  expect(ran).toEqual([]);
  lib.setEnqueueTaskImpl(((t: () => void) => {
    ran.push("enqueued");
    t();
  }) as never);
  enqueueTask(task);
  expect(ran).toEqual(["enqueued", "task"]);
});

// React's getComponentNameFromFiber, ported: `let return_fiber =
// fiber.r#return.borrow().clone()?;`, as clippy's question_mark asks. What
// `?` tries is read once, as Rust reads it, by its `let`'s name, `const
// returnFiber = fiber.return; if (returnFiber == null) { return null; }`;
// not `if (fiber.return != null) { const returnFiber = fiber.return; .. }`,
// which reads it twice (ADR 0035, amended). A variable is tested in place.
test("a place a question mark tries is read once", async () => {
  const { js, lib } = await compiled("tried-read-once", `use std::cell::RefCell;
use std::rc::Rc;

pub const DEV: bool = true;

pub struct Fiber {
    pub depth: u32,
    pub parent: RefCell<Option<Rc<Fiber>>>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn parent_depth(fiber: &Fiber) -> Option<u32> {
    if DEV {
        if fiber.depth > 3 {
            return Some(3);
        }
        let parent = fiber.parent.borrow().clone()?;
        return Some(parent.depth);
    }
    None
}

pub fn doubled(o: Option<u32>) -> Option<u32> {
    let v = o?;
    Some(v * 2)
}
`);
  expect(js).toContain("    const parent = fiber.parent;\n    if (parent == null) {\n      return null;\n    }\n    return parent.depth;\n");
  expect(js).toContain("  if (o == null) {\n    return;\n  }\n  return (o * 2) >>> 0;\n");
  const root = { depth: 0, parent: null };
  expect([lib.parentDepth({ depth: 1, parent: root } as never), lib.parentDepth({ depth: 1, parent: null } as never), lib.parentDepth({ depth: 5, parent: null } as never), lib.doubled(4 as never), lib.doubled(undefined as never)]).toEqual([0, null, 3, 8, undefined]);
});

// React's getCurrentFiberStackInDev, ported: `if __DEV__ { let Some(current)
// = CURRENT.with_borrow(..) else { return String::new(); }; // Safe ..
// return get_owner_stack_by_fiber_in_dev(&current); } String::new()`. A
// written `let … else` is a guard, first, as upstream's `if (current ===
// null) { return ''; }`: not `if (current != null) { .. }` with its exit
// after, nor gone into the function's tail it's alike (ADR 0879, amended).
test("a let else before a return is a guard first", async () => {
  const { js, lib } = await compiled("let-else-guard-first", `use std::cell::RefCell;

thread_local! {
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn set_current(name: Option<String>) {
    CURRENT.set(name);
}

pub fn current_stack(dev: bool) -> String {
    if dev {
        let Some(current) = CURRENT.with_borrow(|current| current.clone()) else {
            return String::new();
        };
        // The stack of what's current.
        return current + "!";
    }
    String::new()
}

pub fn named(name: Option<String>) -> String {
    let Some(name) = name else {
        return String::new();
    };
    // Named.
    return name + "?";
}

/// A \`?\`'s guard is first already, its rest's \`let\` written before it.
fn parse(s: &str) -> Result<u32, String> {
    s.parse().map_err(|_| s.to_string())
}

pub fn tried(a: &str, b: &str) -> Result<u32, Big> {
    let x = parse(a)?;
    let y = parse(b)?;
    if x + y > 4 {
        return Err(Big(x + y));
    }
    Ok(x + y)
}

pub struct Big(pub u32);

impl From<String> for Big {
    fn from(_: String) -> Self {
        Big(0)
    }
}
`);
  expect(js).toContain('  if (dev) {\n    if (current == null) {\n      return "";\n    }\n    // The stack of what\'s current.\n    return current + "!";\n  }\n  return "";\n');
  expect(js).toContain('  if (name == null) {\n    return "";\n  }\n  // Named.\n  return name + "?";\n');
  expect(js).toMatch(/export function tried\(a, b\) \{\n  const result = parse\(a\);\n  if \(result\.TAG === "Err"\) \{\n/);
  lib.setCurrent("a" as never);
  const set = [lib.currentStack(true as never), lib.currentStack(false as never)];
  lib.setCurrent(undefined as never);
  expect([...set, lib.currentStack(true as never), lib.named("b" as never), lib.named(undefined as never)]).toEqual(["a!", "", "", "b?", ""]);
});

// ADR 0702, amended: React's getHostSibling, `siblings: while (true) { ..
// while (node.tag !== HostComponent && node.tag !== HostText) { if
// (isScope(node)) continue siblings; .. } if (!(node.flags & Placement))
// return node.stateNode; }`. The `while` the program wrote is a `while`,
// the `break` MIR wrote after its test falling out to what follows it;
// each `continue 'siblings` is where the program wrote it, not a `break`
// past a block to MIR's one; and the `if` after the loop keeps its body.
test("a while's continues of the loop around are where the program wrote them", async () => {
  const { js, lib } = await compiled(
    "host-sibling",
    `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    pub tag: Cell<u8>,
    pub flags: Cell<i32>,
    pub child: RefCell<Option<Rc<Fiber>>>,
    pub sibling: RefCell<Option<Rc<Fiber>>>,
    pub r#return: RefCell<Option<Rc<Fiber>>>,
    pub state_node: Cell<u32>,
}

pub const HOST_COMPONENT: u8 = 5;
pub const HOST_TEXT: u8 = 6;
pub const HOST_PORTAL: u8 = 4;
pub const PLACEMENT: i32 = 2;

fn is_scope(fiber: &Fiber) -> bool {
    fiber.tag.get() == 21
}

pub fn get_host_sibling(fiber: &Rc<Fiber>) -> Option<u32> {
    let mut node = Rc::clone(fiber);
    'siblings: loop {
        while node.sibling.borrow().is_none() {
            let return_fiber = node.r#return.borrow().clone();
            let Some(return_fiber) = return_fiber else {
                return None;
            };
            node = return_fiber;
        }
        let sibling = node.sibling.borrow().clone().unwrap();
        node = sibling;
        while node.tag.get() != HOST_COMPONENT && node.tag.get() != HOST_TEXT {
            if is_scope(&node) {
                continue 'siblings;
            }
            if (node.flags.get() & PLACEMENT) != 0 {
                continue 'siblings;
            }
            let child = node.child.borrow().clone();
            if let Some(child) = child
                && node.tag.get() != HOST_PORTAL
            {
                child.r#return.replace(Some(Rc::clone(&node)));
                node = child;
            } else {
                continue 'siblings;
            }
        }
        if (node.flags.get() & PLACEMENT) == 0 {
            return Some(node.state_node.get());
        }
    }
}

// A \`loop\` the program wrote stays one, \`while (true)\`.
pub fn settle(mut n: u32) -> u32 {
    loop {
        if n % 2 == 1 {
            n += 3;
            continue;
        }
        if n > 10 {
            break;
        }
        return n;
    }
    n * 2
}
`,
  );
  expect(js).toContain(`    while (node.tag !== HOST_COMPONENT && node.tag !== HOST_TEXT) {
      if (isScope(node)) {
        continue siblings;
      }
      if (node.flags & PLACEMENT) {
        continue siblings;
      }
`);
  expect(js).toContain("      continue siblings;\n    }\n    if (!(node.flags & PLACEMENT)) {\n      return node.stateNode;\n    }\n  }\n}");
  const fiber = (tag: number, stateNode: number, flags = 0): Record<string, unknown> => ({ tag, flags, child: undefined, sibling: undefined, return: undefined, stateNode });
  // A parent of: a placed host, a scope, a portal of a host, a wrapper of a host, a host.
  const parent = fiber(3, 0);
  const inner = fiber(6, 4);
  const wrapper = fiber(0, 3);
  wrapper.child = inner;
  const portal = fiber(4, 5);
  portal.child = fiber(5, 6);
  const first = fiber(5, 0);
  const last = fiber(5, 7);
  const chain = [first, fiber(5, 1, 2), fiber(21, 2), portal, wrapper, last];
  chain.forEach((f, i) => {
    f.return = parent;
    f.sibling = chain[i + 1];
  });
  expect([lib.getHostSibling(first as never), lib.getHostSibling(portal as never), lib.getHostSibling(last as never)]).toEqual([4, 4, undefined]);
  expect(inner.return).toBe(wrapper);
  expect(js).toContain("export function settle(n) {\n  while (true) {\n");
  expect([1, 4, 9, 12].map((n) => lib.settle(n as never))).toEqual([4, 4, 24, 24]);
});

// React's findCurrentFiberUsingSlowPath: `const parentA = a.return; if
// (parentA === null) { // We're at the root. break; }`, and a second guard
// whose `else` goes on before it breaks. MIR joins both `else`s' `break`
// into one after them, so each guard was nested in the one before, and its
// comments were before the loop. Each `let .. else` is its guard first, its
// `else`'s `break` its own, the comment before it (ADR 0879, amended).
test("let elses that each break a loop are each a guard first", async () => {
  const { js, lib } = await compiled("let-elses-break", `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub r#return: RefCell<Option<Rc<Fiber>>>,
    pub alternate: RefCell<Option<Rc<Fiber>>>,
}

pub fn fiber(tag: u8, parent: Option<Rc<Fiber>>, alternate: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag,
        r#return: RefCell::new(parent),
        alternate: RefCell::new(alternate),
    })
}

pub fn root_of(fiber: &Rc<Fiber>) -> u8 {
    let mut a = Rc::clone(fiber);
    loop {
        let parent_a = a.r#return.borrow().clone();
        let Some(parent_a) = parent_a else {
            // We're at the root.
            break;
        };
        let parent_b = parent_a.alternate.borrow().clone();
        let Some(_parent_b) = parent_b else {
            let next_parent = parent_a.r#return.borrow().clone();
            if let Some(next_parent) = next_parent {
                a = next_parent;
                continue;
            }
            // If there's no parent, we're at the root.
            break;
        };
        a = parent_a;
    }
    a.tag
}
`);
  expect(js).toContain(`    const parentA = a.return;
    if (parentA == null) {
      // We're at the root.
      break;
    }
    const parentB = parentA.alternate;
    if (parentB == null) {
      const nextParent = parentA.return;
      if (nextParent != null) {
        a = nextParent;
        continue;
      }
      // If there's no parent, we're at the root.
      break;
    }
    a = parentA;
  }`);
  const none = undefined as never;
  const root = lib.fiber(3 as never, none, none) as never;
  const other = lib.fiber(7 as never, none, none) as never;
  const middle = lib.fiber(1 as never, root, other) as never;
  const loose = lib.fiber(1 as never, middle, none) as never;
  const lone = lib.fiber(5 as never, lib.fiber(9 as never, none, none) as never, none) as never;
  const climbed = lib.fiber(2 as never, lib.fiber(4 as never, root, none) as never, none) as never;
  expect([lib.rootOf(root), lib.rootOf(middle), lib.rootOf(loose), lib.rootOf(lone), lib.rootOf(climbed)]).toEqual([
    3, 1, 1, 5, 3,
  ]);
});

// React's findCurrentFiberUsingSlowPath: `while (child) { if (child === a) {
// ..; break; } if (child === b) { ..; break; } child = child.sibling; }`.
// MIR joins the two `break`s after an `if` chain, `if (..) { .. } else if
// (..) { .. } else { child = ..; continue; } break;`: each `if` the
// program wrote without an `else` ends with the `break` it wrote (ADR
// 0879, amended).
test("ifs that each break a loop are each written with their break", async () => {
  const { js, lib } = await compiled("ifs-break-loop", `use std::rc::Rc;

pub struct Node {
    pub sibling: Option<Rc<Node>>,
}

pub fn list(n: u32) -> Option<Rc<Node>> {
    (0..n).fold(None, |sibling, _| Some(Rc::new(Node { sibling })))
}

pub fn which(first: Option<Rc<Node>>, a: &Rc<Node>, b: &Rc<Node>) -> u8 {
    let mut found = 0;
    let mut child = first;
    while let Some(current) = child {
        if Rc::ptr_eq(&current, a) {
            found = 1;
            break;
        }
        if Rc::ptr_eq(&current, b) {
            found = 2;
            break;
        }
        child = current.sibling.clone();
    }
    found
}

pub fn chained(first: Option<Rc<Node>>, a: &Rc<Node>, b: &Rc<Node>) -> u8 {
    let mut found = 0;
    let mut child = first;
    while let Some(current) = child {
        if Rc::ptr_eq(&current, a) {
            found = 1;
        } else if Rc::ptr_eq(&current, b) {
            found = 2;
        } else {
            child = current.sibling.clone();
            continue;
        }
        break;
    }
    found
}
`);
  // An `else` the program wrote stays.
  expect(js).toContain("      found = 1;\n    } else if (child === b) {\n");
  expect(js).toContain(`  while (child != null) {
    if (child === a) {
      found = 1;
      break;
    }
    if (child === b) {
      found = 2;
      break;
    }
    child = child.sibling;
  }`);
  const nodes = lib.list(3 as never) as { sibling: unknown };
  const second = nodes.sibling as { sibling: unknown };
  const lone = lib.list(1 as never);
  expect([lib.which(nodes as never, second as never, nodes as never), lib.which(nodes as never, lone as never, second as never), lib.which(nodes as never, lone as never, lone as never), lib.chained(nodes as never, second as never, nodes as never)]).toEqual([2, 2, 0, 2]);
});

// React's appendAllChildren walks a tree, `while (node !== null) { .. if
// (node === workInProgress) { return; } while (node.sibling === null) { if
// (node.return === null || node.return === workInProgress) { return; } node
// = node.return; } .. }`. MIR joins its `return`s into one past a block the
// inner loop leaves, `bb6: while (..) { .. while (true) { .. break; }
// continue bb6; } return;`: each is the `return` written, no label (ADR
// 0742, amended).
test("a loop's early return MIR joined past a block is a return", async () => {
  const { js, lib } = await compiled("loop-returns-past-block", `use std::cell::RefCell;
use std::rc::Rc;

pub struct Node {
    pub id: u32,
    pub child: RefCell<Option<Rc<Node>>>,
    pub sibling: RefCell<Option<Rc<Node>>>,
    pub r#return: RefCell<Option<Rc<Node>>>,
}

pub fn node(id: u32) -> Rc<Node> {
    Rc::new(Node { id, child: RefCell::new(None), sibling: RefCell::new(None), r#return: RefCell::new(None) })
}

pub fn adopt(parent: &Rc<Node>, children: Vec<Rc<Node>>) {
    let mut last: Option<Rc<Node>> = None;
    for child in children {
        child.r#return.replace(Some(Rc::clone(parent)));
        match &last {
            Some(last) => _ = last.sibling.replace(Some(Rc::clone(&child))),
            None => _ = parent.child.replace(Some(Rc::clone(&child))),
        }
        last = Some(child);
    }
}

pub fn walk(work_in_progress: &Rc<Node>, log: &mut Vec<u32>) {
    let mut node = work_in_progress.child.borrow().clone();
    while let Some(mut current) = node {
        log.push(current.id);
        if Rc::ptr_eq(&current, work_in_progress) {
            return;
        }
        while current.sibling.borrow().is_none() {
            let return_fiber = current.r#return.borrow().clone();
            match return_fiber {
                Some(return_fiber) if !Rc::ptr_eq(&return_fiber, work_in_progress) => current = return_fiber,
                _ => return,
            }
        }
        let sibling = current.sibling.borrow().clone();
        node = sibling;
    }
}
`);
  expect(js).not.toMatch(/\bbb\d+/);
  expect(js).toContain("    if (current === workInProgress) {\n      return;\n    }\n    while (current.sibling == null) {\n");
  const root = lib.node(0 as never);
  const [a, b, c] = [1, 2, 3].map((id) => lib.node(id as never));
  lib.adopt(root as never, [a, b] as never);
  lib.adopt(b as never, [c] as never);
  const log: number[] = [];
  lib.walk(root as never, log as never);
  expect(log).toEqual([1, 2]);
});

// React's recordLegacyContextWarning, `const strictRoot =
// findStrictRoot(fiber); if (strictRoot === null) { console.error(..);
// return; } if (didWarn.has(fiber.type)) { return; } ..`, of a unit
// function whose `return`s MIR joins to where it ends, which printed `if
// (strictRoot == null) { .. } else if (!didWarn.has(..)) { .. }`: each a
// guard first, as written, the `return;` the function ends with in it
// (ADR 0879, amended).
test("a unit function's early returns joined to its end are each a guard first", async () => {
  const { js, lib } = await compiled("early-returns-at-the-end", `use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub root: Option<Rc<Fiber>>,
}

fn find_root(fiber: &Fiber) -> Option<Rc<Fiber>> {
    fiber.root.clone()
}

pub fn record(fiber: &Fiber, log: &mut Vec<u8>) {
    let root = find_root(fiber);
    let Some(root) = root else {
        log.push(0);
        return;
    };
    if fiber.tag == 3 {
        return;
    }
    if root.tag > 2 {
        log.push(root.tag);
    }
}

pub fn record_within(fiber: &Fiber, log: &mut Vec<u8>) {
    let root = find_root(fiber);
    if fiber.tag != 3 {
        if root.is_none() {
            return;
        }
        log.push(fiber.tag);
    }
}

pub fn fiber(tag: u8, root: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { tag, root })
}
`);
  expect(js).toContain(
    "  if (root == null) {\n    log.push(0);\n    return;\n  }\n  if (fiber.tag === 3) {\n    return;\n  }\n  if (root.tag > 2) {\n",
  );
  const log: number[] = [];
  const [two, five] = [2, 5].map((tag) => lib.fiber(tag as never, null as never));
  for (const [tag, root] of [
    [1, null],
    [3, five],
    [1, five],
    [1, two],
  ] as const) {
    lib.record(lib.fiber(tag as never, root as never) as never, log as never);
    lib.recordWithin(lib.fiber(tag as never, root as never) as never, log as never);
  }
  expect(log).toEqual([0, 5, 1, 1]);
  // Not an `if` whose `then` the program wrote with the rest in it.
  expect(js).toContain("  if (fiber.tag !== 3) {\n    if (root == null) {\n      return;\n    }\n    log.push(fiber.tag);\n  }\n");
});

// ADR 0742, amended (#438): a unit function's guards MIR joins to its end,
// each `return;` as written. `if did_warn.contains(..) { return; }`, a test
// of a call's value, which says no place of its own, is React's
// recordLegacyContextWarning's `if (didWarnAboutLegacyContext.has(..)) {
// return; }`: its `return` is the one after where the `if` is written,
// not `if (!didWarn.has(..)) { rest }`. And diffHydratedStyles' two guards,
// `if a == b { return; } if c && d { // .. return; }`, MIR's block that the
// second leaves as well as the first's test: the `return` after it neither
// guard's, each as upstream writes it with its comment, not `if (a === b ||
// (c && d)) { return; }`.
test("a unit function's written guards joined to its end are each where written", async () => {
  const { js, lib } = await compiled("joined-guards-as-written", `use std::collections::HashSet;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u8,
    pub root: Option<Rc<Fiber>>,
}

fn find_root(fiber: &Fiber) -> Option<Rc<Fiber>> {
    fiber.root.clone()
}

pub fn record(fiber: &Fiber, did_warn: &mut HashSet<u8>, log: &mut Vec<u8>) {
    let root = find_root(fiber);
    if root.is_none() {
        log.push(0);
        return;
    }
    if did_warn.contains(&fiber.tag) {
        return;
    }
    if fiber.tag > 2 {
        log.push(fiber.tag);
        did_warn.insert(fiber.tag);
    }
}

pub fn fiber(tag: u8, root: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber { tag, root })
}

fn normalize(s: Option<&str>) -> String {
    s.unwrap_or("").trim().to_string()
}

fn has_transition(n: &u32) -> bool {
    *n > 3 || *n == 0
}

pub fn diff_styles(value: Option<u32>, client: Option<String>, server: Option<String>, n: &u32, out: &mut Vec<String>) {
    if value.is_some() && !matches!(value, Some(1 | 2)) {
        if n > &5 {
            out.push("expects a mapping".to_string());
        }
        return;
    }
    if server == client {
        return;
    }
    let normalized_client = normalize(client.as_deref());
    let normalized_server = normalize(server.as_deref());
    if normalized_server == normalized_client {
        return;
    }

    if js::string::get(&normalized_server, f64::from(js::string::length(&normalized_server)) - 1.0) == Some(";")
        && has_transition(n)
    {
        // A view transition's style.
        return;
    }

    out.push(normalized_server);
}
`);
  expect(js).toContain(
    "  if (root == null) {\n    log.push(0);\n    return;\n  }\n  if (didWarn.has(fiber.tag)) {\n    return;\n  }\n  if (fiber.tag > 2) {\n",
  );
  expect(js).toContain(
    '  if (normalizedServer === normalizedClient) {\n    return;\n  }\n\n  if (normalizedServer[normalizedServer.length - 1] === ";" && hasTransition(n)) {\n    // A view transition\'s style.\n    return;\n  }\n\n  out.push(normalizedServer);\n}\n',
  );
  const log: number[] = [];
  const warned = new Set<number>([4]);
  const root = lib.fiber(9 as never, null as never);
  for (const [tag, rooted] of [
    [1, false],
    [4, true],
    [3, true],
    [3, true],
    [1, true],
  ] as const) {
    lib.record(lib.fiber(tag as never, (rooted ? root : null) as never) as never, warned as never, log as never);
  }
  expect(log).toEqual([0, 3]);
  const out: string[] = [];
  for (const [value, client, server, n] of [
    [7, "a", "b", 6],
    [null, "a", "a", 4],
    [null, " a ", "a", 4],
    [null, "a", "b;", 4],
    [null, "a", "b;", 2],
    [1, "a", "b", 0],
  ] as const) {
    lib.diffStyles(value as never, client as never, server as never, n as never, out as never);
  }
  expect(out).toEqual(["expects a mapping", "b;", "b"]);
});

// ADR 0879, amended (#437): React's workLoopConcurrent, `if let Some(mut
// w) = WORK.with_borrow(Clone::clone) { ..; loop { .. } }`, a unit function
// that goes on to its end: the `return;` MIR made at the end of its `if`'s
// branch is nothing, as falling off the end, `if (value != null) { ..; do {
// .. } while (..); }`, as upstream writes it.
test("a unit function's last if has no return MIR made at its end", async () => {
  const { js, lib } = await compiled("last-if-no-return", `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub tag: u32,
    pub next: Option<Rc<Fiber>>,
}

thread_local! {
    static WORK: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
}

fn perform(fiber: &Rc<Fiber>, log: &mut Vec<u32>) {
    log.push(fiber.tag);
    WORK.set(fiber.next.clone());
}

pub fn work_loop(budget: u32, log: &mut Vec<u32>) {
    if let Some(mut work) = WORK.with_borrow(Clone::clone) {
        let until = log.len() as u32 + budget;
        loop {
            perform(&work, log);
            match WORK.with_borrow(Clone::clone) {
                Some(next) if (log.len() as u32) < until => work = next,
                _ => break,
            }
        }
    }
}

pub fn start(tags: Vec<u32>) {
    let mut next = None;
    for tag in tags.into_iter().rev() {
        next = Some(Rc::new(Fiber { tag, next }));
    }
    WORK.set(next);
}
`);
  expect(js).toContain("    } while (work$1 != null && log.length < until);\n  }\n}\n");
  const log: number[] = [];
  lib.workLoop(2 as never, log as never);
  lib.start([1, 2, 3, 4, 5] as never);
  lib.workLoop(2 as never, log as never);
  lib.workLoop(9 as never, log as never);
  expect(log).toEqual([1, 2, 3, 4, 5]);
});

// React's commitDeletedPairViewTransitions, `while (child !== null) { ..
// if (pairs.size === 0) { break; } .. }`, the loop its function ends with:
// MIR's jump of the `break` goes to the function's end, which printed
// `return;`. The `break` the program wrote, where it wrote it (ADR 0684,
// amended); a `return` it wrote stays.
test("a break of the loop a function ends with is a break", async () => {
  const { js, lib } = await compiled("break-of-last-loop", `pub fn find(items: &[u32], wanted: &mut Vec<u32>, log: &mut Vec<u32>) {
    for &item in items {
        if let Some(index) = wanted.iter().position(|&w| w == item) {
            wanted.remove(index);
            if wanted.is_empty() {
                break;
            }
        }
        if item == 0 {
            return;
        }
        log.push(item);
    }
}
`);
  expect(js).toContain("      if (wanted.length === 0) {\n        break;\n      }\n");
  expect(js).toContain("    if (item === 0) {\n      return;\n    }\n");
  const [wanted, log]: number[][] = [[2, 3], []];
  lib.find([1, 2, 4, 3, 5] as never, wanted as never, log as never);
  lib.find([6, 0, 7] as never, [9] as never, log as never);
  expect(log).toEqual([1, 2, 4, 6]);
});

// rd73: a let-else whose else returns, the first of a block that ends the
// function, `if (DEV) { let Some(next) = .. else { return; }; .. }`: the
// guard, then the block's statements, no `return;` MIR made at their end
// (CSSPropertyOperations' validateShorthandPropertyCollisionInDev).
test("a let-else's return in a block that ends the function leaves no return at its end", async () => {
  const { js, lib } = await compiled("let-else-ending-block", `pub const DEV: bool = true;

pub fn check(next: Option<&Vec<u32>>, log: &mut Vec<u32>) {
    if DEV {
        let Some(next) = next.filter(|next| !next.is_empty()) else {
            return;
        };
        for key in next {
            log.push(*key);
        }
    }
}`);
  expect(js).not.toMatch(/\n    return;\n  }\n}/);
  const log: number[] = [];
  lib.check([1, 2] as never, log as never);
  lib.check([] as never, log as never);
  lib.check(undefined as never, log as never);
  expect(log).toEqual([1, 2]);
});
