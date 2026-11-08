// What a function pointer is, by identity (ADR 0395): where Rust gives its
// answer, JS's `===` gives it; where Rust leaves it unspecified, as rustc's
// `unpredictable_function_pointer_comparisons` lint and std's `fn_addr_eq`
// say, a generic function's instance may be two functions, and JS's answer
// is today's, pinned here so it can't change unseen (ADR 0262's
// Consequences, the user's decision, 2026-11-08, #987).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

test("a generic function's instance taken in generic code is made where it's taken, and one per module", async () => {
  const dir = fixture("fn-identity");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::fmt::Debug;
use std::ptr::fn_addr_eq;

fn shown<T: Debug>(value: &T) -> String {
    format!("<{value:?}>")
}

fn pick<T: Debug>() -> fn(&T) -> String {
    shown::<T>
}

fn get() -> fn(&u8) -> String {
    pick::<u8>()
}

mod other {
    pub fn direct() -> fn(&u8) -> String {
        super::shown::<u8>
    }
}

pub fn in_one_body() -> bool {
    fn_addr_eq(pick::<u8>(), pick::<u8>())
}

pub fn across_calls() -> bool {
    fn_addr_eq(get(), get())
}

pub fn across_modules() -> bool {
    fn_addr_eq(shown::<u8> as fn(&u8) -> String, other::direct())
}

pub fn in_one_module() -> bool {
    fn_addr_eq(shown::<u8> as fn(&u8) -> String, shown::<u8> as fn(&u8) -> String)
}

pub fn shows() -> String {
    pick::<u8>()(&7)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // The arrow that gives it its caller's dictionaries, made each time the
  // function that takes it runs.
  expect(js).toContain("function pick(TDebug) {\n  return (arg0) => shown(arg0, TDebug);\n}");
  // Native Rust's answers are all `true` with one codegen unit; Rust
  // promises none of them but the last.
  const lib = await import(join(dir, "lib.js"));
  expect([lib.inOneBody(), lib.acrossCalls(), lib.acrossModules(), lib.inOneModule(), lib.shows()]).toEqual([false, false, false, true, "<7>"]);
});
