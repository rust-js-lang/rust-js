// What the compiler writes of a `let` that shadows a binding: the one JS
// variable where the binding it shadows is read no more, assigned its new
// value (ADR 0650, extending 0609's parameters to every binding), and a
// name of its own where anything could still see the old one. Each test is
// a program of its own, compiled and run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildNode, buildReact, compiler, fixture, run, target } from "./support";

// React's `formatOwnerStack` (packages/shared/ReactOwnerStackFrames.js), as
// its port writes it. Its JS was
//
//   const stack = error.stack ?? '';
//   Error.prepareStackTrace = prevPrepareStackTrace;
//   let stack$1 = stack;
//   if (stack$1.startsWith('Error: react-stack-top-frame\n')) {
//     stack$1 = stack$1.slice(29);
//   }
//   const idx = stack$1.indexOf('\n');
//   ..
//   const idx$1 = at !== -1 ? stack$1.slice(0, at).lastIndexOf('\n') : -1;
//
// where React has one `let stack` and one `let idx`, each assigned again.
test("a let that shadows a binding read no more is that binding, as React's formatOwnerStack", () => {
  const dir = fixture("shadowed-react-owner-stack");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::JsError;
use node::js_error;

pub fn format_owner_stack(error: &JsError) -> String {
    let prev_prepare_stack_trace = js_error::prepare_stack_trace();
    js_error::set_prepare_stack_trace::<String>(None);
    let stack = error.stack().unwrap_or_default();
    js_error::set_prepare_stack_trace(prev_prepare_stack_trace);
    let mut stack = stack.as_str();
    if let Some(rest) = stack.strip_prefix("Error: react-stack-top-frame\\n") {
        stack = rest;
    }
    if let Some(idx) = stack.find('\\n') {
        // Pop the JSX frame.
        stack = &stack[idx + 1..];
    }
    let idx = stack
        .find("react_stack_bottom_frame")
        .and_then(|idx| stack[..idx].rfind('\\n'));
    if let Some(idx) = idx {
        stack = &stack[..idx];
    } else {
        return String::new();
    }
    stack.to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `node=${join(node, "libnode.rmeta")}`, "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`  let stack = error.stack ?? "";
  Error.prepareStackTrace = prevPrepareStackTrace;
  if (stack.startsWith("Error: react-stack-top-frame\\n")) {
    stack = stack.slice(29);
  }
  let idx = stack.indexOf("\\n");
  if (idx !== -1) {
    // Pop the JSX frame.
    stack = stack.slice(idx + 1);
  }
  const at = stack.indexOf("react_stack_bottom_frame");
  idx = at !== -1 ? stack.slice(0, at).lastIndexOf("\\n") : -1;
  if (idx !== -1) {
    stack = stack.slice(0, idx);
  } else {
    return "";
  }
  return stack;
`);
  expect(js).not.toContain("$1");
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "run.js"), `import { formatOwnerStack } from "./lib.js";

function react_stack_bottom_frame(f) { return f(); }
const inner = () => new Error("react-stack-top-frame");
const owner = formatOwnerStack(react_stack_bottom_frame(() => inner()));
const outside = formatOwnerStack(new Error("react-stack-top-frame"));
console.log(JSON.stringify({ frames: owner.split("\\n").length, at: owner.trimStart().startsWith("at "), outside }));
`);
  const p = Bun.spawnSync(["node", "run.js"], { cwd: dir, stderr: "pipe" });
  expect([p.exitCode, p.stderr.toString()]).toEqual([0, ""]);
  // The top frame and the JSX frame cut, and what's below the bottom one.
  expect(JSON.parse(p.stdout.toString())).toEqual({ frames: 1, at: true, outside: "" });
});

// Where anything could still see the old binding, the new one keeps its
// own name: a loop's turn, which reads what's outside it again; a closure
// made before, which reads it when it's called; a `&mut`'s place, and a
// handle on it, which reads and sets it where it's used; and a drop at the
// scope's end. The new value's type is JS's to change, and a
// `for`'s pattern is its block's binding, `let` once it's set.
test("a let that shadows a binding something still sees keeps its own name", async () => {
  const dir = fixture("shadowed-and-seen");
  writeFileSync(join(dir, "lib.rs"), `pub struct Noisy(pub &'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn looped(x: u32) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 0..3 {
        let x = x + i;
        out.push(x);
    }
    out
}

pub fn captured(s: &str) -> usize {
    let n = s.len();
    let f = move || n;
    let n = n * 2 + s.len();
    f() + n
}

pub fn handled(s: &str) -> usize {
    let mut a = s.len();
    let r = &mut a;
    let a = s.len() * 10;
    *r += 1;
    *r + a
}

fn shout<T: AsMut<str>>(mut t: T, n: usize) -> usize {
    let part = t.as_mut();
    let t = n;
    part.make_ascii_uppercase();
    t + part.len()
}

pub fn shouted(s: &str, n: usize) -> usize {
    shout(s.to_string(), n)
}

pub fn dropped() -> &'static str {
    let g = Noisy("a");
    let g = Noisy(if g.0.is_empty() { "c" } else { "b" });
    g.0
}

pub fn retyped(d: u32) -> u32 {
    let text = format!("{d}1");
    let text: u32 = text.parse().unwrap();
    text + 1
}

pub fn trimmed(lines: &[&str]) -> Vec<usize> {
    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        out.push(line.len());
    }
    out
}

