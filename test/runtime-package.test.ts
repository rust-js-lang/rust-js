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
// writes no `Iterator.from` around, are `$iterator`, `$intoIter` (ADR
// 0475), `$cycleOf` (ADR 0491), `$repeating` and
// `$repeatingN` (ADR 0513), `$rev` (ADR 0516), `$closing` (ADR 0518),
// `$mapEnds` (ADR 0455), `$filterEnds` (ADR 0526),
// `$enumerate` (ADR 0528), `$taking` and `$skipping` (ADR 0542),
// `$memchrIter` (ADR 0529), `$stdIterator`, a
// compiled std struct's (ADR 0561), and `$successors`, which drops what it
// holds where it's closed (ADR 0465), and each of the runtime's generators
// (ADR 0472): the compiler's list, held to the runtime.
test("the compiler's iterator helpers are $iterator, $given, $intoIter, $cycleOf, $repeating, $repeatingN, $rev, $closing, $mapEnds, $filterEnds, $enumerate, $taking, $skipping, $memchrIter, $stdIterator, $successors, $lazyScan, $lazyMapWhile, $lazyTakeWhile, $lazySkipWhile, $lazyChain, $lazyZip, $stepBy, $flattenEnds and the runtime's generators", () => {
  const printed = run([compiler, "--runtime-module"]);
  const generators = [...printed.matchAll(/^export function\* (\$[\w$]+)/gm)].map((m) => m[1]);
  const source = readFileSync(join(root, "src", "lower", "iterators.rs"), "utf8");
  const list = source.match(/pub\(crate\) const ITERATOR_HELPERS: \[&str; \d+\] = \[([^\]]*)\]/)![1];
  // A range's, named where ranges are.
  const range = readFileSync(join(root, "src", "lower", "std_types", "range.rs"), "utf8");
  const named = (constant: string) => range.match(new RegExp(`pub\\(crate\\) const ${constant}: &str = "(\\$\\w+)";`))![1];
  const listed = [
    ...list
      .replace("super::std_types::range::LAZY_RANGE_BACK", `"${named("LAZY_RANGE_BACK")}"`)
      .replace("super::std_types::range::LAZY_RANGE", `"${named("LAZY_RANGE")}"`)
      .matchAll(/"(\$[\w$]+)"/g),
  ].map(
    (m) => m[1],
  );
  expect(listed.toSorted()).toEqual(
    [
      "$iterator",
      "$given",
      "$intoIter",
      "$cycleOf",
      "$repeating",
      "$repeatingN",
      "$rev",
      "$closing",
      "$mapEnds",
      "$filterEnds",
      "$enumerate",
      "$taking",
      "$skipping",
      "$memchrIter",
      "$stdIterator",
      "$successors",
      "$lazyScan",
      "$lazyMapWhile",
      "$lazyTakeWhile",
      "$lazySkipWhile",
      "$lazyChain",
      "$lazyZip",
      "$stepBy",
      "$flattenEnds",
      ...generators,
    ].toSorted(),
  );
});

// Each helper is a file of its own, `src/runtime/<name>.js`, that the
// compiler reads in: JS that parses as JS, and none left behind unread.
// The package loads in a host with JavaScript's own built-ins and none of
// the web's: a module that imports `$swap` runs where `TextDecoder`
// isn't, as React's scheduler's tests run under Jest's jsdom (ADR 0574).
// What a helper needs of the host it makes when it's first asked.
test("@rust-js/runtime loads in a host with none of the web's APIs", () => {
  const web = ["TextDecoder", "TextEncoder", "URL", "URLSearchParams", "fetch", "crypto", "performance", "navigator",
    "structuredClone", "queueMicrotask", "setTimeout", "setInterval", "atob", "btoa", "Intl", "WebAssembly", "process"];
  const removed = web.map((name) => `delete globalThis.${name};`).join(" ");
  const script = `${removed} const runtime = await import(${JSON.stringify(join(root, "runtime/index.js"))}); console.log(Object.keys(runtime).length);`;
  const count = Number(run([node ?? "node", "--input-type=module", "--eval", script]).trim());
  expect(count).toBeGreaterThan(300);
});

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

