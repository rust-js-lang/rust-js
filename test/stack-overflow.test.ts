// `js::catch_stack_overflow(f)` (ADR 1140): the JS engine's stack overflow,
// which no Rust program can catch, caught where a program asks, as React's
// Fizz recovers from a tree too deep to render. Inside one, the overflow is
// a panic whose payload is the engine's error: each frame's drops run as it
// unwinds, so a `RefCell` a frame borrowed is free again; a `catch_unwind`
// takes it, and `resume_unwind` gives it back; the innermost catches; and
// a drop that overflows as it unwinds aborts, as a second panic does.
// Outside one, it's Rust's abort (#1193). Natively, where an overflow
// aborts, it's `Ok(f())`, as it is here where nothing overflows.
import { beforeAll, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

thread_local! {
    static ENTERED: Cell<u64> = const { Cell::new(0) };
    static DROPPED: Cell<u64> = const { Cell::new(0) };
}

struct Counted;

impl Drop for Counted {
    fn drop(&mut self) {
        DROPPED.with(|d| d.set(d.get() + 1));
    }
}

fn deep(n: u64) -> u64 {
    if n == u64::MAX {
        return 0;
    }
    deep(n + 1) + 1
}

fn counted(n: u64) -> u64 {
    ENTERED.with(|e| e.set(e.get() + 1));
    let _counted = Counted;
    if n == 1000 {
        return deep(0).checked_add(1).unwrap();
    }
    counted(n + 1).checked_add(1).unwrap()
}

fn borrowing(cell: &RefCell<u64>) -> u64 {
    let mut held = cell.borrow_mut();
    *held += 1;
    counted(0) + *held
}

struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped");
    }
}

fn loud(n: u64) -> u64 {
    let _loud = Loud;
    if n == u64::MAX {
        return 0;
    }
    loud(n + 1).checked_add(1).unwrap()
}

struct Deeper;

impl Drop for Deeper {
    fn drop(&mut self) {
        deep(0).checked_add(1).unwrap();
    }
}

fn deep_holding(n: u64) -> u64 {
    let _deeper = Deeper;
    if n == u64::MAX {
        return 0;
    }
    deep_holding(n + 1) + 1
}

pub fn caught() -> String {
    let cell = RefCell::new(0);
    let result = js::catch_stack_overflow(AssertUnwindSafe(|| borrowing(&cell)));
    let all_dropped = ENTERED.with(|e| e.get()) == DROPPED.with(|d| d.get());
    format!("{} {} {}", result.is_err(), cell.try_borrow_mut().is_ok(), all_dropped)
}

pub fn caught_loud() {
    let result = js::catch_stack_overflow(|| loud(0));
    println!("{}", result.is_err());
}

pub fn taken_by_catch_unwind() -> String {
    let taken = js::catch_stack_overflow(|| catch_unwind(|| deep(0)).is_err());
    let resumed = js::catch_stack_overflow(|| {
        if let Err(payload) = catch_unwind(|| deep(0)) {
            resume_unwind(payload);
        }
    });
    format!("{:?} {}", taken.ok(), resumed.is_err())
}

pub fn nested() -> String {
    let result = js::catch_stack_overflow(|| js::catch_stack_overflow(|| deep(0)).is_err());
    format!("{:?}", result.ok())
}

pub fn returned() -> String {
    let panicked = js::catch_stack_overflow(|| catch_unwind(|| -> u32 { panic!("inner") }).is_err());
    let through = catch_unwind(|| js::catch_stack_overflow(|| -> u32 { panic!("through") })).is_err();
    format!("{:?} {:?} {}", js::catch_stack_overflow(|| 7).ok(), panicked.ok(), through)
}

pub fn overflowed_in_its_arm() {
    match js::catch_stack_overflow(|| deep(0)) {
        Ok(_) => println!("returned"),
        Err(_) => {
            loud(0);
        }
    }
}

pub fn overflowed_as_it_unwound() {
    let _ = js::catch_stack_overflow(|| deep_holding(0));
    println!("went on");
}

pub fn unwind_caught_outside() {
    let _ = catch_unwind(|| deep(0));
    println!("went on");
}
`;

function compiled(name: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  return dir;
}

function ran(dir: string, name: string) {
  const call = name.replace(/_(.)/g, (_, c: string) => c.toUpperCase());
  writeFileSync(join(dir, "main.js"), `import { ${call} } from "./lib.js";\nconst said = ${call}();\nif (said !== undefined) console.log(said);\n`);
  return runSync(["node", join(dir, "main.js")], dir, 60_000);
}

// The overflow caught: `Err`, the frames' drops run as it unwound, each
// `Counted` and the cell's guard, so it's free. The frames that hold them
// may panic, so each unwinds through its drops, and are a thousand above
// the overflow, where it leaves room to run them; a frame whose calls
// can't panic, or one the overflow leaves no room in, may skip its own.
// A drop the program sees, `Loud`'s, runs too, its scope's `finally`.
test("a stack overflow js::catch_stack_overflow catches unwinds as a panic does", () => {
  const dir = compiled("stack-overflow-caught");
  const p = ran(dir, "caught");
  expect(p.stderr).toBe("");
  expect(p.stdout).toBe("true true true\n");
  expect(p.code).toBe(0);
  const loud = ran(dir, "caught_loud");
  expect(loud.stdout).toContain("dropped\ndropped\n");
  expect(loud.stdout.endsWith("true\n")).toBe(true);
  expect(loud.code).toBe(0);
});

// Inside a catcher, a `catch_unwind` takes the overflow, as JS's `catch`
// does, and `resume_unwind` gives it back to the catcher; the innermost
// catcher catches; one where nothing overflows gives `Ok`.
test("inside js::catch_stack_overflow, catch_unwind takes an overflow, the innermost catches", () => {
  const dir = compiled("stack-overflow-innermost");
  expect(ran(dir, "taken_by_catch_unwind").stdout).toBe("Some(true) true\n");
  expect(ran(dir, "nested").stdout).toBe("Some(true)\n");
  // Where nothing overflows, `Ok` of what `f` gives; a panic in it is a
  // panic, which a `catch_unwind` inside it or around it takes.
  expect(ran(dir, "returned").stdout).toBe("Some(7) Some(true) true\n");
});

// What the program does with the `Result` runs after the catcher returns,
// outside it: an overflow there is Rust's abort, with no drop run. So is
// a drop that may panic and overflows as an overflow unwinds, as a panic
// in a drop as a panic unwinds is; a `catch_unwind` with no catcher open
// takes none, Rust's abort, as it was (#1193).
test("an overflow js::catch_stack_overflow can't take aborts", () => {
  const dir = compiled("stack-overflow-aborts");
  for (const call of ["overflowed_in_its_arm", "overflowed_as_it_unwound", "unwind_caught_outside"]) {
    const p = ran(dir, call);
    expect([call, p.stdout]).toEqual([call, ""]);
    expect(p.stderr).toContain("thread 'main' has overflowed its stack");
    expect(p.code).toBe(134);
  }
});
