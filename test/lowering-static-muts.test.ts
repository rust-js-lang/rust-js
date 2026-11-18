// What a `static mut` is to the modules that read and write it: its
// module's `let`, exported where it's `pub` and its module alone writes
// it, refused where another module writes it or a pointer to it is given
// on (ADR 0096, amended). Split from lowering-module-variables.test.ts by
// concept, at its cap.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const flags = `use std::cell::{Cell, RefCell};

thread_local! {
    pub static ENABLED: Cell<bool> = const { Cell::new(false) };
    pub(crate) static COUNT: Cell<u32> = const { Cell::new(0) };
    pub static LIMIT: Cell<u32> = const { Cell::new(4) };
    pub static LABEL: RefCell<String> = const { RefCell::new(String::new()) };
    pub static SEEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub fn enable(label: &str) {
    ENABLED.set(true);
    COUNT.set(COUNT.get() + 1);
    LABEL.set(label.to_string());
}
`;

// A `static mut` is its module's variable as an exported cell is (ADR 0096,
// amended): `export let` where it's `pub`, written only in its module and
// read live by its importers, JS's and Rust's.
const counter = `#[derive(Clone, Copy)]
pub struct Stats {
    pub calls: u32,
}

pub static mut LINES: usize = 0;
pub static mut STATS: Stats = Stats { calls: 0 };
pub static mut SLOTS: [u8; 2] = [0; 2];
pub static mut LIMIT: u32 = 4;

pub fn line() -> usize {
    unsafe {
        LINES += 1;
        STATS.calls += 1;
        SLOTS[LINES % 2] += 1;
        LINES
    }
}

pub fn reset() {
    unsafe {
        LINES = 0;
        STATS = Stats { calls: 0 };
    }
}
`;

test("a pub static mut its module alone writes is its export let, which importers read", async () => {
  const dir = fixture("static-mut-let");
  writeFileSync(join(dir, "lib.rs"), "pub mod counter;\npub mod users;\n");
  writeFileSync(join(dir, "counter.rs"), counter);
  writeFileSync(join(dir, "users.rs"), `use crate::counter;

pub fn told() -> (usize, u32, u8, u32) {
    unsafe { (counter::LINES, counter::STATS.calls, counter::SLOTS[1], counter::LIMIT) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const counterJs = readFileSync(join(dir, "counter.js"), "utf8");
  expect(counterJs).toContain("export let LINES = 0;\nexport let STATS = { calls: 0 };\nexport const SLOTS = [0, 0];\nexport const LIMIT = 4;\n");
  expect(counterJs).toContain("  LINES = (LINES + 1) >>> 0;\n  STATS.calls = (STATS.calls + 1) >>> 0;\n");
  expect(counterJs).toContain("  LINES = 0;\n  STATS = { calls: 0 };\n");
  expect(counterJs).not.toContain(".value");
  const usersJs = readFileSync(join(dir, "users.js"), "utf8");
  expect(usersJs).toContain("  return [Counter.LINES, Counter.STATS.calls, Counter.SLOTS[1], Counter.LIMIT];\n");
  expect(usersJs).not.toContain(".value");
  expect(readFileSync(join(dir, "counter.d.ts"), "utf8")).toContain("export let LINES: number;\n\nexport let STATS: Stats;\n\nexport const SLOTS: number[];\n\nexport const LIMIT: number;\n");
  // A JS module of the program's imports it and reads what it is now.
  writeFileSync(join(dir, "reader.js"), 'import { LINES, STATS } from "./counter.js";\nexport const read = () => [LINES, STATS.calls];\n');
  const reader = await import(join(dir, "reader.js"));
  const counterModule = await import(join(dir, "counter.js"));
  const users = await import(join(dir, "users.js"));
  expect(reader.read()).toEqual([0, 0]);
  expect([counterModule.line(), counterModule.line(), counterModule.line()]).toEqual([1, 2, 3]);
  expect(reader.read()).toEqual([3, 3]);
  expect(users.told()).toEqual([3, 3, 2, 4]);
  counterModule.reset();
  expect(reader.read()).toEqual([0, 0]);
});

// A private one is its module's `let`; one a module beneath it reads is
// exported to it, its `export let`, which that module reads by its name.
test("a private static mut is its module's let", async () => {
  const dir = fixture("static-mut-private");
  writeFileSync(join(dir, "lib.rs"), `static mut HITS: u32 = 0;
static mut SEEN: u32 = 0;

pub mod child;

pub fn hit() -> u32 {
    unsafe {
        HITS += 1;
        SEEN += 2;
        HITS
    }
}
`);
  writeFileSync(join(dir, "child.rs"), "pub fn seen() -> u32 {\n    unsafe { super::SEEN }\n}\n");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const libJs = readFileSync(join(dir, "lib.js"), "utf8");
  expect(libJs).toContain("let HITS = 0;\nexport let SEEN = 0;\n");
  expect(libJs).toContain("  HITS = (HITS + 1) >>> 0;\n  SEEN = (SEEN + 2) >>> 0;\n  return HITS;\n");
  expect(readFileSync(join(dir, "child.js"), "utf8")).toContain("  return SEEN;\n");
  const lib = await import(join(dir, "lib.js"));
  const child = await import(join(dir, "child.js"));
  expect([lib.hit(), lib.hit(), child.seen()]).toEqual([1, 2, 4]);
});

// JS can't set another module's `let`: a write of a `static mut` from
// another module, whole, a field's, an item's, or a pointer taken to it,
// is refused, saying to write a function of its module that sets it.
test("another module's write of a static mut is refused", () => {
  const dir = fixture("static-mut-written");
  const compiled = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), "pub mod counter;\npub mod users;\n");
    writeFileSync(join(dir, name, "counter.rs"), counter);
    writeFileSync(join(dir, name, "users.rs"), `use crate::counter;

pub fn written() -> usize {
    unsafe {
        ${body}
    }
}
`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js")], root, 120_000);
  };
  for (const [name, body] of [
    ["assigned", "counter::LINES = 3;\n        0"],
    ["added-to", "counter::LINES += 1;\n        0"],
    ["field", "counter::STATS.calls = 1;\n        0"],
    ["item", "counter::SLOTS[0] = 1;\n        0"],
    ["raw-mut", "let lines = &raw mut counter::LINES;\n        *lines = 2;\n        0"],
    ["raw-const", "*std::ptr::addr_of!(counter::LINES)"],
  ]) {
    const out = compiled(name, body);
    expect([name, out.code === 0, out.stderr]).toEqual([
      name,
      false,
      expect.stringContaining("is its module's variable, which JS lets only that module set: write a `pub fn` in that module that sets it"),
    ]);
  }
  const read = compiled("read", "counter::LINES + counter::STATS.calls as usize + usize::from(counter::SLOTS[0])");
  expect([read.code, read.stderr]).toEqual([0, ""]);
});

