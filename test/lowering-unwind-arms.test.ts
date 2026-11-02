// Where the arms of a `match` of `catch_unwind` go in JS's `try` and
// `catch` (ADRs 0637, 0660): the `Ok` arm in the `try` where it can't
// throw, else after it, where the `Err` arm, the `catch`, leaves; and what
// the `catch` does first (rd550). Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS, and where it is. */
function compiled(name: string, rust: string): { js: string; dir: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dir };
}

// React's throwAndUnwindWorkLoop, `try { const didFatal = throwException(..);
// .. } catch (error) { if (returnFiber !== null) { workInProgress =
// returnFiber; throw error; } else { panicOnRootError(..); return; } }`: an
// `Err` arm that ends the function leaves it, so the `Ok` arm, which can
// throw, is after the `try`, and the `catch` returns (ADR 0637, amended).
test("an Err arm that ends the function returns from its catch", async () => {
  const { js, dir } = compiled(
    "unwind-err-ends",
    `use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

thread_local! {
    static WORK: Cell<Option<u32>> = const { Cell::new(None) };
}

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#throwException")]
fn throw_exception(_x: u32) -> bool {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#panicOnRootError")]
fn panic_on_root_error(_x: u32) {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#complete")]
fn complete(_x: u32) {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#fatal")]
fn fatal(_x: u32) {
    unreachable!()
}

pub fn unwind(x: u32, return_fiber: Option<u32>) {
    match catch_unwind(AssertUnwindSafe(|| throw_exception(x))) {
        Ok(did_fatal) => {
            if did_fatal {
                fatal(x);
                return;
            }
        }
        Err(error) => {
            if let Some(return_fiber) = return_fiber {
                WORK.set(Some(return_fiber));
                resume_unwind(error);
            } else {
                panic_on_root_error(x);
                return;
            }
        }
    }
    complete(x);
}

pub fn inside(x: u32, go: bool) {
    if go {
        match catch_unwind(AssertUnwindSafe(|| throw_exception(x))) {
            Ok(did_fatal) => {
                if did_fatal {
                    fatal(x);
                }
            }
            Err(_) => panic_on_root_error(x),
        }
    }
    complete(x);
}

pub fn last(x: u32, return_fiber: Option<u32>) {
    match catch_unwind(AssertUnwindSafe(|| throw_exception(x))) {
        Ok(did_fatal) => {
            if did_fatal {
                fatal(x);
            }
            complete(x);
        }
        Err(error) => {
            if let Some(return_fiber) = return_fiber {
                WORK.set(Some(return_fiber));
                resume_unwind(error);
            } else {
                panic_on_root_error(x);
            }
        }
    }
}

pub fn work() -> Option<u32> {
    WORK.get()
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export const log = [];
export const thrown = new Error("thrown");
export function throwException(x) {
  log.push("throw " + x);
  if (x === 0) throw thrown;
  return x > 5;
}
export function panicOnRootError(x) { log.push("panic " + x); }
export function complete(x) { log.push("complete " + x); if (x === 3) throw new Error("completing"); }
export function fatal(x) { log.push("fatal " + x); }
`,
  );
  expect(js).toContain("  let didFatal;\n  try {\n    didFatal = throwException(x);\n  } catch (error) {\n");
  expect(js).toContain("      throw error;\n    } else {\n      panicOnRootError(x);\n      return;\n    }\n  }\n");
  expect(js.slice(js.indexOf("export function unwind"), js.indexOf("export function inside"))).not.toContain("TAG");
  // Of a `match` that ends the function, an `else` that goes on returns
  // where it ends, as the `if`'s other branch throws.
  const last = js.slice(js.indexOf("export function last"), js.indexOf("export function work"));
  expect(last).toContain("    } else {\n      panicOnRootError(x);\n      return;\n    }\n  }\n  if (didFatal) {");
  expect(last).not.toContain("TAG");
  const lib = await import(join(dir, "lib.js"));
  const host = await import(join(dir, "host.js"));
  lib.unwind(0, null);
  lib.unwind(1, null);
  lib.unwind(9, null);
  expect(() => lib.unwind(0, 4)).toThrow(host.thrown);
  expect(lib.work()).toBe(4);
  // What the `Ok` arm throws isn't caught.
  expect(() => lib.unwind(3, 4)).toThrow("completing");
  expect(host.log).toEqual(["throw 0", "panic 0", "throw 1", "complete 1", "throw 9", "fatal 9", "throw 0", "throw 3", "complete 3"]);
  // Of a `match` that ends an `if`'s branch, not the function, what
  // follows runs after the `Err` arm too.
  host.log.length = 0;
  lib.inside(0, true);
  lib.inside(7, true);
  expect(host.log).toEqual(["throw 0", "panic 0", "complete 0", "throw 7", "fatal 7", "complete 7"]);
  host.log.length = 0;
  lib.last(0, null);
  lib.last(7, null);
  expect(() => lib.last(0, 2)).toThrow(host.thrown);
  expect(lib.work()).toBe(2);
  expect(host.log).toEqual(["throw 0", "panic 0", "throw 7", "fatal 7", "complete 7", "throw 0"]);
});

