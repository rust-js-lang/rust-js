// Where the arms of a `match` of `catch_unwind` go in JS's `try` and
// `catch` (ADRs 0637, 0660): the `Ok` arm in the `try` where it can't
// throw, else after it, where the `Err` arm, the `catch`, leaves. Each test
// is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

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
