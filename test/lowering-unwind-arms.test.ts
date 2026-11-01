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