// React's checkPassiveEvents, `if (canUseDOM) { try { .. } catch (e) {
// passiveBrowserEventsSupported = false; } }`: a JS call's `Result` tested
// after what the `if` asks first is the `try` inside that `if`, the call
// made only where it's true, as `&&` makes it (ADR 0706, amended).
test("a JS call's Result tested after what an if asks first is a try inside it", async () => {
  const { js, dir } = compiled(
    "unwind-asked-first",
    `use js::JsError;

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#probe")]
fn probe(_n: u32) -> Result<(), &'static JsError> {
    unreachable!()
}

pub fn supported(can_use_dom: bool, n: u32) -> bool {
    let mut supported = can_use_dom;
    if can_use_dom && probe(n).is_err() {
        supported = false;
    }
    supported
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export const log = [];
export function probe(n) { log.push(n); if (n > 2) throw new Error("no"); }
`,
  );
  expect(js).toContain(`export function supported(canUseDom, n) {
  let supported = canUseDom;
  if (canUseDom) {
    try {
      probe(n);
    } catch {
      supported = false;
    }
  }
  return supported;
}`);
  const lib = await import(join(dir, "lib.js"));
  const host = await import(join(dir, "host.js"));
  expect([lib.supported(true, 1), lib.supported(true, 3), lib.supported(false, 3)]).toEqual([true, false, false]);
  // Not called where what's asked first is false.
  expect(host.log).toEqual([1, 3]);
});

// React DOM's commitMount: `// For object src, .. \n try { setSrcObject(..);
// return; } catch (x) { // If URL.createObjectURL() errors, .. }`. The
// comments above the `match` are above the `try`, one in the closure is in
// it, and an `Err` arm of only comments is the `catch`'s (rd156).
test("comments around a match of catch_unwind are where they're written", async () => {
  const { js, dir } = compiled(
    "unwind-arm-comments",
    `use std::panic::{AssertUnwindSafe, catch_unwind};

fn risky(n: i32) {
    if n > 2 {
        panic!("big");
    }
}

pub fn mount(n: i32, out: &mut Vec<i32>) {
    if n > 0 {
        // Try the fast way first.
        match catch_unwind(AssertUnwindSafe(|| {
            // In the try.
            risky(n)
        })) {
            Ok(()) => return,
            Err(_) => {
                // It threw: fall through to the normal
                // path.
            }
        }
        out.push(n);
    }
}
`,
  );
  expect(js).toContain(`  if (n > 0) {
    // Try the fast way first.
    try {
      // In the try.
      risky(n);
      return;
    } catch {
      // It threw: fall through to the normal
      // path.
    }
    out.push(n);
  }`);
  const lib = await import(join(dir, "lib.js"));
  const out: number[] = [];
  for (const n of [1, 5, 0]) lib.mount(n, out);
  expect(out).toEqual([5]);
});