// A map keyed by a struct is a `$KeyMap` (ADR 0121), a B-tree of a key with
// an `Ord` of its own a `$OrdMap` (ADR 0404), each of which JS code given one
// reads as any `Map`: `forEach` gives each value, then its key; `keys()`
// gives the keys, not the strings they're kept under; `get` and `has` find
// a key by its value. No Rust program calls its `forEach`; JS's does.
test("a map keyed by a struct, hashed or ordered, reads as a Map to the JS given it", () => {
  const dir = fixture("runtime-key-map");
  writeFileSync(join(dir, "lib.rs"), `use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub fn places() -> HashMap<Point, u32> {
    let mut m = HashMap::new();
    m.insert(Point { x: 1, y: 2 }, 3);
    m.insert(Point { x: 4, y: 5 }, 6);
    m
}

// Ordered by an \`Ord\` of its own, so a B-tree of it is a \`$OrdMap\`.
impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.x, self.y).cmp(&(other.x, other.y))
    }
}

pub fn ordered() -> BTreeMap<Point, u32> {
    let mut m = BTreeMap::new();
    m.insert(Point { x: 4, y: 5 }, 6);
    m.insert(Point { x: 1, y: 2 }, 3);
    m.remove(&Point { x: 2, y: 0 });
    m
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const read = `const m = (await import(${JSON.stringify(out)})).places();
const seen = [];
m.forEach((value, key) => seen.push([key.x, key.y, value]));
console.log(JSON.stringify(seen), JSON.stringify([...m.keys()]), JSON.stringify([...m.values()]), JSON.stringify([...m]), m.get({ x: 4, y: 5 }), m.has({ x: 1, y: 2 }), m.has({ x: 2, y: 1 }), m.size);`;
  expect(run([node ?? "node", "--input-type=module", "--eval", read])).toBe(
    '[[1,2,3],[4,5,6]] [{"x":1,"y":2},{"x":4,"y":5}] [3,6] [[{"x":1,"y":2},3],[{"x":4,"y":5},6]] 6 true false 2\n',
  );
  // A B-tree's `$OrdMap` is kept in its keys' order, as its JS sees it.
  expect(run([node ?? "node", "--input-type=module", "--eval", read.replace(".places()", ".ordered()")])).toBe(
    '[[1,2,3],[4,5,6]] [{"x":1,"y":2},{"x":4,"y":5}] [3,6] [[{"x":1,"y":2},3],[{"x":4,"y":5},6]] 6 true false 2\n',
  );
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
  expect(call("rwRead")).toBe(error);
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

// Room past `isize::MAX` bytes, rust-js's `usize`'s, `2 ** 31 - 1`, is a
// `capacity overflow` panic, and room up to it isn't (ADR 0774): native
// Rust's `usize` here is 64-bit, so its corpus can't tell where it ends.
test("room past rust-js's isize::MAX bytes panics, and room up to it doesn't", () => {
  const dir = fixture("room-for");
  writeFileSync(join(dir, "lib.rs"), `pub fn units(n: usize) -> usize {
    Vec::<u16>::with_capacity(n).len()
}

pub fn bytes(n: usize) -> usize {
    String::with_capacity(n).len()
}

pub fn text_room(n: usize) -> usize {
    let mut s = String::from("é");
    s.reserve(n);
    s.len()
}

pub fn none_room(n: usize) -> usize {
    let mut v = vec![(); 3];
    v.reserve(n);
    v.len()
}

// A mebibyte and one in Rust, \`undefined\` in JS.
pub fn nones(n: usize) -> usize {
    vec![None::<[u8; 1 << 20]>; n].len()
}

pub fn repeated_nones(n: usize) -> usize {
    [None::<[u8; 1 << 20]>].repeat(n).len()
}

pub struct Big(Option<[u8; 1 << 20]>);

impl Clone for Big {
    fn clone(&self) -> Self {
        Big(self.0)
    }
}

impl Drop for Big {
    fn drop(&mut self) {}
}

pub fn bigs(n: usize) -> usize {
    vec![Big(None); n].len()
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string, n: number) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}(${n})); } catch (e) { console.log(e.message); }`]).trim();
  expect(call("units", 2 ** 30 - 1)).toBe("0");
  expect(call("units", 2 ** 30)).toBe("capacity overflow");
  expect(call("bytes", 2 ** 31 - 1)).toBe("0");
  expect(call("bytes", 2 ** 31)).toBe("capacity overflow");
  // "é"'s two bytes, one UTF-16 unit.
  expect(call("textRoom", 2 ** 31 - 3)).toBe("2");
  expect(call("textRoom", 2 ** 31 - 2)).toBe("capacity overflow");
  // Of no size, by their count alone, `usize::MAX`.
  expect(call("noneRoom", 2 ** 32 - 4)).toBe("3");
  expect(call("noneRoom", 2 ** 32 - 3)).toBe("capacity overflow");
  // 2,047 of 1,048,577 bytes fit, 2,048 don't.
  for (const name of ["nones", "repeatedNones", "bigs"]) {
    expect(call(name, 2047)).toBe("2047");
    expect(call(name, 2048)).toBe("capacity overflow");
  }
});

// A union's field read while it holds another reads the bytes it holds as
// the field's type: plain data's, as wasm32 lays them out (ADR 0562); a
// field of the same type reads the same bytes (ADR 0406); a field whose
// bytes JS has none of, rust-js's error where it runs.
test("a union's field it doesn't hold reads the bytes it holds, or is rust-js's error where it's read", () => {
  const dir = fixture("union-punned");
  writeFileSync(join(dir, "lib.rs"), `union Number {
    int: u32,
    float: f32,
    same: u32,
    name: &'static str,
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

pub fn named() -> &'static str {
    let n = Number { int: 1 };
    unsafe { n.name }
}

