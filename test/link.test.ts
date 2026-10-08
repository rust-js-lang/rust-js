import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, buildWebapi, compiler, fixture, run, target } from "./support";

beforeAll(buildCompiler, 600_000);

test("late import aliases avoid locals, nested parameters and generated bindings", async () => {
  const dir = fixture("link-names");
  const source = `
    pub mod util { pub fn add(n: i32) -> i32 { n + 1 } }
    pub mod value { pub fn add(n: i32) -> i32 { super::util::add(n) + 1 } }
    pub fn run() -> i32 {
      let add = 10;
      let value = 20;
      let f = |add: i32| self::util::add(add) + self::value::add(value);
      f(add)
    }
    pub fn text() -> String { format!("answer={}", util::add(1)) }
  `;
  writeFileSync(join(dir, "lib.rs"), source);
  writeFileSync(join(dir, "native.rs"), 'mod lib; fn main() { println!("{}", lib::run()); println!("{}", lib::text()); }');
  run(["rustc", "--edition=2024", "-Awarnings", join(dir, "native.rs"), "-o", join(dir, "native")]);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = await import(join(dir, "lib.js"));
  expect([String(js.run()), js.text()]).toEqual(run([join(dir, "native")]).trim().split("\n"));
  const code = readFileSync(join(dir, "lib.js"), "utf8");
  expect(code).toContain('import { add as add$2 } from "./util.js";');
  expect(code).toContain('import { add as add$3 } from "./value.js";');
  expect(code).not.toContain("\0");
});

test("a sparse cyclic graph emits only each module's actual imports", async () => {
  const dir = fixture("link-graph"), count = 32;
  writeFileSync(join(dir, "lib.rs"), Array.from({ length: count }, (_, i) => `
    pub mod m${i} { pub fn f(n: u32) -> u32 { if n == 0 { ${i} } else { super::m${(i + 1) % count}::f(n - 1) } } }
  `).join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const first = await import(join(dir, "m0.js"));
  expect(first.f(47)).toBe(15);
  for (const file of readdirSync(dir).filter(name => /^m\d+\.js$/.test(name))) {
    const code = readFileSync(join(dir, file), "utf8");
    expect(code.match(/^import /gm)?.length).toBe(1);
    expect(code).not.toContain("\0");
  }
});

test("derived Debug dependencies are retained transitively without unused implementations", async () => {
  const dir = fixture("derived-reachability");
  writeFileSync(join(dir, "lib.rs"), `
    #[derive(Debug)] pub struct Leaf(u32);
    #[derive(Debug)] pub struct Branch(Leaf);
    #[derive(Debug)] pub struct Unused(u32);
    pub fn report() -> String { format!("{:?}", Branch(Leaf(7))) }
  `);
  writeFileSync(join(dir, "native.rs"), 'mod lib; fn main() { println!("{}", lib::report()); }');
  run(["rustc", "--edition=2024", "-Awarnings", join(dir, "native.rs"), "-o", join(dir, "native")]);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const generated = await import(join(dir, "lib.js"));
  expect(generated.report()).toBe(run([join(dir, "native")]).trim());
  expect(readFileSync(join(dir, "lib.js"), "utf8")).not.toContain("Unused");
});

// Each module imports its own operations from @rust-js/runtime (ADR 0103);
// what those use, `$siftUp` and `$siftDown`, the package has for them.
test("heap modules import only their operations, and the package has what those use", async () => {
  const dir = fixture("heap-runtime-dependencies");
  writeFileSync(join(dir, "lib.rs"), `
    pub mod push_pop {
      use std::collections::BinaryHeap;
      pub fn run() -> Vec<i32> {
        let mut heap = BinaryHeap::new();
        for n in [3, 1, 8, 2, 8] { heap.push(n); }
        let mut out = Vec::new();
        while let Some(n) = heap.pop() { out.push(n); }
        out
      }
    }
    pub mod sorted {
      use std::collections::BinaryHeap;
      pub fn run() -> Vec<i32> { BinaryHeap::from(vec![3, 1, 8, 2, 8]).into_sorted_vec() }
    }
  `);
  writeFileSync(join(dir, "native.rs"), 'mod lib; fn main() { println!("{:?}", lib::push_pop::run()); println!("{:?}", lib::sorted::run()); }');
  run(["rustc", "--edition=2024", "-Awarnings", join(dir, "native.rs"), "-o", join(dir, "native")]);
  const expected = run([join(dir, "native")]).trim().split("\n").map(line => JSON.parse(line));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  for (const [i, name, imports] of [[0, "push_pop", "$cmp, $heapPop, $heapPush"], [1, "sorted", "$cmp, $heapFrom, $heapSorted"]] as const) {
    const file = join(dir, `${name}.js`);
    expect((await import(file)).run()).toEqual(expected[i]);
    const code = readFileSync(file, "utf8");
    expect(code).toContain(`import { ${imports} } from "@rust-js/runtime";`);
    expect(code).not.toContain("$sift");
  }
});

// What a module imports of the package is the helpers its code calls, not
// names its strings happen to spell: `"$bigCountOnes"` is text (found in
// review: the printed JS was scanned for them).
test("a string that spells a helper's name doesn't import it", () => {
  const dir = fixture("helper-names-in-strings");
  writeFileSync(join(dir, "main.rs"), `
    fn main() {
      let n: u64 = 5;
      println!("{}", n.leading_zeros());
      println!("$bigTrailingZeros and $bigCountOnes");
      println!("$bigCountOnes");
    }
  `);
  run([compiler, join(dir, "main.rs"), "-o", join(dir, "main.js")]);
  const code = readFileSync(join(dir, "main.js"), "utf8");
  expect(code).toContain(`import { $bigLeadingZeros } from "@rust-js/runtime";`);
  expect(code).toContain(`"$bigTrailingZeros and $bigCountOnes"`);
  expect(code).toContain(`console.log("$bigCountOnes")`);
});

// Module-load bodies are linked with functions and constants, including the
// names they bind at module scope (ADRs 0069, 0267).
test("module-load statements resolve imports around their local bindings", async () => {
  buildWebapi();
  const dir = fixture("link-on-load");
  writeFileSync(join(dir, "lib.rs"), `
    mod util { pub fn value(n: u32) -> u32 { n + 1 } }
    unsafe extern "Rust" {
      #[link_name = "globalThis.auditSeed"]
      safe fn seed() -> u32;
      #[link_name = "globalThis.auditLoaded"]
      safe fn loaded(n: u32);
    }
    js::on_load! {
      let value = seed();
      loaded(value);
      let read = |value: u32| util::value(value);
      loaded(read(value));
    }
    pub fn ready() -> u32 { util::value(1) }
  `);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"),
    "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const code = readFileSync(join(dir, "lib.js"), "utf8");
  expect(code).toContain('import { value as value$1 } from "./util.js";');
  expect(code).not.toContain("\0");
  const seen: number[] = [];
  const globals = globalThis as typeof globalThis & { auditSeed?: () => number; auditLoaded?: (n: number) => void };
  globals.auditSeed = () => 3;
  globals.auditLoaded = n => seen.push(n);
  try {
    const generated = await import(join(dir, "lib.js"));
    expect(seen).toEqual([3, 4]);
    expect(generated.ready()).toBe(2);
  } finally {
    delete globals.auditSeed;
    delete globals.auditLoaded;
  }
});