// React's Fizz replayElement pops its replay first in its catch, `catch (x)
// { task.replay = replay; .. throw x; }`, which must run where the stack is
// spent, rendering very deep trees: a `catch`'s payload is made where it's
// first read, after what its arm does first, and one resumed as it was
// given, nothing written through it, is what was thrown, `throw thrown`,
// with no call before it (rd550).
test("a catch makes its payload where it's first read, and resumes it as thrown", () => {
  const { js, dir } = compiled("unwind-where-read", `use js::JsError;
use std::any::Any;
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#overflow")]
fn overflow() {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#render")]
fn render() {
    unreachable!()
}

pub fn f(d: u32, s: &Cell<u32>) {
    if d == 0 {
        return overflow();
    }
    if let Err(e) = catch_unwind(AssertUnwindSafe(|| f(d - 1, s))) {
        s.set(d);
        resume_unwind(e);
    }
}

fn name_of(payload: &(dyn Any + Send)) -> String {
    payload.downcast_ref::<&'static JsError>().map(|e| e.name()).unwrap_or_default()
}

/// Each level that entered its \`try\`, each whose \`catch\` wrote first, and
/// those that read a \`RangeError\` after.
pub fn deep(d: u32, entered: &Cell<u32>, wrote: &Cell<u32>, ranges: &Cell<u32>) {
    if d == 0 {
        return;
    }
    entered.set(entered.get() + 1);
    if let Err(e) = catch_unwind(AssertUnwindSafe(|| deep(d - 1, entered, wrote, ranges))) {
        wrote.set(wrote.get() + 1);
        if name_of(&*e) == "RangeError" {
            ranges.set(ranges.get() + 1);
        }
        resume_unwind(e);
    }
}

/// What JS threw, a \`&JsError\`, read after a write, then resumed.
pub fn rethrown(seen: &Cell<u32>) -> String {
    match catch_unwind(AssertUnwindSafe(render)) {
        Ok(()) => String::new(),
        Err(e) => {
            seen.set(seen.get() + 1);
            let name = name_of(&*e);
            if name == "TypeError" {
                resume_unwind(e);
            }
            name
        }
    }
}

/// A panic's message read after a write, and resumed.
pub fn message(n: u32, seen: &Cell<u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| if n > 2 { panic!("big {n}") } else { n })) {
        Ok(v) => v,
        Err(e) => {
            seen.set(seen.get() + 1);
            if let Some(m) = e.downcast_ref::<String>() {
                seen.set(seen.get() + m.len() as u32);
            }
            resume_unwind(e)
        }
    }
}

/// Changed through \`downcast_mut\`, so resumed as what it is now.
pub fn changed(n: u32) -> u32 {
    match catch_unwind(|| if n > 2 { panic!("big {n}") } else { n }) {
        Ok(v) => v,
        Err(mut e) => {
            if let Some(m) = e.downcast_mut::<String>() {
                m.push('!');
            }
            resume_unwind(e)
        }
    }
}
`);
  // What the arm does first runs first: no call before it.
  expect(js).toContain(`  } catch (e) {
    s.value = d;
    throw e;
  }`);
  expect(js).toContain(`  } catch (thrown) {
    wrote.value = (wrote.value + 1) >>> 0;
    const e = $unwound(thrown);
    if (nameOf(e) === "RangeError") {
      ranges.value = (ranges.value + 1) >>> 0;
    }
    throw thrown;
  }`);
  expect(js).toContain(`  } catch (thrown) {
    seen.value = (seen.value + 1) >>> 0;
    const e = $unwound(thrown);
    const name = nameOf(e);
    if (name === "TypeError") {
      throw thrown;
    }
    return name;
  }`);
  // Changed, it's resumed as what it is now, `$unwinding`'s.
  expect(js).toContain("    throw $unwinding(e);\n");
  writeFileSync(join(dir, "host.js"), `export function overflow() {
  overflow();
}
export function render() {
  throw globalThis.toThrow;
}
`);
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const cell = () => ({ value: 0 });
const outcome = (f) => {
  try {
    return ["returned", f()];
  } catch (e) {
    return ["threw", e];
  }
};
const s = cell();
const [how, error] = outcome(() => lib.f(3, s));
console.log("f", how, error.constructor.name, s.value);
for (let i = 0; i < 3; i++) {
  const [entered, wrote, ranges] = [cell(), cell(), cell()];
  const [how, error] = outcome(() => lib.deep(1e6, entered, wrote, ranges));
  console.log("deep", how, error.constructor.name, entered.value > 1000, entered.value === wrote.value, ranges.value > 0);
}
const seen = cell();
globalThis.toThrow = new TypeError("no props");
const [rethrew, again] = outcome(() => lib.rethrown(seen));
console.log("rethrown", rethrew, again === globalThis.toThrow, seen.value);
globalThis.toThrow = new RangeError("deep");
console.log("named", JSON.stringify(outcome(() => lib.rethrown(seen))), seen.value);
const counted = cell();
console.log("message", JSON.stringify(outcome(() => lib.message(2, counted))), counted.value);
const [threw, panic] = outcome(() => lib.message(5, counted));
console.log("message", threw, panic.constructor === Error, panic.message, counted.value);
const [, changed] = outcome(() => lib.changed(7));
console.log("changed", changed.constructor === Error, changed.message);
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 60_000);
  expect(ran.stderr).toBe("");
  expect(ran.stdout).toBe(`f threw RangeError 3
deep threw RangeError true true true
deep threw RangeError true true true
deep threw RangeError true true true
rethrown threw true 1
named ["returned","RangeError"] 2
message ["returned",2] 0
message threw true big 5 6
changed true big 7!
`);
});

