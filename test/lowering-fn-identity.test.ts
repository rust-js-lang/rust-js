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

test("two boxes of a value of no size made a dyn Fn are apart, where Rust promises no address", async () => {
  const dir = fixture("fn-identity-zst");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::ptr::addr_eq;

fn double(x: u32) -> u32 {
    x * 2
}

fn make() -> Box<dyn Fn(u32) -> u32> {
    Box::new(|x| x + 1)
}

fn boxed<F: Fn(u32) -> u32 + 'static>(f: F) -> Box<dyn Fn(u32) -> u32> {
    Box::new(f)
}

pub fn items() -> bool {
    let first: Box<dyn Fn(u32) -> u32> = Box::new(double);
    let second: Box<dyn Fn(u32) -> u32> = Box::new(double);
    addr_eq(&*first, &*second)
}

pub fn made_twice() -> bool {
    let (first, second) = (make(), make());
    addr_eq(&*first, &*second)
}

pub fn generic() -> bool {
    let (first, second) = (boxed(double), boxed(double));
    addr_eq(&*first, &*second)
}

pub fn called() -> u32 {
    make()(1) + boxed(double)(2)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A function of its own where each is unsized (#1053).
  expect(js).toContain("function boxed(f) {\n  return (arg0) => f(arg0);\n}");
  // Native Rust's answers are all `true`: a `Box` of a value of no size
  // allocates nothing, and its address is no allocation's, which Rust
  // promises nothing of. JS's are `false`, each box apart, as two that
  // allocate are (ADR 0262's second case, the user's decision, #1053).
  const lib = await import(join(dir, "lib.js"));
  expect([lib.items(), lib.madeTwice(), lib.generic(), lib.called()]).toEqual([false, false, false, 6]);
});

// A copy of a `Copy` closure that captures something is a function of its
// own, `c1.bind()`, only where the crate reads a `dyn Fn`'s address, which
// tells the copy's place from the original's (#1090, the user's decision);
// elsewhere the copy is the function, as nothing can tell them apart.
test("a copy of a capturing closure is a function of its own only where a dyn Fn's address is read", async () => {
  const dir = fixture("fn-identity-copied-closure");
  const program = (compared: boolean) => `pub fn copies() -> String {
    let n = 3;
    let c1 = move || n + 1;
    let c2 = c1;
    ${compared ? "let same = std::ptr::eq(&c1 as &dyn Fn() -> i32, &c2 as &dyn Fn() -> i32);" : "let same = false;"}
    format!("{} {}", same, c1() + c2())
}
`;
  writeFileSync(join(dir, "lib.rs"), program(true));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const compared = readFileSync(join(dir, "lib.js"), "utf8");
  expect(compared).toContain("const c2 = c1.bind();");
  expect((await import(join(dir, "lib.js"))).copies()).toBe("false 8");
  writeFileSync(join(dir, "lib.rs"), program(false));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "plain.js")]);
  const plain = readFileSync(join(dir, "plain.js"), "utf8");
  expect(plain).not.toContain(".bind()");
  expect(plain).toContain("const c2 = c1;");
});