pub fn chained(s: &str) -> String {
    let t = s.trim();
    let t = t.to_uppercase();
    t + "!"
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Each turn reads the parameter anew.
  expect(js).toContain("    const x$1 = (x + i) >>> 0;\n    out.push(x$1);\n");
  // Read by a closure made before.
  expect(js).toContain("  const f = () => n;\n  const n$1 = ");
  // Set through the `&mut` after.
  expect(js).toContain("  const a$1 = ($byteLen(s) * 10) >>> 0;\n");
  // Read and set through a handle on the old one after.
  expect(js).toContain("  const t$1 = n;\n  part.value = $asciiCase(part.value, true);\n");
  // Dropped at the scope's end, after the new one.
  expect(js).toContain("  console.log(`drop ${g$1}`);\n  console.log(`drop ${g}`);\n");
  // A number where it was text.
  expect(js).toContain("  let text = `${d}1`;\n  text = $unwrapOk(");
  // The pattern's own, set.
  expect(js).toContain("  for (let line of lines) {\n    line = $trim(line);\n");
  expect(js).toContain("  let t = $trim(s);\n  t = t.toUpperCase();\n  return t + \"!\";\n");
  const lib = await import(join(dir, "lib.js"));
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  let dropped: string;
  try {
    dropped = lib.dropped();
  } finally {
    console.log = log;
  }
  expect([lib.looped(10), lib.captured("ab"), lib.handled("ab"), dropped, logged]).toEqual([
    [10, 11, 12],
    2 + 6,
    3 + 20,
    "b",
    ["drop b", "drop a"],
  ]);
  expect([lib.retyped(4), lib.trimmed([" a ", "bc  "]), lib.chained(" ok "), lib.shouted("ab", 3)]).toEqual([
    42,
    [1, 2],
    "OK!",
    5,
  ]);
});

// React's `cache`: a closure's loop variable shadowed by its cast, `let
// arg: &JsObject = unsafe { js::cast(arg) };`, is that variable, as a
// function's is: nothing outside the closure sets what it declares (ADR
// 0853). Its JS had `const arg$1 = arg;`.
test("a closure's loop variable shadowed by its own value is that variable", () => {
  const dir = fixture("shadowed-closure-loop-variable");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use js::{JsObject, Kind, Unknown, WeakMap};

pub fn keyed(map: &'static WeakMap<JsObject, f64>) -> impl Fn(&[Option<&'static Unknown>]) -> f64 {
    move |arguments: &[Option<&'static Unknown>]| {
        let mut total = 0.0;
        for &arg in arguments {
            if let Kind::Function(_) | Kind::Object(_) | Kind::Array(_) = js::classify(arg) {
                let arg: &JsObject = unsafe { js::cast(arg) };
                seen();
                total += map.get(arg).unwrap_or(1.0);
            }
        }
        total
    }
}

fn seen() {
    println!("seen");
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`      if (typeof arg === "function" || (typeof arg === "object" && arg !== null)) {
        seen();
        total += map.get(arg) ?? 1;
      }`);
  expect(js).not.toContain("arg$1");
}, 300_000);