// React's Fizz renderNode keeps what each arm caught in a `let thrown;` of
// its own, `thrown = thrown_value;`, and restores its task in the `catch`
// before anything else: where the stack is spent, rendering very deep
// trees, `$catchUnwind(() => ..)`'s two frames, and its `$unwound` before
// the arm's first statement, throw again before the arm writes. A
// `catch`'s binding is `thrown`, or given apart from what the `catch`
// reads, `thrown$1`, so a name of the program's never keeps the helper
// (rd551).
test("a catch_unwind whose arm reads a thrown of its own is a try", () => {
  const { js, dir } = compiled("unwind-thrown-named", `use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

/// renderNode's: each arm a \`let Err(..) = catch_unwind(..) else { return; }\`
/// that writes first, then keeps what it caught in the function's \`thrown\`.
pub fn render_node(d: u32, entered: &Cell<u32>, wrote: &Cell<u32>, segment: Option<u32>) {
    if d == 0 {
        return;
    }
    entered.set(entered.get() + 1);
    let thrown;
    match segment {
        None => {
            let Err(thrown_value) = catch_unwind(AssertUnwindSafe(|| render_node(d - 1, entered, wrote, None))) else {
                return;
            };
            wrote.set(wrote.get() + 1);
            thrown = thrown_value;
        }
        Some(step) => {
            let Err(thrown_value) =
                catch_unwind(AssertUnwindSafe(|| render_node(d - 1, entered, wrote, Some(step)))) else {
                return;
            };
            wrote.set(wrote.get() + step);
            thrown = thrown_value;
        }
    }
    resume_unwind(thrown);
}

/// A \`Result\` named, then matched, its \`Err\` arm restoring by a \`thrown\`
/// it's given.
pub fn restored(d: u32, entered: &Cell<u32>, wrote: &Cell<u32>, thrown: u32) {
    if d == 0 {
        return;
    }
    entered.set(entered.get() + 1);
    let result = catch_unwind(AssertUnwindSafe(|| restored(d - 1, entered, wrote, thrown)));
    match result {
        Ok(()) => {}
        Err(x) => {
            wrote.set(wrote.get() + thrown);
            resume_unwind(x)
        }
    }
}

/// A \`let Err(..) else\` at a function's top whose arm reads \`thrown\`.
pub fn guarded(d: u32, entered: &Cell<u32>, wrote: &Cell<u32>, thrown: u32) {
    if d == 0 {
        return;
    }
    entered.set(entered.get() + 1);
    let Err(x) = catch_unwind(AssertUnwindSafe(|| guarded(d - 1, entered, wrote, thrown))) else {
        return;
    };
    wrote.set(wrote.get() + thrown);
    resume_unwind(x);
}

fn big(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    n
}

/// The payload itself named \`thrown\`, read as a message.
pub fn named(n: u32) -> String {
    match catch_unwind(|| big(n)) {
        Ok(v) => v.to_string(),
        Err(thrown) => thrown.downcast_ref::<String>().cloned().unwrap_or_default(),
    }
}
`);
  expect(js).not.toContain("$catchUnwind");
  expect(js).toContain(`  let thrown;
  if (segment == null) {
    try {
      renderNode(d - 1, entered, wrote);
      return;
    } catch (thrown$1) {
      wrote.value = (wrote.value + 1) >>> 0;
      const thrownValue = $unwound(thrown$1);
      thrown = thrownValue;
    }
  } else {`);
  expect(js).toContain(`  try {
    restored(d - 1, entered, wrote, thrown);
  } catch (x) {
    wrote.value = (wrote.value + thrown) >>> 0;
    throw x;
  }`);
  expect(js).toContain(`  try {
    guarded(d - 1, entered, wrote, thrown);
  } catch (x) {
    wrote.value = (wrote.value + thrown) >>> 0;
    throw x;
  }`);
  expect(js).toContain(`  try {
    const v = big(n);
    return String(v);
  } catch (thrown$1) {
    const thrown = $unwound(thrown$1);
`);
  writeFileSync(join(dir, "run.mjs"), `import * as lib from "./lib.js";
const cell = () => ({ value: 0 });
const outcome = (f) => {
  try {
    return ["returned", f()];
  } catch (e) {
    return ["threw", e];
  }
};
for (const [name, run] of [
  ["renderNode", (e, w) => lib.renderNode(1e6, e, w, undefined)],
  ["renderNode", (e, w) => lib.renderNode(1e6, e, w, 1)],
  ["restored", (e, w) => lib.restored(1e6, e, w, 1)],
  ["guarded", (e, w) => lib.guarded(1e6, e, w, 1)],
]) {
  const [entered, wrote] = [cell(), cell()];
  const [how, error] = outcome(() => run(entered, wrote));
  console.log(name, how, error.constructor.name, entered.value > 1000, entered.value === wrote.value);
}
const [entered, wrote] = [cell(), cell()];
console.log("shallow", JSON.stringify(outcome(() => lib.renderNode(3, entered, wrote, 2))), entered.value, wrote.value);
console.log("named", lib.named(1), lib.named(5));
`);
  const ran = runSync([node!, join(dir, "run.mjs")], root, 60_000);
  expect(ran.stderr).toBe("");
  // Every level that entered its \`try\` wrote in its \`catch\`, where the
  // helper's left some unwritten.
  expect(ran.stdout).toBe(`renderNode threw RangeError true true
renderNode threw RangeError true true
restored threw RangeError true true
guarded threw RangeError true true
shallow ["returned",null] 3 0
named 1 big 5
`);
});

