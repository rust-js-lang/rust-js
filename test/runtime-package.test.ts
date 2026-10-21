// The runtime as a package (ADR 0103): `@rust-js/runtime`, released with
// the compiler, as ReScript's `@rescript/runtime` is. A module imports the
// helpers its code names from it, instead of carrying its own copy of each.

import { beforeAll, expect, test } from "bun:test";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

// The package's module is the compiler's helpers, each exported: made by
// the compiler, so it can't drift from what the compiler's imports name.
test("the committed @rust-js/runtime is the compiler's helpers, each exported once", () => {
  const printed = run([compiler, "--runtime-module"]);
  expect(readFileSync(join(root, "runtime", "index.js"), "utf8")).toBe(printed);
  const names = [...printed.matchAll(/^export (?:async )?(?:function\*?|class|const|let) (\$[\w$]+)/gm)].map((m) => m[1]);
  expect(names.length).toBeGreaterThan(100);
  expect(new Set(names).size).toBe(names.length);
  const pkg = JSON.parse(readFileSync(join(root, "runtime", "package.json"), "utf8"));
  const version = readFileSync(join(root, "Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)![1];
  expect([pkg.name, pkg.version]).toEqual(["@rust-js/runtime", version]);
});

// The helpers the compiler takes for a JS iterator as they're made, and
// writes no `Iterator.from` around, are `$iterator`, `$fuse` (ADR 0473),
// `$intoIter` (ADR 0475), `$cycleOf` (ADR 0491), `$repeating` and
// `$repeatingN` (ADR 0513), `$rev` (ADR 0516), `$closing` (ADR 0518),
// `$mapEnds` (ADR 0455), `$filterEnds` (ADR 0526),
// `$enumerate` (ADR 0528), `$memchrIter` (ADR 0529) and `$stdIterator`, a
// compiled std struct's (ADR 0561), and each of the runtime's generators
// (ADR 0472): the compiler's list, held to the runtime.
test("the compiler's iterator helpers are $iterator, $fuse, $intoIter, $cycleOf, $repeating, $repeatingN, $rev, $closing, $mapEnds, $filterEnds, $enumerate, $memchrIter, $stdIterator and the runtime's generators", () => {
  const printed = run([compiler, "--runtime-module"]);
  const generators = [...printed.matchAll(/^export function\* (\$[\w$]+)/gm)].map((m) => m[1]);
  const source = readFileSync(join(root, "src", "lower", "iterators.rs"), "utf8");
  const list = source.match(/pub\(crate\) const ITERATOR_HELPERS: \[&str; \d+\] = \[([^\]]*)\]/)![1];
  // A range's, named where ranges are.
  const range = readFileSync(join(root, "src", "lower", "std_types", "range.rs"), "utf8");
  const lazyRange = range.match(/pub\(crate\) const LAZY_RANGE: &str = "(\$\w+)";/)![1];
  const listed = [...list.replace("super::std_types::range::LAZY_RANGE", `"${lazyRange}"`).matchAll(/"(\$[\w$]+)"/g)].map(
    (m) => m[1],
  );
  expect(listed.toSorted()).toEqual(
    [
      "$iterator",
      "$fuse",
      "$intoIter",
      "$cycleOf",
      "$repeating",
      "$repeatingN",
      "$rev",
      "$closing",
      "$mapEnds",
      "$filterEnds",
      "$enumerate",
      "$memchrIter",
      "$stdIterator",
      ...generators,
    ].toSorted(),
  );
});

// Each helper is a file of its own, `src/runtime/<name>.js`, that the
// compiler reads in: JS that parses as JS, and none left behind unread.
test("each helper's file parses as JavaScript, and the compiler reads each", () => {
  const dir = join(root, "src", "runtime");
  const files = readdirSync(dir).filter((file) => file.endsWith(".js"));
  const rust = readFileSync(join(root, "src", "runtime.rs"), "utf8");
  expect(files.length).toBeGreaterThan(150);
  for (const file of files) {
    expect(rust, file).toContain(`include_str!("runtime/${file}")`);
    // Compiled, and never run.
    expect(() => new Function(readFileSync(join(dir, file), "utf8")), file).not.toThrow();
  }
});

// A module names a few helpers, and imports just those; one a helper uses,
// the package has for it.
test("a module imports the helpers it names from @rust-js/runtime, and runs", () => {
  const dir = fixture("runtime-package");
  writeFileSync(join(dir, "lib.rs"), `pub fn main() {
    let names = vec!["ada", "grace"];
    let i = names.len() - 1;
    println!("{:?} {}", names[i], 7 / (i as i32));
    let parsed: Result<u32, _> = "x".parse();
    println!("{:?}", parsed.is_err());
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const js = readFileSync(out, "utf8");
  expect(js).toMatch(/^import \{ [^}]+ \} from "@rust-js\/runtime";$/m);
  expect(js).not.toMatch(/^function \$/m);
  const imported = js.match(/^import \{ ([^}]+) \} from "@rust-js\/runtime";$/m)![1].split(", ");
  expect(imported).toEqual([...imported].sort());
  for (const name of imported) expect(js.split("\n").slice(3).join("\n")).toContain(name);
  const printed = run([node ?? "node", "--input-type=module", "--eval", `(await import(${JSON.stringify(out)})).main();`]);
  expect(printed).toBe('"grace" 7\ntrue\n');
});

// A `OnceLock`'s or `LazyLock`'s init that uses its own cell deadlocks in
// Rust, which JS can't do: rust-js says so, never a std-looking panic
// (ADRs 0317, 0318).
test("a lock's init that uses its own lock is rust-js's error, not a deadlock", () => {
  const dir = fixture("runtime-lock-reentrant");
  writeFileSync(join(dir, "lib.rs"), `use std::sync::{LazyLock, OnceLock};

static LAZY: LazyLock<u32> = LazyLock::new(|| *LAZY + 1);
static ONCE: OnceLock<u32> = OnceLock::new();

pub fn lazy() -> u32 {
    *LAZY
}

pub fn once() -> u32 {
    *ONCE.get_or_init(|| {
        let _ = ONCE.set(1);
        2
    })
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { (await import(${JSON.stringify(out)})).${name}(); } catch (e) { console.log(e.message); }`]);
  expect(call("lazy")).toBe("rust-js does not support a `LazyLock` whose init uses it, which deadlocks in Rust\n");
  expect(call("once")).toBe("rust-js does not support a `OnceLock` whose init sets it, which deadlocks in Rust\n");
});

// Locking a lock its thread holds deadlocks in Rust, which JS can't do:
// rust-js says so (ADR 0328).
test("a lock locked again while held is rust-js's error, not a deadlock", () => {
  const dir = fixture("runtime-lock-held");
  writeFileSync(join(dir, "lib.rs"), `use std::sync::{Mutex, RwLock};

pub fn mutex() -> u32 {
    let m = Mutex::new(1);
    let held = m.lock().unwrap();
    let again = *m.lock().unwrap();
    *held + again
}

pub fn rw() -> u32 {
    let rw = RwLock::new(1);
    let reading = rw.read().unwrap();
    let shared = *rw.read().unwrap();
    *rw.write().unwrap() += 1;
    *reading + shared
}

pub fn rw_read() -> u32 {
    let rw = RwLock::new(1);
    let mut writing = rw.write().unwrap();
    *writing += *rw.read().unwrap();
    *writing
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { (await import(${JSON.stringify(out)})).${name}(); } catch (e) { console.log(e.message); }`]);
  const error = "rust-js does not support locking a lock its thread holds, which deadlocks in Rust\n";
  expect(call("mutex")).toBe(error);
  expect(call("rw")).toBe(error);
  expect(call("rw_read")).toBe(error);
});

// A path's extension with a separator in it panics, as std's
// `set_extension` and `with_extension` do (ADR 0405).
test("a path's extension with a separator panics, as std's does", () => {
  const dir = fixture("path-extension-separator");
  writeFileSync(join(dir, "lib.rs"), `use std::path::{Path, PathBuf};

pub fn set() -> bool {
    PathBuf::from("a.txt").set_extension("b/c")
}

pub fn copied() -> String {
    Path::new("a.txt").with_extension("b/c").display().to_string()
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { (await import(${JSON.stringify(out)})).${name}(); } catch (e) { console.log(e.message); }`]);
  const error = 'extension cannot contain path separators: "b/c"\n';
  expect(call("set")).toBe(error);
  expect(call("copied")).toBe(error);
});

// A union's field read while it holds another reads its bytes as another
// type's, which JS has none of: rust-js's error where it runs; a field of
// the same type reads the same bytes (ADR 0406).
test("a union's field it doesn't hold is rust-js's error where it's read", () => {
  const dir = fixture("union-punned");
  writeFileSync(join(dir, "lib.rs"), `union Number {
    int: u32,
    float: f32,
    same: u32,
}

pub fn punned() -> f32 {
    let n = Number { int: 1 };
    unsafe { n.float }
}

pub fn same() -> u32 {
    let n = Number { int: 7 };
    unsafe { n.same }
}

pub fn rewritten() -> f32 {
    let mut n = Number { float: 0.5 };
    n.int = 2;
    unsafe { n.float }
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}()); } catch (e) { console.log(e.message); }`]);
  expect(call("punned")).toBe("rust-js does not support reading a union's `float` while it holds `int`, which reads its bytes as another type's\n");
  expect(call("same")).toBe("7\n");
  expect(call("rewritten")).toBe("rust-js does not support reading a union's `float` while it holds `int`, which reads its bytes as another type's\n");
});

// A pointer's address, which JS hasn't, is rust-js's error where it's
// shown; a null one's is `0x0`, as every program shows it (ADR 0407).
test("a pointer's address is rust-js's error where it's shown", () => {
  const dir = fixture("pointer-shown");
  writeFileSync(join(dir, "lib.rs"), `pub fn debug() -> String {
    let x = 1;
    format!("{:?}", &raw const x)
}

pub fn pointer() -> String {
    let x = 1;
    format!("{:p}", &raw const x)
}

pub fn none() -> String {
    format!("{:?} {:p}", std::ptr::null::<u8>(), std::ptr::null::<u8>())
}

fn named() {}

// A function pointer is a pointer, its address shown as a raw one's is.
pub fn function_pointer() -> String {
    format!("{:?}", named as fn())
}

// What only a failing assertion would show: never.
pub fn compared() -> bool {
    assert_eq!(named as fn(), named as fn());
    assert_eq!(std::any::TypeId::of::<u8>(), std::any::TypeId::of::<u8>());
    true
}

// A TypeId shows its hash, which rust-js doesn't keep.
pub fn type_id() -> String {
    format!("{:?}", std::any::TypeId::of::<u8>())
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}()); } catch (e) { console.log(e.message); }`]);
  const error = "rust-js does not support showing a pointer's address, which JS hasn't\n";
  expect(call("debug")).toBe(error);
  expect(call("pointer")).toBe(error);
  expect(call("none")).toBe("0x0 0x0\n");
  expect(call("function_pointer")).toBe(error);
  expect(call("compared")).toBe("true\n");
  expect(call("type_id")).toBe("rust-js does not support showing a `TypeId`'s hash, which it doesn't keep\n");
});

// A `None` given to JS is `undefined` given, never left out: a JS function
// may count its arguments, as `Math.max` does (ADR 0330). Not a binding's
// optional parameter's, an `Option`, which is left out, `Math.max(1)` of
// `max(1.0, None)` (ADR 0443); one of any value, `B`, is given.
test("an undefined given to JS is an argument, never left out", () => {
  const dir = fixture("given-undefined");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "Math.max")] #[allow(unused_variables)] fn math_max<B>(a: f64, b: B) -> f64 { unreachable!() }

pub fn bound() -> f64 {
    math_max(1.0, None::<f64>)
}

pub fn given(f: &dyn Fn(f64, Option<f64>) -> f64) -> f64 {
    f(1.0, None)
}

fn own(a: f64, b: Option<f64>) -> f64 {
    a + b.unwrap_or(0.0)
}

pub fn called() -> f64 {
    own(1.0, None)
}

pub fn closed(n: f64) -> f64 {
    let add = move |a: f64, b: Option<f64>| {
        let total = a + b.unwrap_or(n);
        total * 2.0
    };
    add(1.0, None) + add(2.0, Some(1.0))
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const js = readFileSync(out, "utf8");
  expect(js).toContain("Math.max(1, undefined)");
  expect(js).toContain("f(1, undefined)");
  // The crate's own function takes it as missing: `own(1)`, as a person writes it.
  expect(js).toContain("own(1)");
  expect(js).toContain("add(1)");
  const call = (expression: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `const m = await import(${JSON.stringify(out)}); console.log(${expression});`]);
  expect(call("m.bound()")).toBe("NaN\n");
  expect(call("m.given(Math.max)")).toBe("NaN\n");
  expect(call("m.called()")).toBe("1\n");
  expect(call("m.closed(3)")).toBe("14\n");
});
