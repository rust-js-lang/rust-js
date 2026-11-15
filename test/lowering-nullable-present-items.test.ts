// A thread-local a test proved `Some` where a closure is made, of a type
// with no interior mutability, so its initializer's value is every read's,
// read by the closure through `with`'s `&` parameter: `Some` wherever the
// closure runs, so its `unwrap()` is no `$unwrap`, MIR's answer
// (`NulledParams::given_present`, `present_item_read`, #1933 step 4 (G)).
// React DOM's `localRequestAnimationFrame`, `scheduleTimeout(..)`. Of the
// closure made in that closure, of `is_none()`'s false edge too; not of a
// thread-local whose `Option` holds a `Cell`, nor
// of a closure made on the edge where it's `None`, nor of one no test
// precedes, nor of one another way than the test's edge reaches, `||`'s. Each compiled and run; test/corpus/thread_local_tested_present_
// in_a_closure.rs runs them against native Rust.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const compile = (name: string, source: string) => {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const counts = join(dir, "decisions.tsv");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 120_000, { RUST_JS_DECISIONS: counts });
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const tidied = readFileSync(counts, "utf8")
    .split("\n")
    .map((line) => line.split("\t"))
    .filter((cells) => cells[0]!.startsWith("tidy-") && Number(cells[4]) > 0)
    .map((cells) => `${cells[0]} ${cells[1]}`);
  return { dir, js, tidied };
};

test("a thread-local a test proved Some where a closure is made is Some in the closure", async () => {
  const { dir, js, tidied } = compile(
    "nullable-present-items",
    `type Scheduled = fn(i32) -> i32;

fn double(x: i32) -> i32 {
    x * 2
}

thread_local! {
    static SCHEDULE: Option<Scheduled> = if "ab".len() > 1 { Some(double) } else { None };
    static FRAME: Option<Scheduled> = if SCHEDULE.with(Option::is_some) {
        Some(|x| SCHEDULE.with(|schedule| schedule.unwrap())(x) + 1)
    } else {
        None
    };
    static NOT_NONE: Option<Scheduled> = if SCHEDULE.with(Option::is_none) {
        None
    } else {
        Some(|x| SCHEDULE.with(|schedule| schedule.unwrap())(x) + 2)
    };
}

pub fn framed(x: i32) -> i32 {
    FRAME.with(|frame| frame.unwrap())(x)
}

pub fn not_none(x: i32) -> i32 {
    NOT_NONE.with(|frame| frame.unwrap())(x)
}
`,
  );
  expect(tidied).toEqual([]);
  expect(js).toContain("schedule != null ? (x) => (schedule(x) + 1) | 0 : undefined;");
  expect(js).toContain("(x) => (schedule(x) + 2) | 0");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.framed(20), lib.notNone(20)]).toEqual([41, 42]);
});

test("a thread-local of interior mutability, or one not tested where its closure is made, is unproved", async () => {
  const { dir, js, tidied } = compile(
    "nullable-present-items-unproved",
    `use std::cell::Cell;

type Scheduled = fn(i32) -> i32;

fn double(x: i32) -> i32 {
    x * 2
}

thread_local! {
    static COUNTER: Option<Cell<i32>> = if "ab".len() > 1 { Some(Cell::new(3)) } else { None };
    static COUNTED: Option<fn() -> i32> = if COUNTER.with(Option::is_some) {
        Some(|| COUNTER.with(|counter| counter.as_ref().unwrap().get()))
    } else {
        None
    };
    static MISSING: Option<Scheduled> = if "ab".len() > 5 { Some(double) } else { None };
    static ON_NONE: Option<Scheduled> = if MISSING.with(Option::is_none) {
        Some(|x| MISSING.with(|missing| missing.unwrap())(x))
    } else {
        None
    };
    static UNTESTED: Scheduled = |x| MISSING.with(|missing| missing.unwrap())(x);
    static EITHER: Option<Scheduled> = if MISSING.with(Option::is_some) || "ab".len() > 1 {
        Some(|x| MISSING.with(|missing| missing.unwrap())(x))
    } else {
        None
    };
}

pub fn counted() -> i32 {
    COUNTED.with(|counted| counted.unwrap())()
}

pub fn on_none(x: i32) -> bool {
    std::panic::catch_unwind(|| ON_NONE.with(|on_none| on_none.unwrap())(x)).is_err()
}

pub fn either(x: i32) -> bool {
    std::panic::catch_unwind(|| EITHER.with(|either| either.unwrap())(x)).is_err()
}

pub fn untested(x: i32) -> bool {
    std::panic::catch_unwind(|| UNTESTED.with(|untested| *untested)(x)).is_err()
}
`,
  );
  // Each MIR leaves as it is, `$unwrap`, which the printed JS's tidy may
  // read later, the `Cell`'s `counter` above all: not MIR's to prove.
  expect(tidied).toEqual(["tidy-present-in unwrap"]);
  expect(js).toContain("const ON_NONE = missing == null ? (x) => $unwrap(missing)(x) : undefined;");
  expect(js).toContain("const UNTESTED = (x) => $unwrap(missing)(x);");
  expect(js).toContain(": (x) => $unwrap(missing)(x);\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.counted(), lib.onNone(1), lib.untested(1), lib.either(1)]).toEqual([3, true, true, true]);
});