// React's throwAndUnwindWorkLoop keeps the value its closure makes in the
// function's `didFatal`, `didFatal = throwException(..)` in the `try`, where
// it was `const didFatal$1 = throwException(..); .. didFatal = didFatal$1;`
// (rd534): what follows the value may throw, `panicOnRootError(..)`, and
// the `catch` never goes on, so nothing sees `didFatal` set early. Where
// the `catch` goes on to what reads the variable, the copy stays, and the
// variable is as it was. Run in Node.
test("a closure's value a try keeps sets its variable where it's made", () => {
  const { js, dir } = compiled(
    "unwind-set-where-made",
    `use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#throwException")]
fn throw_exception(_x: u32) -> bool {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#panicOnRootError")]
fn panic_on_root_error(_x: u32) {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#note")]
fn note(_x: u32) {
    unreachable!()
}

pub fn unwind(x: u32, return_fiber: Option<u32>) -> bool {
    let did_fatal = match catch_unwind(AssertUnwindSafe(|| {
        let did_fatal = throw_exception(x);
        if did_fatal {
            panic_on_root_error(x);
        }
        did_fatal
    })) {
        Ok(did_fatal) => did_fatal,
        Err(error) => {
            if let Some(return_fiber) = return_fiber {
                note(return_fiber);
                resume_unwind(error);
            } else {
                note(0);
                return false;
            }
        }
    };
    if did_fatal {
        note(5);
        return true;
    }
    note(6);
    false
}

pub fn kept(x: u32) -> bool {
    let mut did_fatal = false;
    match catch_unwind(AssertUnwindSafe(|| {
        let found = throw_exception(x);
        panic_on_root_error(x);
        found
    })) {
        Ok(found) => did_fatal = found,
        Err(_) => note(1),
    }
    did_fatal
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export const log = [];
export function throwException(x) { log.push("throw " + x); return x > 5; }
export function panicOnRootError(x) { log.push("panic " + x); if (x > 7) throw new Error("root " + x); }
export function note(x) { log.push("note " + x); }
`,
  );
  const unwind = js.slice(js.indexOf("export function unwind"), js.indexOf("export function kept"));
  expect(unwind).toContain("  let didFatal;\n  try {\n    didFatal = throwException(x);\n    if (didFatal) {\n      panicOnRootError(x);\n    }\n  } catch (error) {\n");
  expect(unwind).not.toContain("$1");
  const kept = js.slice(js.indexOf("export function kept"));
  expect(kept).toContain("    const found = throwException(x);\n    panicOnRootError(x);\n    didFatal = found;\n");
  writeFileSync(
    join(dir, "main.mjs"),
    `import * as lib from "./lib.js";
import { log } from "./host.js";
const thrown = (f) => { try { return f(); } catch (e) { return e.message; } };
console.log(JSON.stringify([lib.unwind(1, null), lib.unwind(6, null), lib.unwind(9, null), thrown(() => lib.unwind(9, 4))]));
console.log(JSON.stringify([lib.kept(1), lib.kept(6), lib.kept(9)]));
console.log(log.join(", "));
`,
  );
  const out = runSync([node!, join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(out.stdout).toBe(`[false,true,false,"root 9"]
[false,true,false]
throw 1, note 6, throw 6, panic 6, note 5, throw 9, panic 9, note 0, throw 9, panic 9, note 4, throw 1, panic 1, throw 6, panic 6, throw 9, panic 9, note 1
`);
});

// React's isRenderConsistentWithExternalStores, `try { if
// (!objectIs(getSnapshot(), renderedValue)) { return false; } } catch {
// return false; }`: a `match` of `catch_unwind` whose `Ok` arms test the
// value, `Ok(true) => {}, Ok(false) => return false`, read once where the
// `try` evaluates it first, is the call there, not a `const catchUnwind`
// named after the `Result` (ADR 0706, amended). Run in Node.
test("a try's value read once where it's evaluated first is the call there", () => {
  const { js, dir } = compiled(
    "unwind-value-in-place",
    `use std::panic::{AssertUnwindSafe, catch_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#same")]
fn same(_a: u32, _b: u32) -> bool {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#note")]
fn note(_x: u32) {
    unreachable!()
}

pub fn consistent(checks: &[(u32, u32)]) -> bool {
    let mut i = 0;
    while i < checks.len() {
        let (a, b) = checks[i];
        match catch_unwind(AssertUnwindSafe(|| same(a, b))) {
            Ok(true) => {}
            Ok(false) => {
                // Found an inconsistent store.
                return false;
            }
            Err(_) => {
                // If it throws, return \`false\`.
                return false;
            }
        }
        i += 1;
    }
    true
}

pub fn picked(n: u32) -> u32 {
    match catch_unwind(|| {
        note(n);
        same(n, 3)
    }) {
        Ok(true) => 1,
        Ok(false) => 2,
        Err(_) => 3,
    }
}

pub fn twice(n: u32) -> u32 {
    match catch_unwind(|| same(n, 3)) {
        Ok(found) if found => 1,
        Ok(found) => u32::from(found) + 2,
        Err(_) => 3,
    }
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export const log = [];
export function same(a, b) { log.push("same " + a); if (a > 7) throw new Error("same " + a); return a === b; }
export function note(x) { log.push("note " + x); }
`,
  );
  expect(js).toContain("    try {\n      if (!same(a, b)) {\n        // Found an inconsistent store.\n        return false;\n      }\n");
  expect(js).toContain("    note(n);\n    return same(n, 3) ? 1 : 2;\n");
  expect(js).not.toContain("catchUnwind");
  writeFileSync(
    join(dir, "main.mjs"),
    `import * as lib from "./lib.js";
import { log } from "./host.js";
console.log(JSON.stringify([lib.consistent([[1, 1], [2, 2]]), lib.consistent([[1, 1], [2, 3], [4, 4]]), lib.consistent([[9, 9], [1, 1]])]));
console.log(JSON.stringify([lib.picked(3), lib.picked(4), lib.picked(9), lib.twice(3), lib.twice(4), lib.twice(9)]));
console.log(log.join(", "));
`,
  );
  const out = runSync([node!, join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(out.stdout).toBe(`[true,false,false]
[1,2,3,1,2,3]
same 1, same 2, same 1, same 2, same 9, note 3, same 3, note 4, same 4, note 9, same 9, same 3, same 4, same 9
`);
});

// rd60: a `catch_unwind` whose closure returns early, a guard's `if n == 0
// { return 0; }`, its value a `let`'s in an outer `catch_unwind`'s
// closure: each `return v;` sets the `let`, `found = 0;`, what follows the
// guard its `else`, where it was `found = (() => { .. })();`, a closure
// called (ADR 0706, amended). Run in Node.
test("a closure that returns early sets the value a try keeps in each branch", () => {
  const { js, dir } = compiled(
    "unwind-returns-set",
    `use std::panic::catch_unwind;

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#step")]
fn step(_n: u32) -> u32 {
    unreachable!()
}

pub fn guarded(n: u32) -> u32 {
    match catch_unwind(|| {
        let found = match catch_unwind(|| {
            if n == 0 {
                return 0;
            }
            let a = step(n);
            a * 2
        }) {
            Ok(v) => v,
            Err(_) => return 3,
        };
        step(found)
    }) {
        Ok(v) => v,
        Err(_) => 9,
    }
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export const log = [];
export function step(n) { log.push("step " + n); if (n > 7) throw new Error("big " + n); return n + 1; }
`,
  );
  expect(js).toContain(
    "    let found;\n    try {\n      if (n === 0) {\n        found = 0;\n      } else {\n        const a = step(n);\n        found = (a * 2) >>> 0;\n      }\n    } catch {\n      return 3;\n    }\n    return step(found);\n",
  );
  expect(js).not.toContain("(() =>");
  writeFileSync(
    join(dir, "main.mjs"),
    `import * as lib from "./lib.js";
import { log } from "./host.js";
console.log(JSON.stringify([lib.guarded(0), lib.guarded(2), lib.guarded(3), lib.guarded(9)]));
console.log(log.join(", "));
`,
  );
  const out = runSync([node!, join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(out.stdout).toBe(`[1,7,9,3]
step 0, step 2, step 6, step 3, step 8, step 9
`);
});

// React's throwAndUnwindWorkLoop, `try { const didFatal = throwException(..);
// if (didFatal) { panicOnRootError(..); return; } } catch (error) { .. }`:
// a closure that returns a literal, `return true;` after
// `panic_on_root_error(..)`, and `Ok` arms that test it, `Ok(true) =>
// return 0, Ok(false) => {}`, is each arm where the closure returns its
// value, `return 0;` there, and `false` what falls out of the `try`, with
// no `catchUnwind` it set (rd60, ADR 0706 amended). An arm that may throw
// stays after the `try`, so what it throws isn't the `catch`'s. Run in Node.
test("an Ok arm of a literal the closure returns is where it's returned", () => {
  const { js, dir } = compiled(
    "unwind-arms-where-returned",
    `use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#throwException")]
fn throw_exception(_x: u32) -> bool {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#panicOnRootError")]
fn panic_on_root_error(_x: u32) {
    unreachable!()
}
#[cfg_attr(rust_js, rust_js::link_name = "./host.js#note")]
fn note(_x: u32) {
    unreachable!()
}

pub fn unwind(x: u32, ret: Option<u32>) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        let did_fatal = throw_exception(x);
        if did_fatal {
            panic_on_root_error(x);
            return true;
        }
        false
    })) {
        Ok(true) => return 0,
        Ok(false) => {}
        Err(error) => {
            if let Some(r) = ret {
                note(r);
                resume_unwind(error);
            } else {
                panic_on_root_error(x);
                return 1;
            }
        }
    }
    x + 1
}

pub fn arm_throws(x: u32) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if throw_exception(x) {
            return true;
        }
        note(x);
        false
    })) {
        Ok(true) => {
            panic_on_root_error(x);
            return 2;
        }
        Ok(false) => {}
        Err(_) => return 9,
    }
    x
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export const log = [];
export function throwException(x) { log.push("exception " + x); if (x === 5) throw new Error("exception " + x); return x > 2; }
export function panicOnRootError(x) { log.push("root " + x); if (x >= 7) throw new Error("root " + x); }
export function note(x) { log.push("note " + x); }
`,
  );
  expect(js).toContain(
    "  try {\n    const didFatal = throwException(x);\n    if (didFatal) {\n      panicOnRootError(x);\n      return 0;\n    }\n  } catch (error) {\n",
  );
  expect(js).toContain("      return 1;\n    }\n  }\n  return (x + 1) >>> 0;\n}\n");
  expect(js).not.toContain("catchUnwind = true");
  writeFileSync(
    join(dir, "main.mjs"),
    `import * as lib from "./lib.js";
import { log } from "./host.js";
const attempt = (f) => { try { return f(); } catch (e) { return "threw " + e.message; } };
console.log(JSON.stringify([1, 3, 8, 5].map((x) => attempt(() => lib.unwind(x))).concat([attempt(() => lib.unwind(5, 4))])));
console.log(JSON.stringify([1, 3, 5, 8].map((x) => attempt(() => lib.armThrows(x)))));
console.log(log.join(", "));
`,
  );
  const out = runSync([node!, join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(out.stdout).toBe(`[2,0,"threw root 8",1,"threw exception 5"]
[1,2,9,"threw root 8"]
exception 1, exception 3, root 3, exception 8, root 8, root 8, exception 5, root 5, exception 5, note 4, exception 1, note 1, exception 3, root 3, exception 5, exception 8, root 8
`);
});

// #462: `catch_unwind(|| js::has(m, "default")).is_err()` asks whether
// `"default" in m` throws, as JS's `in` of `undefined`, `null` or a string
// does; the value nothing reads is still evaluated, an expression
// statement in the `try`, so `caught(undefined)` is `true` as Rust's is,
// where applied arrows' value nothing read went, `try { return false; }`.
// So for each JS operator of any value, which may throw or run a
// `valueOf`: `==`, its negation `!=`, `instanceof` and `+`; for a getter's
// read, a call, and an unread `let` of `in` outside any `try`, which
// prepare's unread `const`s kept no more; not `+s` of text, which can't
// throw. Run in Node.
test("a caught value nothing reads that may throw is still evaluated", () => {
  const { js, dir } = compiled(
    "unwind-unread-throws",
    `use js::Unknown;
use std::panic::catch_unwind;

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#work")]
fn work(_n: u32) -> u32 {
    unreachable!()
}

pub fn caught(m: Option<&'static Unknown>) -> bool {
    catch_unwind(|| js::has(m, "default")).is_err()
}

pub fn read(m: Option<&'static Unknown>) -> bool {
    catch_unwind(|| js::get(m, "value")).is_err()
}

pub fn called(n: u32) -> bool {
    catch_unwind(|| work(n)).is_err()
}

pub fn fine(m: Option<&'static Unknown>) -> bool {
    catch_unwind(|| js::has(m, "default")).is_ok()
}

pub fn loose(m: Option<&'static Unknown>) -> bool {
    catch_unwind(|| js::loose_equals(m, 1.0)).is_err()
}

pub fn unequal(m: Option<&'static Unknown>) -> bool {
    catch_unwind(|| !js::loose_equals(m, 1.0)).is_err()
}

pub fn instance(m: Option<&'static Unknown>, c: Option<&'static Unknown>) -> bool {
    catch_unwind(|| js::instance_of(m, c)).is_err()
}

pub fn number(m: &'static Unknown) -> bool {
    catch_unwind(|| js::to_number(m).unwrap()).is_err()
}

pub fn unread(m: Option<&'static Unknown>) -> u32 {
    let _present = js::has(m, "default");
    1
}

pub fn of_text(s: &str) -> u32 {
    let _n = js::to_number(s);
    1
}
`,
  );
  writeFileSync(
    join(dir, "host.js"),
    `export function work(n) { if (n > 2) throw new Error("big " + n); return n; }
`,
  );
  expect(js).toContain('export function caught(m) {\n  try {\n    "default" in m;\n    return false;\n  } catch {\n');
  expect(js).toContain("export function loose(m) {\n  try {\n    m == 1;\n    return false;\n");
  expect(js).toContain("export function unequal(m) {\n  try {\n    m != 1;\n    return false;\n");
  expect(js).toContain("export function instance(m, c) {\n  try {\n    m instanceof c;\n    return false;\n");
  expect(js).toContain("export function number(m) {\n  try {\n    +m;\n    return false;\n");
  expect(js).toContain('export function unread(m) {\n  const _present = "default" in m;\n  return 1;\n}');
  // `+s` of text throws nothing and runs nothing: unread, it's no statement.
  expect(js).toContain("export function ofText(s) {\n  return 1;\n}");
  writeFileSync(
    join(dir, "main.mjs"),
    `import * as lib from "./lib.js";
const getter = { get value() { throw new Error("getter"); } };
console.log(JSON.stringify([lib.caught(undefined), lib.caught(null), lib.caught("text"), lib.caught({}), lib.caught({ default: 1 })]));
console.log(JSON.stringify([lib.read(undefined), lib.read(null), lib.read(getter), lib.read({ value: 1 })]));
console.log(JSON.stringify([lib.called(1), lib.called(3)]));
console.log(JSON.stringify([lib.fine(undefined), lib.fine({})]));
const valued = { valueOf() { throw new Error("valueOf"); } };
console.log(JSON.stringify([lib.loose(valued), lib.loose(1), lib.unequal(valued), lib.unequal(1), lib.instance({}, 1), lib.instance({}, Object), lib.number(Symbol()), lib.number(2n), lib.number("2")]));
const thrown = (f) => { try { return f(); } catch (e) { return e.constructor.name; } };
console.log(JSON.stringify([thrown(() => lib.unread(undefined)), thrown(() => lib.unread({}))]));
`,
  );
  const out = runSync([node!, join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(out.stdout).toBe(`[true,true,true,false,false]
[true,true,true,false]
[false,true]
[false,true]
[true,false,true,false,true,false,true,true,false]
["TypeError",1]
`);
});

// `catch_unwind(|| inner(n))`'s `try` is the closure called, which tidy's
// applied arrows make its body: its `return inner(n);` maps to the
// closure's `inner(n)`, where the program wrote the call, not to the `Ok`
// arm's line, as it did where the closure's body was taken out of it
// without its place (ADR 0660).
test("a try's call of a closure of one value maps to the closure's body", async () => {
  const { js, dir } = compiled(
    "unwind-call-mapped",
    `use std::panic::{catch_unwind, resume_unwind};

fn inner(n: u32) -> u32 {
    if n > 2 {
        panic!("big {n}");
    }
    n + 1
}

pub fn again(n: u32) -> u32 {
    match catch_unwind(|| inner(n)) {
        Ok(v) => v,
        Err(payload) => resume_unwind(payload),
    }
}
`,
  );
  const { decodeMappings } = await import("./sourcemap");
  const lines = js.split("\n");
  const at = lines.findIndex((line) => line.includes("return inner(n);"));
  expect(at).toBeGreaterThan(0);
  const map = JSON.parse(readFileSync(join(dir, "lib.js.map"), "utf8"));
  const column = lines[at].indexOf("return");
  const mapped = decodeMappings(map.mappings).filter((s) => s.jsLine === at && s.jsCol <= column).at(-1);
  // Line 11, 1-based, of `match catch_unwind(|| inner(n)) {`, at `inner(n)`.
  expect(mapped && [mapped.srcLine + 1, mapped.srcCol]).toEqual([11, 26]);
});

// A JS call's `Result` matched, `let value = match js::eval(code) { Ok(v)
// => v, Err(_) => None };`: the `try`'s `value = (0, eval)(code);` is where
// that value is made, the call's line, not where the `Ok` arm is written,
// which only a `return` the program wrote there is (rd525, rd545). Run in
// Node.
test("a try's assignment of a JS call's Ok value maps to the call", async () => {
  const { js, dir } = compiled(
    "tried-assignment-mapped",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "./host.js#note")]
fn note(_v: Option<&'static Unknown>) {
    unreachable!()
}

pub fn assigned(code: &str) -> Option<&'static Unknown> {
    let value = match js::eval(code) {
        Ok(v) => v,
        Err(_) => None,
    };
    note(value);
    value
}
`,
  );
  writeFileSync(join(dir, "host.js"), "export const log = [];\nexport function note(v) { log.push(v); }\n");
  expect(js).toContain("  let value;\n  try {\n    value = (0, eval)(code);\n  } catch {\n    value = undefined;\n  }\n");
  const { decodeMappings } = await import("./sourcemap");
  const lines = js.split("\n");
  const at = lines.findIndex((line) => line.includes("value = (0, eval)(code);"));
  const map = JSON.parse(readFileSync(join(dir, "lib.js.map"), "utf8"));
  const column = lines[at].indexOf("value");
  // What the statement's start is mapped to, its own or the last before it.
  const mapped = decodeMappings(map.mappings)
    .filter((s) => s.jsLine < at || (s.jsLine === at && s.jsCol <= column))
    .at(-1);
  // Line 9, 1-based, of `let value = match js::eval(code) {`; not line 10's
  // `Ok(v) => v,`.
  expect(mapped && mapped.srcLine + 1).toBe(9);
  writeFileSync(
    join(dir, "main.mjs"),
    `import * as lib from "./lib.js";
import { log } from "./host.js";
console.log(JSON.stringify([lib.assigned("1 + 2"), lib.assigned("("), log]));
`,
  );
  const out = runSync([node!, join(dir, "main.mjs")], dir, 60_000);
  expect(out.stderr).toBe("");
  expect(out.stdout).toBe("[3,null,[3,null]]\n");
});