// A library's `static mut` is its `export let` to its consumers too, whose
// manifest says so: a consumer reads it by its name and can't set it.
test("a library's static mut is a variable its consumers read and can't set", async () => {
  const dir = fixture("static-mut-library");
  const manifest = join(dir, "dep", "lib.manifest.json"), metadata = join(dir, "libdep.rmeta");
  writeFileSync(join(dir, "dep.rs"), `pub static mut LINES: usize = 0;

pub fn line() -> usize {
    unsafe {
        LINES += 1;
        LINES
    }
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", manifest,
    "--", "--crate-name", "dep", `--emit=metadata=${metadata}`]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("export let LINES = 0;\n");
  const app = (name: string, body: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `pub fn lines() -> usize {\n    unsafe {\n        ${body}\n    }\n}\n`);
    return runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js"), "--dependency", manifest,
      "--", "--crate-name", name, "--extern", `dep=${metadata}`, "-L", dir], root, 120_000);
  };
  const reads = app("reads", "dep::LINES");
  expect([reads.code, reads.stderr]).toEqual([0, ""]);
  expect(readFileSync(join(dir, "reads", "lib.js"), "utf8")).toContain("  return Dep.LINES;\n");
  const { lines } = await import(join(dir, "reads", "lib.js"));
  const { line } = await import(join(dir, "dep", "lib.js"));
  expect(lines()).toBe(0);
  line();
  expect(lines()).toBe(1);
  for (const [name, body] of [["sets", "dep::LINES = 5;\n        0"], ["points", "*&raw const dep::LINES"]]) {
    const out = app(name, body);
    expect([name, out.code === 0, out.stderr]).toEqual([name, false, expect.stringContaining("is its module's variable, which JS lets only that module set")]);
  }
});

// A pointer to a `static mut` given on, `&raw mut TOTAL`, is a handle on
// its place, which needs its box, `{ value }`: one its module keeps is its
// box; one it exports, which a JS importer would read as the box, is
// refused at its declaration. One only written through where it's taken,
// `*addr_of_mut!(COUNT) += 2`, is the variable written.
test("a static mut a pointer to is given on is its box, refused where it's exported", async () => {
  const dir = fixture("static-mut-pointer");
  const compiled = (name: string, visibility: string) => {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, name, "lib.rs"), `use std::ptr::addr_of_mut;

${visibility}static mut COUNT: u32 = 0;
${visibility}static mut TOTAL: u32 = 0;

fn add(total: *mut u32, n: u32) {
    unsafe { *total += n }
}

pub fn counted() -> (u32, u32) {
    unsafe {
        *addr_of_mut!(COUNT) += 2;
        add(&raw mut TOTAL, 3);
        add(&raw mut TOTAL, 4);
        (COUNT, TOTAL)
    }
}
`);
    const out = runSync([compiler, join(dir, name, "lib.rs"), "-o", join(dir, name, "lib.js")], root, 120_000);
    return [out.code, out.stderr, out.code === 0 ? readFileSync(join(dir, name, "lib.js"), "utf8") : ""];
  };
  const [code, stderr, libJs] = compiled("kept", "");
  expect([code, stderr]).toEqual([0, ""]);
  expect(libJs).toContain("let COUNT = 0;\nconst TOTAL = { value: 0 };\n");
  expect(libJs).toContain("  COUNT = (COUNT + 2) >>> 0;\n");
  const { counted } = await import(join(dir, "kept", "lib.js"));
  expect([counted(), counted()]).toEqual([[2, 7], [4, 14]]);
  const [exportedCode, exportedErr] = compiled("exported", "pub ");
  expect([exportedCode === 0, exportedErr]).toEqual([
    false,
    expect.stringContaining("`TOTAL` is exported, so a JS module imports it as its module's `export let`, but its module gives on a pointer to it"),
  ]);
  expect(exportedErr).not.toContain("`COUNT`");
});
