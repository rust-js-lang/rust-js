// A guard whose `Drop` restores what its scope changed, as React restores
// it in a `finally` (ADRs 0589, 0693, 0705). Each test compiles a program
// and runs it.

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

/** React's `runWithFiberInDEV`, its guard's `Drop` of `drop`'s body. */
const runWithFiber = (guard: string, drop: string) => `use std::cell::RefCell;

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable(fiber))]
pub fn set_current_fiber(fiber: Option<String>) {
    CURRENT.set(fiber);
}

pub fn current_fiber() -> Option<String> {
    CURRENT.with_borrow(|current| current.clone())
}

${guard}

impl Drop for Restore {
    fn drop(&mut self) {
        ${drop}
    }
}

pub fn run_with_fiber_in_dev<T>(fiber: Option<String>, wrap: bool, callback: impl FnOnce() -> T) -> T {
    let previous_fiber = CURRENT.with_borrow(|current| current.clone());
    set_current_fiber(fiber);
    let _restore = Restore(previous_fiber);
    if wrap {
        return callback();
    }
    callback()
}
`;

/** Each way out of `runWithFiberInDEV` restores the fiber before it, once. */
async function restoresEach(dir: string) {
  const lib = await import(join(dir, "lib.js"));
  lib.setCurrentFiber("root");
  const seen: unknown[] = [];
  expect(lib.runWithFiberInDev("a", true, () => (seen.push(lib.currentFiber()), 1))).toBe(1);
  expect(lib.currentFiber()).toBe("root");
  expect(lib.runWithFiberInDev("b", false, () => (seen.push(lib.currentFiber()), 2))).toBe(2);
  expect(lib.currentFiber()).toBe("root");
  expect(() =>
    lib.runWithFiberInDev("c", true, () => {
      seen.push(lib.currentFiber());
      throw new RangeError("thrown");
    }),
  ).toThrow("thrown");
  expect([seen, lib.currentFiber()]).toEqual([["a", "b", "c"], "root"]);
}

// React's `runWithFiberInDEV` returns from inside its `try`, whose
// `finally` restores the fiber: `try { if (..) { return ..; } return
// callback(..); } finally { setCurrentFiber(previousFiber); }`. Its port's
// `return callback();` from the guard's scope gives the function's return
// place, which neither a closure nor the drop sets: returned in the `try`,
// whose `finally` drops the guard after the value's made, as Rust drops it
// (ADR 0693, amended), not a `try` of each way out and a `let result`.
test("an early return in a guard's scope returns in its try", async () => {
  const { js, dir } = compiled(
    "guard-early-return",
    runWithFiber("struct Restore {\n    previous: Option<String>,\n}", "set_current_fiber(self.previous.clone());").replace(
      "let _restore = Restore(previous_fiber);",
      "let _restore = Restore { previous: previous_fiber };",
    ),
  );
  expect(js).toContain(`  try {
    if (wrap) {
      return callback();
    }
    return callback();
  } finally {
    setCurrentFiber(previousFiber);
  }
`);
  expect(js).not.toContain("let result");
  expect(js.match(/finally/g)?.length).toBe(1);
  await restoresEach(dir);
});

// Its port's guard is a tuple struct, `struct Restore(Option<Fiber>)`: its
// one field, made of a variable nothing sets once the guard's made, reads
// as that variable, `setCurrentFiber(previousFiber)`, as a named field's
// does (ADR 0705), not a `const restore = previousFiber;` of its own. One
// set later is what it was when the guard was made, a `const` of its own.
test("a one-field guard of a variable reads as that variable", async () => {
  const { js, dir } = compiled(
    "guard-one-field-variable",
    runWithFiber("struct Restore(Option<String>);", "set_current_fiber(self.0.clone());"),
  );
  expect(js).toContain(`  try {
    if (wrap) {
      return callback();
    }
    return callback();
  } finally {
    setCurrentFiber(previousFiber);
  }
`);
  expect(js).not.toContain("restore");
  await restoresEach(dir);
  const later = compiled(
    "guard-one-field-set-later",
    `struct Restore(u32);

impl Drop for Restore {
    fn drop(&mut self) {
        println!("restore {}", self.0);
    }
}

pub fn run(n: u32) -> u32 {
    let mut previous = n;
    let _restore = Restore(previous);
    previous += 1;
    previous
}
`,
  );
  expect(later.js).toContain("const restore = previous;");
  expect(later.js).toContain("console.log(`restore ${restore}`);");
});