// React's ReactFiberLane, `markRootFinished`: `if let Some(hidden_updates_for_lane)
// = hidden_updates[index].take()` is upstream's `const hiddenUpdatesForLane =
// hiddenUpdates[index]`, the program's name for what `take()` gives, not
// `old`, rust-js's, which the binding copied (ADR 0598, amended).
test("a binding of what take() gives names it", async () => {
  const dir = fixture("taken-binding-name");
  writeFileSync(join(dir, "lib.rs"), `pub fn clear(hidden_updates: &mut [Option<Vec<i32>>], index: usize) -> i32 {
    let mut n = 0;
    if let Some(hidden_updates_for_lane) = hidden_updates[index].take() {
        for update in hidden_updates_for_lane {
            n += update;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const hiddenUpdatesForLane = hiddenUpdates[index];\n  hiddenUpdates[index] = undefined;\n  if (hiddenUpdatesForLane != null) {\n    for (const update of hiddenUpdatesForLane) {\n");
  expect(js).not.toContain("old");
  const lib = await import(join(dir, "lib.js"));
  const hidden = [[1, 2], undefined];
  expect([lib.clear(hidden, 0), hidden, lib.clear(hidden, 1)]).toEqual([3, [undefined, undefined], 0]);
});

// React's enqueueCapturedUpdate: `if (newLast === null) { .. } else {
// newLast.next = clone; newLast = clone; }`. Its port matches `&new_last`,
// `Some(last) =>`, a binding of the variable the arm sets once it's done
// with it: the variable, where each read of the binding is before it's set,
// not `const last = newLast;`; but its own `const` where one is after.
// The two sets of `clone` are two statements, as upstream writes them (ADR
// 0968, superseded).
test("a binding of a variable set again past its reads is the variable", async () => {
  const dir = fixture("binding-read-before-set");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;
use std::rc::Rc;

pub struct Update {
    pub lane: i32,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub next: RefCell<Option<Rc<Update>>>,
}

pub fn append(lanes: &[i32]) -> Option<Rc<Update>> {
    let mut new_last: Option<Rc<Update>> = None;
    for &lane in lanes {
        let clone = Rc::new(Update { lane, next: RefCell::new(None) });
        match &new_last {
            None => new_last = Some(clone),
            Some(last) => {
                last.next.replace(Some(Rc::clone(&clone)));
                new_last = Some(clone);
            }
        }
    }
    new_last
}

pub fn swapped(lanes: &[i32]) -> i32 {
    let mut held: Option<Rc<Update>> = None;
    let mut sum = 0;
    for &lane in lanes {
        match &held {
            None => held = Some(Rc::new(Update { lane, next: RefCell::new(None) })),
            Some(last) => {
                let last = Rc::clone(last);
                held = Some(Rc::new(Update { lane, next: RefCell::new(None) }));
                sum += last.lane;
            }
        }
    }
    sum
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    if (newLast == null) {\n      newLast = clone;\n    } else {\n      newLast.next = clone;\n      newLast = clone;\n    }\n");
  expect(js).toContain("      const last = held;\n");
  const lib = await import(join(dir, "lib.js"));
  const last = lib.append([1, 2, 3]) as { lane: number };
  expect([last.lane, lib.swapped([1, 2, 3])]).toEqual([3, 3]);
});

// React's getNearestMountedFiber walks up: `while (parent !== null) { node =
// parent; parent = node.return; }`. Its port binds each step, `while let
// Some(parent_fiber) = parent { node = parent_fiber; .. }`, read before
// `parent` is set again: `parent`, not `const parentFiber = parent;`.
test("a while let binding read before its variable is set again is the variable", async () => {
  const dir = fixture("while-let-binding-read-before-set");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub r#return: RefCell<Option<Rc<Fiber>>>,
    pub tag: Cell<u32>,
}

pub fn root_tag(fiber: &Rc<Fiber>) -> u32 {
    let mut node = Rc::clone(fiber);
    let mut parent = node.r#return.borrow().clone();
    while let Some(parent_fiber) = parent {
        node = parent_fiber;
        parent = node.r#return.borrow().clone();
    }
    node.tag.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  while (parent != null) {\n    node = parent;\n    parent = node.return;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  const root = { return: null, tag: 3 };
  const child = { return: { return: root, tag: 1 }, tag: 0 };
  expect([lib.rootTag(child), lib.rootTag(root)]).toEqual([3, 3]);
});

// React's ReactFiberCacheComponent and ReactFiberThenable test a module
// variable in place, `if (lastFiber !== null && lastFiber.index ===
// fiber.index)`. The port's `if let Some(last_fiber) =
// LAST_FIBER.with_borrow(Clone::clone) && ..` was `const lastFiber$1 =
// lastFiber; if (lastFiber$1 != null && ..)`: a thread-local's value an `if
// let` binds, read only in its test, is the variable read there (ADR 0866,
// amended). Read after the variable is set, or past a call that may set it,
// it's its own `const`.
test("an if let's binding of a thread-local only its test reads is the variable", async () => {
  const dir = fixture("thread-local-tested-in-place");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Fiber {
    pub index: f64,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static LAST_FIBER: RefCell<Option<Rc<Fiber>>> = const { RefCell::new(None) };
    static COUNT: Cell<Option<f64>> = const { Cell::new(None) };
}

pub fn remember(index: f64) {
    LAST_FIBER.set(Some(Rc::new(Fiber { index })));
}

pub fn check(fiber: &Fiber) -> bool {
    if let Some(last_fiber) = LAST_FIBER.with_borrow(Clone::clone)
        && last_fiber.index == fiber.index
    {
        LAST_FIBER.set(None);
        return true;
    }
    false
}

pub fn forget() -> f64 {
    if let Some(last) = LAST_FIBER.with_borrow(Clone::clone) {
        LAST_FIBER.set(None);
        return last.index;
    }
    -1.0
}

fn bump() -> f64 {
    COUNT.set(Some(COUNT.get().unwrap_or(0.0) + 1.0));
    1.0
}

pub fn counted() -> f64 {
    COUNT.set(Some(5.0));
    if let Some(count) = COUNT.get()
        && bump() < count
    {
        return count;
    }
    0.0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (lastFiber != null && lastFiber.index === fiber.index) {\n    lastFiber = null;\n    return true;\n  }\n");
  expect(js).toContain("const last = lastFiber;");
  expect(js).toContain("const count$1 = count;");
  const lib = await import(join(dir, "lib.js"));
  lib.remember(3);
  expect([lib.check({ index: 2 }), lib.check({ index: 3 }), lib.check({ index: 3 })]).toEqual([false, true, false]);
  lib.remember(4);
  expect([lib.forget(), lib.forget(), lib.counted()]).toEqual([4, -1, 5]);
});

// React's ReactFiberThenable `getSuspendedThenable`: `const thenable =
// suspendedThenable; suspendedThenable = null; return thenable;`. Its port's
// `let Some(thenable) = SUSPENDED_THENABLE.take() else { panic!(..) };`
// was `const old = suspendedThenable; ..; return old;`, rust-js's name for
// what `take()` gives: a `let`-`else`'s binding names it, as an `if let`'s
// does (ADR 0598, amended).
test("a let else's binding of what take() gives names it", async () => {
  const dir = fixture("taken-let-else-name");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static SUSPENDED_THENABLE: Cell<Option<&'static str>> = const { Cell::new(None) };
}

pub fn suspend(thenable: &'static str) {
    SUSPENDED_THENABLE.set(Some(thenable));
}

pub fn get_suspended_thenable() -> &'static str {
    let Some(thenable) = SUSPENDED_THENABLE.take() else {
        panic!("Expected a suspended thenable.");
    };
    thenable
}

pub fn last(items: &mut Vec<u32>) -> u32 {
    let Some(item) = items.pop() else {
        return 0;
    };
    item + 1
}

pub fn popped(items: &mut Vec<u32>) -> u32 {
    let popped_value = items.pop();
    let Some(item) = popped_value else {
        return 0;
    };
    item + 2
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const thenable = suspendedThenable;\n  suspendedThenable = null;\n  if (thenable == null) {\n");
  expect(js).toContain("  return thenable;\n");
  expect(js).not.toContain("old");
  // A variable the program named keeps its name.
  expect(js).toContain("  const poppedValue = items.pop();\n");
  const lib = await import(join(dir, "lib.js"));
  lib.suspend("a");
  expect(lib.getSuspendedThenable()).toBe("a");
  expect(() => lib.getSuspendedThenable()).toThrow("Expected a suspended thenable.");
  expect([lib.last([1, 2]), lib.last([]), lib.popped([1, 2]), lib.popped([])]).toEqual([3, 0, 4, 0]);
});

// React's ReactFiberHotReloading `scheduleFibersWithFamiliesRecursively`:
// `if (failedBoundaries !== null && (failedBoundaries.has(fiber) ||
// (alternate !== null && failedBoundaries.has(alternate))))`. Its port's
// copy of the thread-local was `const failedBoundaries$1 =
// failedBoundaries;`, as the first `has(..)` is a call, which may run
// code: a weak set's runs none of the program's, so sets none of its
// variables (`Known::RunsNothing`). A binding's call that may, given a
// callback, keeps the copy, as Rust reads what it copied (ADR 0277,
// amended).
test("a copy of a module's variable read past a call that runs nothing is the variable", async () => {
  const dir = fixture("shadow-past-runs-nothing");
  const node = buildNode();
  writeFileSync(join(dir, "lib.rs"), `use std::cell::RefCell;

pub struct Fiber {
    pub tag: u32,
}

unsafe impl js::WeakKey for Fiber {}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static FAILED_BOUNDARIES: RefCell<Option<&'static js::WeakSet<Fiber>>> =
        const { RefCell::new(None) };
}

pub fn needs_remount(fiber: &Fiber, alternate: Option<&Fiber>) -> bool {
    if let Some(failed_boundaries) = FAILED_BOUNDARIES.with_borrow(|failed_boundaries| *failed_boundaries)
        && (failed_boundaries.has(fiber)
            || alternate.is_some_and(|alternate| failed_boundaries.has(alternate)))
    {
        return true;
    }
    false
}

pub fn remounts_after(fiber: &Fiber, before: &dyn Fn()) -> bool {
    if let Some(failed_boundaries) = FAILED_BOUNDARIES.with_borrow(|failed_boundaries| *failed_boundaries) {
        js::function::call(before, None, ());
        return failed_boundaries.has(fiber);
    }
    false
}

pub fn remounts_checked(fiber: &Fiber) -> bool {
    if let Some(failed_boundaries) = FAILED_BOUNDARIES.with_borrow(|failed_boundaries| *failed_boundaries) {
        return failed_boundaries.has(fiber) && checked(failed_boundaries, fiber);
    }
    false
}

fn checked(failed: &js::WeakSet<Fiber>, fiber: &Fiber) -> bool {
    failed.has(fiber)
}

pub fn fail(fiber: &'static Fiber) {
    FAILED_BOUNDARIES.with_borrow_mut(|failed_boundaries| {
        failed_boundaries.get_or_insert_with(js::weak_set::new).add(fiber);
    });
}

pub fn reset() {
    FAILED_BOUNDARIES.set(Some(js::weak_set::new()));
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(node, "libjs.rmeta")}`, "-L", node]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(`export function needsRemount(fiber, alternate) {
  if (
    failedBoundaries != null &&
    (failedBoundaries.has(fiber) || (alternate != null && failedBoundaries.has(alternate)))
  ) {`);
  expect(js).toContain("  if (failedBoundaries != null) {\n    return failedBoundaries.has(fiber) && checked(failedBoundaries, fiber);\n  }\n");
  expect(js).toContain("  const failedBoundaries$1 = failedBoundaries;\n  if (failedBoundaries$1 != null) {\n    before.call();\n");
  const lib = await import(join(dir, "lib.js"));
  const [fiber, alternate, other] = [{ tag: 1 }, { tag: 2 }, { tag: 3 }];
  expect(lib.needsRemount(fiber, undefined)).toBe(false);
  lib.fail(fiber);
  expect([lib.needsRemount(fiber, undefined), lib.needsRemount(alternate, fiber), lib.needsRemount(other, alternate)]).toEqual([true, true, false]);
  expect([lib.remountsChecked(fiber), lib.remountsChecked(other)]).toEqual([true, false]);
  // What `before` sets, a new set, isn't what the copy holds.
  expect(lib.remountsAfter(fiber, () => lib.reset())).toBe(true);
  expect(lib.needsRemount(fiber, undefined)).toBe(false);
});

// ADR 0952: React's ReactStrictModeWarnings, closures `js::on_load!` sets:
// each block's `sortedNames` and each loop's `fiber` named as written, as a
// function's are, not `sortedNames$1`, `fiber$1`; one that would hide a
// module's name it reads keeps its own.
test("names in the closures of code that runs at load are named as written", async () => {
  buildReact();
  const dir = fixture("on-load-sibling-names");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::name = "offset")]
const OFFSET: usize = 100;

pub struct Warnings {
    pub flush: Cell<fn(&[usize], &[usize]) -> usize>,
}

thread_local! {
    pub static WARNINGS: Warnings = Warnings { flush: Cell::new(|_, _| 0) };
}

js::on_load! {
    WARNINGS.with(|warnings| {
        warnings.flush.set(|a, b| {
            let mut total = 0;
            for fiber in a {
                total += fiber;
            }
            for fiber in b {
                total += fiber;
            }
            if !a.is_empty() {
                let sorted_names = a.len();
                total += sorted_names;
            }
            if !b.is_empty() {
                let sorted_names = b.len();
                total += sorted_names;
            }
            for offset in a {
                total += offset * OFFSET;
            }
            total
        });
    });
}

pub fn flush(a: &[usize], b: &[usize]) -> usize {
    WARNINGS.with(|warnings| (warnings.flush.get())(a, b))
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/fiber\$|sortedNames\$/);
  expect(js).toContain("  for (const fiber of b) {\n");
  expect(js).toContain("    const sortedNames = b.length;\n");
  // The module's `offset`, which the loop reads: its own name kept.
  expect(js).toContain("  for (const offset$1 of a) {\n    total = (total + (Math.imul(offset$1, offset) >>> 0)) >>> 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.flush([1, 2], [3])).toEqual(1 + 2 + 3 + 2 + 1 + 100 + 200);
});

// A copy of a module's variable set after it is the variable only where
// each read of it is before each set (ADR 0277, amended, h4): one a closure
// reads, when it's called, after the set, one read after the set, a store,
// an `if` and a `switch` of it after the set, each stays a copy, which
// holds what Rust read. What each gives is what native Rust prints.
test("a copy of a module's variable read after the variable is set stays a copy", async () => {
  const dir = fixture("shadow-read-after-set");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

thread_local! {
    static COUNT: Cell<u32> = const { Cell::new(0) };
    static SEEN: Cell<bool> = const { Cell::new(false) };
}

pub fn read_later() -> u32 {
    COUNT.set(1);
    let count = COUNT.get();
    let read = move || count;
    COUNT.set(2);
    read() * 10 + COUNT.get()
}

pub fn read_after_set() -> u32 {
    let mut total = 0;
    COUNT.set(3);
    while COUNT.get() < 6 {
        let count = COUNT.get();
        COUNT.set(count + 1);
        total += count * 10 + COUNT.get();
    }
    total
}

pub fn set_twice() -> u32 {
    COUNT.set(1);
    let count = COUNT.get();
    COUNT.set(count + 1);
    COUNT.set(count + 5);
    COUNT.get()
}

pub fn set_then_tested() -> bool {
    SEEN.set(false);
    COUNT.set(1);
    let count = COUNT.get();
    COUNT.set(count + 1);
    if count == 1 {
        SEEN.set(true);
    }
    SEEN.get()
}

pub fn set_then_matched() -> u32 {
    COUNT.set(1);
    let count = COUNT.get();
    COUNT.set(count + 1);
    match count {
        1 => SEEN.set(true),
        2 => COUNT.set(9),
        _ => {}
    }
    COUNT.get()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const read = () => count$1;\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.readLater(), lib.readAfterSet(), lib.setTwice(), lib.setThenTested(), lib.setThenMatched()]).toEqual([
    12, 135, 6, true, 2,
  ]);
});
