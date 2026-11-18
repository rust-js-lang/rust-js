// A `#[repr]` enum: each variant its number, and its variants its
// module's consts, read by name. Split from lowering-structs-enums.test.ts
// by concept, at its cap.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0722: a fieldless enum under a `#[repr(..)]` of an integer is its
// number, `ArrayKind::Empty` `0`, as React's `const EMPTY_ARRAY = 0;` is;
// its `as` the value, its `{:?}` its name, a `.d.ts` its numbers. One
// without a `repr`, or of 64 bits, its name, as ever.
test("a repr enum's variant is its number", async () => {
  const dir = fixture("repr-enum-numbers");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum ArrayKind {
    Empty = 0,
    Complex = 1,
    Primitive = 2,
    Entries = 3,
}

#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub enum Sparse {
    Low = -1,
    High = 10,
}

#[derive(Clone, Copy, Debug)]
#[repr(u64)]
pub enum Wide {
    Small = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plain {
    Empty,
}

pub fn kind(n: usize) -> ArrayKind {
    if n == 0 { ArrayKind::Empty } else { ArrayKind::Entries }
}

pub fn is_empty(kind: ArrayKind) -> bool {
    kind == ArrayKind::Empty
}

pub fn code(kind: ArrayKind) -> u8 {
    kind as u8
}

pub fn shown(kind: ArrayKind, sparse: Sparse) -> String {
    format!("{kind:?} {sparse:?}")
}

pub fn others(wide: Wide, plain: Plain) -> String {
    format!("{wide:?} {plain:?}")
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Cell<'a> {
    Kind(ArrayKind),
    Text(&'a str),
}

pub fn cell(cell: Cell) -> String {
    match cell {
        Cell::Kind(kind) => format!("{kind:?}"),
        Cell::Text(text) => text.to_string(),
    }
}

pub fn name(kind: ArrayKind) -> &'static str {
    match kind {
        ArrayKind::Empty => "Empty",
        ArrayKind::Complex => "Complex",
        ArrayKind::Primitive => "Primitive",
        ArrayKind::Entries => "Entries",
    }
}

pub fn named(n: usize) -> String {
    name(kind(n)).to_string()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Each variant its module's `const`, read by its name (ADR 0722, amended).
  expect(js).toContain("export const Empty = 0;\nexport const Complex = 1;\nexport const Primitive = 2;\nexport const Entries = 3;\n");
  expect(js).toContain("export const Low = -1;\nexport const High = 10;\n");
  expect(js).toContain("return n === 0 ? Empty : Entries;");
  expect(js).toContain("return kind === Empty;");
  expect(js).toContain("export function code(kind) {\n  return kind;\n}");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type ArrayKind = 0 | 1 | 2 | 3;");
  expect(declarations).toContain("export const Empty: 0;");
  expect(declarations).toContain("export const Low: -1;");
  expect(declarations).toContain('export type Plain = "Empty";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.kind(0), lib.kind(4), lib.isEmpty(0), lib.code(3)]).toEqual([0, 3, true, 3]);
  expect([lib.Empty, lib.Entries, lib.Low, lib.High]).toEqual([0, 3, -1, 10]);
  expect([lib.shown(2, -1), lib.shown(0, 10), lib.others("Small", "Empty")]).toEqual(["Primitive Low", "Empty High", "Small Empty"]);
  // A function giving each variant its name gives a name, not the number.
  expect([lib.name(0), lib.name(3), lib.named(0), lib.named(1)]).toEqual(["Empty", "Entries", "Empty", "Entries"]);
  // Shown by its number, an untagged enum's payload told as a number.
  expect(js).toContain('["Empty", "Complex", "Primitive", "Entries"][kind]');
  expect([lib.cell(3), lib.cell("x")]).toEqual(["Entries", "x"]);
});

// React's `ReactLazy` keeps a lazy component's status as module constants,
// `const Pending = 0;`, each read by its name: a `#[repr]` enum's variant
// is its module's `const`, as a `const` item is (ADRs 0031, 0722): tested,
// set, matched by a `switch`'s `case Pending:` and cast, the cast its
// number. Named apart as any of a module's names, two enums' `Pending`
// `Pending` and `Pending$1`; another module's imported by its name.
test("a repr enum's variants are its module's consts, each read by its name", async () => {
  const dir = fixture("repr-enum-consts");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

#[derive(Clone, Copy, PartialEq)]
#[repr(i8)]
enum Status {
    Uninitialized = -1,
    Pending = 0,
    Resolved = 1,
    Rejected = 2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
enum Request {
    Pending = 5,
    Done = 6,
}

pub struct Payload {
    status: Cell<Status>,
}

pub fn lazy() -> Payload {
    Payload { status: Cell::new(Status::Uninitialized) }
}

pub fn settle(payload: &Payload, ok: bool) -> i32 {
    if payload.status.get() == Status::Uninitialized {
        payload.status.set(Status::Pending);
    }
    if matches!(payload.status.get(), Status::Pending | Status::Uninitialized) {
        payload.status.set(if ok { Status::Resolved } else { Status::Rejected });
    }
    payload.status.get() as i32
}

pub fn describe(payload: &Payload) -> &'static str {
    match payload.status.get() {
        Status::Uninitialized => "uninitialized",
        Status::Pending => "pending",
        Status::Resolved => "resolved",
        Status::Rejected => "rejected",
    }
}

pub fn code(done: bool) -> i32 {
    let request = if done { Request::Done } else { Request::Pending };
    Status::Pending as i32 + request as i32
}

pub fn requests() -> String {
    format!("{:?}", [Some(Request::Pending), None])
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum Cache {
    #[cfg_attr(rust_js, rust_js::name = "UNTERMINATED")]
    Unterminated = 0,
    #[cfg_attr(rust_js, rust_js::name = "TERMINATED")]
    Terminated = 1,
}

pub fn cached(done: bool) -> bool {
    let status = if done { Cache::Terminated } else { Cache::Unterminated };
    status == Cache::Terminated
}

pub mod theme {
    #[derive(Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mode {
        Light = 1,
        Dark = 2,
    }
}

pub fn mode(dark: bool) -> theme::Mode {
    if dark { theme::Mode::Dark } else { theme::Mode::Light }
}

mod motion {
    #[derive(Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub(crate) enum Speed {
        Slow = 1,
        Fast = 2,
    }

    pub(crate) fn speed(fast: bool) -> Speed {
        if fast { Speed::Fast } else { Speed::Slow }
    }
}

pub fn pace(fast: bool) -> &'static str {
    match motion::speed(fast) {
        motion::Speed::Slow => "slow",
        motion::Speed::Fast => "fast",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const Uninitialized = -1;\nconst Pending = 0;\nconst Resolved = 1;\nconst Rejected = 2;\n");
  expect(js).toContain("const Pending$1 = 5;\nconst Done = 6;\n");
  expect(js).toContain('import { Dark, Light } from "./theme.js";');
  expect(js).toContain("return { status: Uninitialized };");
  expect(js).toContain("if (payload.status === Uninitialized) {\n    payload.status = Pending;\n  }");
  expect(js).toContain("if (payload.status === Pending || payload.status === Uninitialized) {\n    payload.status = ok ? Resolved : Rejected;\n  }");
  expect(js).toContain("case Uninitialized:");
  expect(js).toContain("case Rejected:");
  // A constant rustc made of one, `[Some(Request::Pending), None]`'s.
  expect(js).toContain("[Pending$1, undefined]");
  expect(js).toContain("const request = done ? Done : Pending$1;");
  expect(js).toContain("return dark ? Dark : Light;");
  expect(readFileSync(join(dir, "theme.js"), "utf8")).toContain("export const Light = 1;\nexport const Dark = 2;\n");
  // Another module's not `pub`, exported as it's read there; the last
  // arm's, whose test an exhaustive `match` needn't write, not imported.
  // Its function called through its module, its namespace's (ADR 1018).
  expect(js).toContain('import { Slow } from "./motion.js";\nimport * as Motion from "./motion.js";');
  expect(js).toContain('  return Motion.speed(fast) === Slow ? "slow" : "fast";\n');
  expect(readFileSync(join(dir, "motion.js"), "utf8")).toContain("export const Slow = 1;\nexport const Fast = 2;\n");
  const lib = await import(join(dir, "lib.js"));
  const [first, second] = [lib.lazy(), lib.lazy()];
  expect([lib.describe(first), lib.settle(first, true), lib.describe(first)]).toEqual(["uninitialized", 1, "resolved"]);
  expect([lib.settle(second, false), lib.describe(second)]).toEqual([2, "rejected"]);
  expect([lib.code(false), lib.code(true), lib.requests(), lib.mode(true), lib.mode(false)]).toEqual([5, 6, "[Some(Pending), None]", 2, 1]);
  expect([lib.pace(true), lib.pace(false)]).toEqual(["fast", "slow"]);
  // A variant's `#[rust_js::name]` names its `const`, as React's cache's
  // `const UNTERMINATED = 0;`; one that isn't a JS name is refused.
  expect(js).toContain("const UNTERMINATED = 0;\nconst TERMINATED = 1;\n");
  expect(js).toContain("const status = done ? TERMINATED : UNTERMINATED;");
  expect([lib.cached(true), lib.cached(false)]).toEqual([true, false]);
  writeFileSync(join(dir, "named.rs"), '#[repr(u8)]\npub enum Cache {\n    #[cfg_attr(rust_js, rust_js::name = "un-terminated")]\n    Unterminated = 0,\n}\npub fn cache() -> Cache {\n    Cache::Unterminated\n}\n');
  expect(() => run([compiler, join(dir, "named.rs"), "-o", join(dir, "named.js")])).toThrow("so it's a JS name: `un-terminated` isn't one");
});