#[derive(Clone, Copy)]
pub struct Small(pub u8);

pub union Mixed {
    whole: u32,
    small: Small,
}

pub fn part_written() -> u32 {
    let mut m = Mixed { whole: 7 };
    unsafe { m.small.0 = 1 };
    unsafe { m.whole }
}

// Four bytes, one of them padding, which a value of it doesn't hold.
#[derive(Clone, Copy)]
pub struct Padded(pub u8, pub u16);

pub union Spaced {
    whole: u32,
    padded: Padded,
}

pub fn padding_written() -> u32 {
    let mut s = Spaced { whole: 7 };
    unsafe { s.padded.0 = 1 };
    unsafe { s.whole }
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}()); } catch (e) { console.log(e.message); }`]);
  expect(call("punned")).toBe("1.401298464324817e-45\n");
  expect(call("same")).toBe("7\n");
  expect(call("rewritten")).toBe("2.802596928649634e-45\n");
  expect(call("named")).toBe("rust-js does not support reading a union's `name` while it holds `int`, which reads its bytes as another type's\n");
  // A struct field smaller than the union, which can't be held in its place
  // without losing bytes: written into, rust-js's error, not a lost write.
  expect(call("partWritten")).toBe("rust-js does not support reading a union's `small` while it holds `whole`, which reads its bytes as another type's\n");
  expect(call("paddingWritten")).toBe("rust-js does not support reading a union's `padded` while it holds `whole`, which reads its bytes as another type's\n");
});

// A pointer's address is a number of rust-js's own, shown as Rust shows one,
// `0x` and hex; the same place shows the same, and a null one's is `0x0`,
// as every program shows it (ADR 0562).
test("a pointer's address is shown", () => {
  const dir = fixture("pointer-shown");
  writeFileSync(join(dir, "lib.rs"), `pub fn debug() -> String {
    let x = 1;
    format!("{:?}", &raw const x)
}

pub fn pointer() -> String {
    let x = 1;
    let p = &raw const x;
    let again = format!("{:p}", p);
    format!("{:p} {}", p, again == format!("{:p}", p))
}

pub fn none() -> String {
    format!("{:?} {:p}", std::ptr::null::<u8>(), std::ptr::null::<u8>())
}

// Reading through a pointer to no place is rust-js's error where it runs.
pub fn no_place() -> u8 {
    let p = 7 as *const u8;
    unsafe { *p }
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
  expect(call("debug")).toMatch(/^0x[0-9a-f]+\n$/);
  expect(call("pointer")).toMatch(/^0x[0-9a-f]+ true\n$/);
  expect(call("noPlace")).toBe("rust-js does not support reading through a pointer to no place, at 0x7, which reads memory by where it is\n");
  expect(call("functionPointer")).toMatch(/^0x[0-9a-f]+\n$/);
  expect(call("none")).toBe("0x0 0x0\n");
  expect(call("compared")).toBe("true\n");
  expect(call("typeId")).toBe("rust-js does not support showing a `TypeId`'s hash, which it doesn't keep\n");
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
