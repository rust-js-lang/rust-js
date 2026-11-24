// A binary crate's `main` runs as std's runtime runs it (#2292): its
// status, what std reports of a `Termination`, and std's panic hook's text
// where a panic is raised, before what it drops, once noted, read through
// one stream as a terminal shows it, `2>&1`; each against native Rust, on
// Node and Bun. A panic's location is the compile's, `file:line:col` of the
// file as rustc was given it, as `Location` is.
import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compileJs, compileJsBinary, compileJsLibrary } from "./programs";
import { buildSerde } from "./serde-crate";
import { compiler, fixture, installStd, nativeBinary, programFlags, root, run, shippedStd } from "./support";

const flags = programFlags("2024");

type Ran = { status: number; out: string };

// What one stream shows of a run, `2>&1`, its process id said as `<pid>`,
// as the hook names the thread's process; and of native's, std's files and
// a registry's as rust-js names them (`shippedStd`), its JS's as it is: of
// both, a crate's own panic named its machine's `$CARGO_HOME` alike (#2435).
function ran(cmd: string[], cwd: string, native = false): Ran {
  const p = runSync(["sh", "-c", `${cmd.map((part) => `'${part}'`).join(" ")} 2>&1`], cwd, 60_000);
  const out = p.stdout.replace(/thread 'main' \(\d+\)/g, "thread 'main' (<pid>)");
  return { status: p.code ?? -1, out: native ? shippedStd(out) : out };
}

// `source` as a binary, native and rust-js's, in `dir`, the same file
// given to both, its directory left out of the name both give it, and each
// run: native's, then Node's and Bun's.
function both(name: string, source: string, dir = fixture(name), crates = { native: [] as string[], js: [] as string[] }): { native: Ran; node: Ran; bun: Ran } {
  mkdirSync(dir, { recursive: true });
  const named = [...flags, `--remap-path-prefix=${dir}/=`];
  const built = nativeBinary(source, dir, [...named, ...crates.native]);
  if ("error" in built) throw new Error(built.error);
  const js = join(dir, "main.js");
  run([compiler, join(dir, "native.rs"), "-o", js, "--", ...named, ...crates.js, "--crate-type", "bin", "-Awarnings"]);
  installStd(dir);
  return { native: ran([built.binary], dir, true), node: ran(["node", js], dir), bun: ran(["bun", js], dir) };
}

function same(name: string, source: string, dir?: string, crates?: { native: string[]; js: string[] }) {
  const { native, node, bun } = both(name, source, dir, crates);
  expect(node).toEqual(native);
  expect(bun).toEqual(native);
  return native;
}

test("a binary's status is its main's: nothing, `Ok`, `Err` reported, an `ExitCode`, `exit`", () => {
  expect(same("unit", 'fn main() {\n    println!("hi");\n}\n').status).toBe(0);
  expect(same("ok", 'fn main() -> Result<(), String> {\n    println!("a");\n    Ok(())\n}\n').status).toBe(0);
  const err = same("err", 'fn main() -> Result<(), String> {\n    println!("a");\n    Err("bad".to_string())\n}\n');
  expect(err).toEqual({ status: 1, out: 'a\nError: "bad"\n' });
  expect(same("code", "fn main() -> std::process::ExitCode {\n    std::process::ExitCode::from(42)\n}\n").status).toBe(42);
  const exit = same(
    "exit",
    'struct D;\n\nimpl Drop for D {\n    fn drop(&mut self) {\n        println!("dropped");\n    }\n}\n\nfn main() {\n    let _d = D;\n    println!("before");\n    std::process::exit(3);\n}\n',
  );
  expect(exit).toEqual({ status: 3, out: "before\n" });
}, 300_000);

test("a panic's hook text is written where it's raised, before what it drops, its note once", () => {
  const source = `struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn boom(n: u32) {
    let _d = D("inner");
    if n > 0 {
        panic!("boom {n}");
    }
}

fn main() {
    println!("start");
    let r = std::panic::catch_unwind(|| boom(1));
    println!("caught {}", r.is_err());
    let r = std::panic::catch_unwind(|| boom(2));
    println!("caught {}", r.is_err());
    let _outer = D("outer");
    boom(3);
}
`;
  const native = same("hooked", source);
  expect(native.status).toBe(101);
  expect(native.out.match(/RUST_BACKTRACE/g)?.length).toBe(1);
  expect(native.out.indexOf("boom 3")).toBeLessThan(native.out.indexOf("drop outer"));
}, 300_000);

test("a resumed unwind no panic raised ends a binary with no hook's text", () => {
  const native = same("resumed", 'fn main() {\n    println!("a");\n    std::panic::resume_unwind(Box::new(5));\n}\n');
  expect(native).toEqual({ status: 101, out: "a\n" });
}, 300_000);

// A binary's `RefCell`s no borrow of which can meet another are quiet, a
// guard their handle, where a library's held across a `dyn` call is
// counted: the corpus case, as a binary, its guard lent as a `&mut dyn`
// after a `get_mut` lend, whose pair writes the field, never a `const`
// read of it (#2445).
test("a quiet `RefCell` field's guard lent as a `&mut dyn` after a `get_mut` lend writes the field", () => {
  const source = readFileSync(join(root, "test/corpus/refcell_guard_lent_as_a_dyn_after_get_mut.rs"), "utf8");
  expect(same("guard-lent-as-a-dyn", source)).toEqual({ status: 0, out: "Some(2)\n4\nSome(5) 0\n" });
}, 300_000);

// A binary's quiet `RefCell` is still counted where its borrows are asked
// of: `try_borrow_mut()` while a `borrow()` is held fails, as Rust's does,
// in a `let`-`else` guard and in a `Drop` or a `Clone` that asks it during
// a held borrow (the Mac mini's #2447 runs of the corpus as binaries,
// #2450, #2451). A library's cells are judged by their type, so another
// borrow of one of the type counted answers for them there.
for (const name of ["let_else_guard_released", "refcell_held_while_asked", "refcell_try_borrow_in_a_closure_while_held"]) {
  test(`a binary's \`RefCell\` held while it's asked of answers as Rust's: ${name}.rs`, () => {
    same(name.replaceAll("_", "-"), readFileSync(join(root, `test/corpus/${name}.rs`), "utf8"));
  }, 300_000);
}

test("what stdout holds of a line is written as a panic ends a binary", () => {
  same("partial", 'fn main() {\n    print!("partial");\n    panic!("then");\n}\n');
}, 300_000);

test("each panic of the program's own code names where it was raised", () => {
  const kinds = [
    "let v = vec![1];\n    let i = std::hint::black_box(3);\n    println!(\"{}\", v[i]);",
    "let o: Option<u8> = std::hint::black_box(None);\n    o.unwrap();",
    "let r: Result<u8, String> = Err(\"no\".into());\n    r.expect(\"wanted\");",
    "let c = std::cell::RefCell::new(1);\n    let _a = c.borrow_mut();\n    let _b = c.borrow();",
    "let z = std::hint::black_box(0);\n    println!(\"{}\", 1 / z);",
    "let mut v = vec![1, 2];\n    v.insert(5, 3);",
    "let s = String::from(\"héllo\");\n    println!(\"{}\", &s[0..2]);",
    "assert_eq!(std::hint::black_box(1), 2, \"why {}\", 3);",
    "println!(\"{}\", 5u32.ilog(std::hint::black_box(1)));",
  ];
  kinds.forEach((body, i) => {
    expect(same(`kind${i}`, `fn main() {\n    ${body}\n}\n`).status).toBe(101);
  });
}, 300_000);

test("a part of a slice used apart from where it's indexed names where it's indexed", () => {
  // A view made where it's indexed, `&mut v[1..]`, and its call made where
  // it's used, a call's argument lines below, or a form of its own over the
  // range, `$reverseRange` (#2292).
  const views = [
    "fn bump(xs: &mut [u32]) {\n    xs[0] += 1;\n}\n\nfn main() {\n    let mut v: Vec<u32> = std::hint::black_box(vec![]);\n    bump(\n        &mut v[1..], // the tail\n    );\n}\n",
    "fn main() {\n    let mut v = vec![1, 2];\n    let b = std::hint::black_box(5);\n    v[1..b].reverse();\n}\n",
  ];
  views.forEach((source, i) => {
    expect(same(`view${i}`, source).status).toBe(101);
  });
}, 300_000);

test("a panic a `#[track_caller]` function raises itself names where it was called", () => {
  // rustc places each panic a tracking function raises, `panic!`'s, std's
  // that tracks its caller, `unwrap`'s, a `Vec`'s index, and its own
  // `Assert`s, a division's and a slice's bound, where it was called, so it
  // takes its caller's location (#2350).
  const tracked = [
    "#[track_caller]\nfn check(n: u32) -> u32 {\n    if n > 3 {\n        panic!(\"too big: {n}\");\n    }\n    n\n}\n\nfn main() {\n    println!(\"{}\", check(1));\n    println!(\"{}\", check(std::hint::black_box(5)));\n}\n",
    "#[track_caller]\nfn first(v: &[u32]) -> u32 {\n    *v.first().unwrap()\n}\n\nfn main() {\n    println!(\"{}\", first(&[4]));\n    println!(\"{}\", first(std::hint::black_box(&[])));\n}\n",
    "#[track_caller]\nfn share(total: u32, n: u32) -> u32 {\n    total / n\n}\n\nfn main() {\n    println!(\"{}\", share(6, 2));\n    println!(\"{}\", share(6, std::hint::black_box(0)));\n}\n",
    "#[track_caller]\nfn at(v: &[u32], i: usize) -> u32 {\n    v[i]\n}\n\nfn main() {\n    println!(\"{}\", at(&[1, 2], 1));\n    println!(\"{}\", at(&[1, 2], std::hint::black_box(5)));\n}\n",
    "#[track_caller]\nfn at(v: &Vec<u32>, i: usize) -> u32 {\n    v[i]\n}\n\nfn main() {\n    let v = vec![1, 2];\n    println!(\"{}\", at(&v, 1));\n    println!(\"{}\", at(&v, std::hint::black_box(5)));\n}\n",
    // Passed on: one that calls another that raises one.
    "#[track_caller]\nfn check(n: u32) -> u32 {\n    if n > 3 {\n        panic!(\"too big\");\n    }\n    n\n}\n\n#[track_caller]\nfn twice(n: u32) -> u32 {\n    check(n) + check(n * 2)\n}\n\nfn main() {\n    println!(\"{}\", twice(1));\n    println!(\"{}\", twice(std::hint::black_box(3)));\n}\n",
    // The crate's `Index`, whose trait tracks its caller, of a `Vec`; and
    // through generic code's dictionary.
    "struct Grid(Vec<u32>);\n\nimpl std::ops::Index<usize> for Grid {\n    type Output = u32;\n    fn index(&self, i: usize) -> &u32 {\n        &self.0[i]\n    }\n}\n\nfn main() {\n    let grid = Grid(vec![4, 5]);\n    println!(\"{}\", grid[1]);\n    println!(\"{}\", grid[std::hint::black_box(7)]);\n}\n",
    "struct Grid(Vec<u32>);\n\nimpl std::ops::Index<usize> for Grid {\n    type Output = u32;\n    fn index(&self, i: usize) -> &u32 {\n        &self.0[i]\n    }\n}\n\nfn get<G: std::ops::Index<usize, Output = u32>>(g: &G, i: usize) -> u32 {\n    g[i]\n}\n\nfn main() {\n    let grid = Grid(vec![4, 5]);\n    println!(\"{}\", get(&grid, 0));\n    println!(\"{}\", get(&grid, std::hint::black_box(9)));\n}\n",
  ];
  tracked.forEach((source, i) => {
    expect(same(`tracked${i}`, source).status).toBe(101);
  });
}, 300_000);

test("a panic std's impl raises at its caller, through a dictionary, names generic code's line", () => {
  // std's impls track their caller where the trait doesn't, integers'
  // `Div::div` and `Ord::clamp`: a dictionary's entry is given generic
  // code's location, as rustc's monomorphized call of the impl is (#2324).
  const generic = [
    "fn halved<T: std::ops::Div<Output = T>>(a: T, b: T) -> T {\n    a / b\n}\n\nfn main() {\n    println!(\"{}\", halved(6u32, 2));\n    println!(\"{}\", halved(1u32, std::hint::black_box(0)));\n}\n",
    "fn rest<T: std::ops::Rem<Output = T>>(a: T, b: T) -> T {\n    a % b\n}\n\nfn main() {\n    println!(\"{}\", rest(i32::MIN, std::hint::black_box(-1)));\n}\n",
    "fn within<T: Ord>(x: T, low: T, high: T) -> T {\n    x.clamp(low, high)\n}\n\nfn main() {\n    println!(\"{}\", within(5, 1, 9));\n    println!(\"{}\", within(5, std::hint::black_box(9), 1));\n}\n",
    // A `char`'s tracks its caller too; std's default, a `String`'s, through
    // a dictionary or called itself, names its own line.
    "fn within<T: Ord>(x: T, low: T, high: T) -> T {\n    x.clamp(low, high)\n}\n\nfn main() {\n    println!(\"{}\", within('m', 'a', 'z'));\n    println!(\"{}\", within('m', std::hint::black_box('z'), 'a'));\n}\n",
    "fn within<T: Ord>(x: T, low: T, high: T) -> T {\n    x.clamp(low, high)\n}\n\nfn main() {\n    let (a, z) = (String::from(\"a\"), String::from(\"z\"));\n    println!(\"{}\", within(String::from(\"m\"), a.clone(), z.clone()));\n    println!(\"{}\", within(String::from(\"m\"), z, a));\n}\n",
    "fn main() {\n    let (a, z) = (String::from(\"a\"), String::from(\"z\"));\n    println!(\"{}\", String::from(\"m\").clamp(std::hint::black_box(z), a));\n}\n",
    // A generic `SliceIndex`'s, an item's and a range's.
    "fn get<I: std::slice::SliceIndex<[u32]>>(v: &[u32], i: I) -> &I::Output {\n    &v[i]\n}\n\nfn main() {\n    let v = [1, 2, 3];\n    println!(\"{} {:?}\", get(&v, 1), get(&v, 1..2));\n    println!(\"{}\", get(&v, std::hint::black_box(7)));\n}\n",
    "fn get<I: std::slice::SliceIndex<[u32]>>(v: &[u32], i: I) -> &I::Output {\n    &v[i]\n}\n\nfn main() {\n    let v = [1, 2, 3];\n    println!(\"{:?}\", get(&v, 2..std::hint::black_box(9)));\n}\n",
  ];
  generic.forEach((source, i) => {
    expect(same(`generic${i}`, source).status).toBe(101);
  });
}, 300_000);

test("a `#[track_caller]` function made a value whose uses say how it's called names its shim's line in a binary, as native", () => {
  // Each a corpus case's shape, compiled as a binary, which refuses what's
  // untold (#2335): a promoted `&dyn Fn`, a `Box<dyn FnOnce>`, a value read
  // out of its box, a reference given as an `F: FnMut` (#2355), and a local
  // also called itself, which names its own line.
  const told = ["std_function_value_as_a_dyn_panic", "std_function_value_boxed_once_panic", "std_function_value_in_a_box_panic", "std_function_value_borrowed_panic", "std_function_value_called_itself_panic"];
  told.forEach((name) => {
    const source = readFileSync(join(root, "test/corpus", `${name}.rs`), "utf8");
    expect(same(`told-${name}`, source).status).toBe(101);
  });
}, 300_000);

test("a `#[track_caller]` function made a value whose calls the body doesn't say is refused in a binary, saying what to write; a library compiles it", () => {
  // Its shim's line is the location its panic names (#2335): where the
  // body doesn't say which shim, a binary, which writes the location,
  // refuses it, Tu's decision; a library keeps compiling.
  const untold = [
    // A local called two ways, `call_mut`'s and `call_once`'s.
    "fn main() {\n    let f = Option::unwrap;\n    let a: Vec<i32> = vec![Some(1)].into_iter().map(f).collect();\n    println!(\"{:?} {:?}\", a, Some(Some(2)).map(f));\n}\n",
    // A `Box<dyn Fn>`, called as its own calls say.
    "fn main() {\n    let b: Box<dyn Fn(Option<i32>) -> i32> = Box::new(Option::unwrap);\n    println!(\"{}\", b(Some(1)));\n}\n",
    // Returned, and captured.
    "fn get() -> impl Fn(Option<i32>) -> i32 {\n    Option::unwrap\n}\n\nfn main() {\n    println!(\"{}\", get()(Some(1)));\n}\n",
    "fn main() {\n    let f = Option::unwrap;\n    let c = move |x: Option<i32>| f(x);\n    println!(\"{}\", c(Some(1)));\n}\n",
    // Stored in a struct given whole; the crate's own, two parameters.
    "struct Held<F>(F);\n\nfn run<F: Fn(Option<i32>) -> i32>(h: Held<F>) -> i32 {\n    (h.0)(Some(1))\n}\n\nfn main() {\n    println!(\"{}\", run(Held(Option::unwrap)));\n}\n",
    "#[track_caller]\nfn pick(v: &[i32], i: usize) -> i32 {\n    v[i]\n}\n\nfn by_fn<F: Fn(&[i32], usize) -> i32>(f: F) -> i32 {\n    f(&[1], 0)\n}\n\nfn by_once<F: FnOnce(&[i32], usize) -> i32>(f: F) -> i32 {\n    f(&[1], 0)\n}\n\nfn main() {\n    let f = pick;\n    println!(\"{} {}\", by_fn(f), by_once(f));\n}\n",
  ];
  untold.forEach((source, i) => {
    const dir = fixture(`untold${i}`);
    mkdirSync(dir, { recursive: true });
    const file = join(dir, "main.rs");
    writeFileSync(file, source);
    const bin = runSync([compiler, file, "-o", join(dir, "main.js"), "--", ...flags, "--crate-type", "bin", "-Awarnings"], dir, 120_000);
    expect(bin.code).not.toBe(0);
    expect(bin.stderr).toContain("a `#[track_caller]` function, called where this function doesn't say how (#2335) yet");
    expect(bin.stderr).toContain(i === untold.length - 1 ? "or write a closure, `|a, b| pick(a, b)`" : "or write a closure, `|x| Option::unwrap(x)`");
    const lib = runSync([compiler, file, "-o", join(dir, "lib.js"), "--", ...flags, "-Awarnings"], dir, 120_000);
    expect(lib.stderr).toBe("");
    expect(lib.code).toBe(0);
  });
}, 300_000);

test("a binary that links a library whose body makes an untold `#[track_caller]` value is refused, naming its site; a library that links it compiles", () => {
  // The library keeps its caller's line, which a binary would write: its
  // manifest lists the site, and a binary that links it is refused (#2335).
  const dir = fixture("untold-linked");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "lib.rs"), "pub fn pair(v: Vec<Option<i32>>) -> (Vec<i32>, Option<i32>) {\n    let f = Option::unwrap;\n    (v.into_iter().map(f).collect(), Some(Some(1)).map(f))\n}\n");
  const lib = compileJsLibrary(join(dir, "lib.rs"), dir, "2024", "tracked");
  if (!("manifest" in lib)) throw new Error(lib.error);
  const untold = JSON.parse(readFileSync(lib.manifest, "utf8")).library.untold_locations;
  expect(untold).toEqual([{ value: "Option::unwrap", at: expect.stringMatching(/lib\.rs:2:13$/), closure: "|x| Option::unwrap(x)" }]);
  writeFileSync(join(dir, "main.rs"), "fn main() {\n    println!(\"{:?}\", tracked::pair(vec![Some(1)]));\n}\n");
  const linking = [["--dependency", lib.manifest], ["--extern", `tracked=${lib.rmeta}`, "-L", dir]];
  const bin = compileJsBinary(join(dir, "main.rs"), dir, "2024", ...linking);
  expect("js" in bin).toBe(false);
  expect("js" in bin ? "" : bin.error).toMatch(/a library whose function makes a value of `Option::unwrap`, a `#\[track_caller\]` function, called where it doesn't say how, `tracked`'s at [^ ]*lib\.rs:2:13, in a binary/);
  expect("js" in bin ? "" : bin.error).toContain("or write a closure, `|x| Option::unwrap(x)`");
  mkdirSync(join(dir, "user"), { recursive: true });
  const user = compileJs(join(dir, "main.rs"), join(dir, "user"), "2024", false, {}, ...linking);
  expect("js" in user ? "" : user.error).toBe("");
}, 300_000);

test("a `Repeat`'s `count` and `last` name where they're called, through generic code and nested in another count too", () => {
  // Its impl tracks its caller: the `count` or `last` that may reach it is
  // given where it's called, held while it runs (#2324).
  const total = "fn total<I: Iterator>(it: I) -> usize {\n    it.count()\n}\n\n";
  const repeats = [
    "fn main() {\n    println!(\"{:?}\", std::iter::repeat(3).take(2).last());\n    println!(\"{:?}\", std::iter::repeat(3).last());\n}\n",
    "fn main() {\n    let n = std::iter::repeat(3).count();\n    println!(\"{n}\");\n}\n",
    `${total}fn main() {\n    println!("{}", total(vec![1, 2].into_iter()));\n    println!("{}", total(std::iter::repeat(3)));\n}\n`,
    `${total}fn main() {\n    let n = total([1, 2].iter().map(|_| total(std::iter::repeat(0).take(2))));\n    println!("{n}");\n    let m = total([1].iter().map(|_| std::iter::repeat(0).count()));\n    println!("{m}");\n}\n`,
    `${total}fn main() {\n    let caught = std::panic::catch_unwind(|| total(std::iter::repeat(0)));\n    println!("{}", caught.is_err());\n    println!("{}", std::iter::repeat(1).count());\n}\n`,
  ];
  repeats.forEach((source, i) => {
    expect(same(`repeat${i}`, source).status).toBe(101);
  });
}, 300_000);

test("a registry crate's panic names its own line, of the version Cargo built, under `/cargo`'s registry", () => {
  // serde_json's `IndexMut` of a `Value`, and hashbrown's own map's
  // `get_disjoint_mut`, each raised where its crate raises it, its build's
  // noted site (#2325); indexmap's, which tracks its caller, the caller's.
  const crates = { native: buildSerde("rlib"), js: buildSerde() };
  const raised = [
    "let mut v = serde_json::json!(3);\n    v[\"a\"] = serde_json::json!(1);",
    "let mut v = serde_json::json!({\"a\": 1});\n    v[0] = serde_json::json!(1);",
    "let mut v = serde_json::json!([1, 2]);\n    v[std::hint::black_box(5)] = serde_json::json!(1);",
    "let mut m: hashbrown::HashMap<u8, u8> = [(1, 1)].into_iter().collect();\n    let _ = m.get_disjoint_mut([&1, &std::hint::black_box(1)]);",
    "let mut m: indexmap::IndexMap<u8, u8> = [(1, 1)].into_iter().collect();\n    let _ = m.get_disjoint_mut([&1, &std::hint::black_box(1)]);",
  ];
  raised.forEach((body, i) => {
    expect(same(`registry${i}`, `fn main() {\n    ${body}\n}\n`, undefined, crates).status).toBe(101);
  });
}, 300_000);

test("each panic std raises in its own code names std's line, as its hook does", () => {
  const kinds = [
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    d.swap(0, std::hint::black_box(5));",
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    d.swap(std::hint::black_box(7), 1);",
    "let mut v = vec![1];\n    v.swap_remove(std::hint::black_box(3));",
    "let s = String::from(\"héllo\");\n    let _ = s.split_at(std::hint::black_box(2));",
    "let _ = std::hint::black_box(1.0f64).clamp(2.0, 1.0);",
    "let _ = std::hint::black_box(1.0f32).clamp(f32::NAN, 1.0);",
    "let a = std::cell::RefCell::new(1);\n    let b = std::cell::RefCell::new(1);\n    let _m = a.borrow_mut();\n    println!(\"{}\", a == b);",
    "let a = std::cell::RefCell::new(1);\n    let b = std::cell::RefCell::new(1);\n    let _m = b.borrow_mut();\n    println!(\"{}\", a == b);",
    "let mut v = [1, 2];\n    v.swap(0, std::hint::black_box(4));",
    "let mut d: std::collections::VecDeque<u8> = [1].into();\n    d.insert(std::hint::black_box(5), 1);",
    "let mut a = [1, 2];\n    a.rotate_left(std::hint::black_box(5));",
    "let mut a = [1, 2];\n    a.rotate_right(std::hint::black_box(5));",
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    d.rotate_left(std::hint::black_box(5));",
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    d.rotate_right(std::hint::black_box(5));",
    "let v: Vec<u64> = Vec::with_capacity(std::hint::black_box(usize::MAX));\n    println!(\"{}\", v.len());",
    "let mut a = [3, 1, 2];\n    a.select_nth_unstable(std::hint::black_box(5));",
    "println!(\"{:?}\", char::from_digit(1, std::hint::black_box(40)));",
    "let mut b = [0u8; 1];\n    let _ = 'é'.encode_utf8(&mut b);",
    "let mut b = [0u16; 1];\n    let _ = '😀'.encode_utf16(&mut b);",
    "println!(\"{:?}\", (0..5).step_by(std::hint::black_box(0)).count());",
    "println!(\"{:?}\", std::time::Duration::new(u64::MAX, std::hint::black_box(1_000_000_000)));",
    "println!(\"{:?}\", std::time::Duration::MAX + std::hint::black_box(std::time::Duration::from_secs(1)));",
    "println!(\"{:?}\", std::time::Duration::ZERO - std::hint::black_box(std::time::Duration::from_secs(1)));",
    "let c: std::cell::OnceCell<u8> = std::cell::OnceCell::new();\n    let _ = c.get_or_init(|| {\n        let _ = c.get_or_init(|| 1);\n        2\n    });",
    "let l: std::cell::LazyCell<u8> = std::cell::LazyCell::new(|| panic!(\"init\"));\n    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| *l));\n    println!(\"{}\", *l);",
    "let mut m = std::collections::BTreeMap::new();\n    m.insert(1u8, 1u8);\n    println!(\"{:?}\", m.range(std::hint::black_box(5)..std::hint::black_box(1)).count());",
    "let mut m = std::collections::BTreeMap::new();\n    m.insert(1u8, 1u8);\n    let b = std::hint::black_box(3);\n    println!(\"{:?}\", m.range((std::ops::Bound::Excluded(b), std::ops::Bound::Excluded(b))).count());",
    "let mut m = std::collections::BTreeSet::new();\n    m.insert(1u8);\n    println!(\"{:?}\", m.range(std::hint::black_box(5)..std::hint::black_box(1)).count());",
    "let mut m = std::collections::BTreeSet::new();\n    m.insert(1u8);\n    let b = std::hint::black_box(3);\n    println!(\"{:?}\", m.range((std::ops::Bound::Excluded(b), std::ops::Bound::Excluded(b))).count());",
    "println!(\"{:?}\", i64::from_str_radix(\"1\", std::hint::black_box(40)));",
    "println!(\"{:?}\", u8::from_str_radix(\"1\", std::hint::black_box(1)));",
    "println!(\"{:?}\", i32::from_str_radix(\"1\", std::hint::black_box(37)));",
    "println!(\"{}\", 5u8.next_multiple_of(std::hint::black_box(0)));",
    "println!(\"{}\", 5u32.next_multiple_of(std::hint::black_box(0)));",
    "println!(\"{}\", 5u64.next_multiple_of(std::hint::black_box(0)));",
    "let mut s = String::from(\"héllo\");\n    let _ = s.split_at_mut(std::hint::black_box(2));",
    // In std's vendored hashbrown, named under its own build's registry (#2325).
    "let mut m = std::collections::HashMap::from([(1u8, 1u8)]);\n    let _ = m.get_disjoint_mut([&1, &std::hint::black_box(1)]);",
    "let mut a = [std::mem::MaybeUninit::<String>::uninit(), std::mem::MaybeUninit::uninit()];\n    a.write_clone_of_slice(std::hint::black_box(&[String::new()]));",
    "let mut a = [std::mem::MaybeUninit::<u8>::uninit(); 2];\n    a.write_copy_of_slice(std::hint::black_box(&[1]));",
    "let mut r = (std::hint::black_box(char::MAX))..;\n    r.next();\n    println!(\"{:?}\", r.next());",
    "let mut p = std::path::PathBuf::from(\"a\");\n    p.set_extension(std::hint::black_box(\"b/c\"));",
    "let _ = std::num::Wrapping(1i32) / std::num::Wrapping(std::hint::black_box(0));",
    "let _ = std::num::Wrapping(1u32) % std::num::Wrapping(std::hint::black_box(0));",
    "let mut w = std::num::Wrapping(1u8);\n    w /= std::num::Wrapping(std::hint::black_box(0));",
    "let _ = std::num::Saturating(1i32) / std::num::Saturating(std::hint::black_box(0));",
    "let mut v = vec![1u8, 2];\n    v.drain(..std::hint::black_box(5));",
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    d.drain(..std::hint::black_box(5));",
    "let d: std::collections::VecDeque<u8> = [1, 2].into();\n    println!(\"{}\", d.range(..std::hint::black_box(5)).count());",
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    println!(\"{}\", d.range_mut(..std::hint::black_box(5)).count());",
    "let mut v = vec![1u8, 2];\n    let _ = v.splice(..std::hint::black_box(5), [9]);",
    "let mut v = vec![1u8, 2];\n    v.extend_from_within(..std::hint::black_box(5));",
    "let mut v = vec![1u8, 2];\n    println!(\"{}\", v.extract_if(..std::hint::black_box(5), |_| true).count());",
    "let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    let _ = d.split_off(std::hint::black_box(5));",
    "let mut v = vec![1u8, 2];\n    let _ = v.split_off(std::hint::black_box(5));",
    "use std::io::BufRead;\n    let mut r: &[u8] = &[1, 2];\n    r.consume(std::hint::black_box(5));",
    "use std::io::BufRead;\n    let mut d: std::collections::VecDeque<u8> = [1, 2].into();\n    d.consume(std::hint::black_box(5));",
    "let mut s = String::from(\"ab\");\n    s.drain(..std::hint::black_box(5));",
    "let mut s = String::from(\"ab\");\n    s.replace_range(..std::hint::black_box(5), \"x\");",
    "let mut v = vec![1u8, 2];\n    v.copy_within(..std::hint::black_box(5), 0);",
    "let mut s = String::from(\"ab\");\n    s.extend_from_within(..std::hint::black_box(5));",
  ];
  kinds.forEach((body, i) => {
    expect(same(`std${i}`, `fn main() {\n    ${body}\n}\n`).status).toBe(101);
  });
}, 300_000);

// A `usize`'s, whose width is wasm32's in rust-js (ADR 0562), std's line of
// its own type: against native Rust built for `wasm32-wasip1`, run by WASI,
// where a panic aborts once its hook has written, so the place it names.
test("a `usize`'s panic std raises names wasm32's std's line, as native wasm32's hook does", () => {
  const dir = fixture("usize-site");
  const source = "fn main() {\n    println!(\"{}\", 5usize.next_multiple_of(std::hint::black_box(0)));\n}\n";
  const built = nativeBinary(source, dir, [...flags, "--target=wasm32-wasip1"]);
  if ("error" in built) throw new Error(built.error);
  const at = (out: string) => /panicked at (.*):\n/.exec(out)?.[1];
  const native = at(ran([process.execPath, join(root, "test", "wasi-run.ts"), built.binary, join(dir, "native.json")], dir, true).out);
  const js = join(dir, "main.js");
  run([compiler, join(dir, "native.rs"), "-o", js, "--", ...flags, "--crate-type", "bin", "-Awarnings"]);
  installStd(dir);
  expect(native).toContain("library/core/src/num/mod.rs:");
  for (const runtime of ["node", "bun"]) expect(at(ran([runtime, js], dir).out)).toBe(native);
}, 300_000);

test("std's panics of a writer that fails, and of an `ExactSizeIterator` that lies, name std's lines", () => {
  const source = (body: string) => `use std::fmt;

struct Bad;

impl fmt::Display for Bad {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Err(fmt::Error)
    }
}

struct Lies;

impl Iterator for Lies {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        None
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (2, Some(3))
    }
}

impl ExactSizeIterator for Lies {}

fn main() {
    ${body}
}
`;
  const bodies = [
    "let _ = Bad.to_string();",
    "let _ = format!(\"{}\", Bad);",
    "println!(\"a {}\", Bad);",
    "let mut s = String::new();\n    fmt::write(&mut s, format_args!(\"{}\", Bad)).unwrap();",
    "println!(\"{}\", Lies.len());",
    "let _ = Lies;\n    let _ = Bad;",
  ];
  bodies.forEach((body, i) => {
    same(`writers${i}`, source(body));
  });
}, 300_000);

test("std's own crate's panics, and a `BufReader`'s, name std's lines", () => {
  same(
    "lazy-lock",
    `use std::sync::LazyLock;

static L: LazyLock<u8> = LazyLock::new(|| panic!("init"));

fn main() {
    let _ = std::panic::catch_unwind(|| *L);
    println!("{}", *L);
}
`,
  );
  same(
    "buf-reader-position",
    `use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

struct Behind;

impl Read for Behind {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf[..2].copy_from_slice(&[1, 2]);
        Ok(2)
    }
}

impl Seek for Behind {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}

fn main() {
    let mut r = BufReader::new(Behind);
    r.fill_buf().unwrap();
    println!("{:?}", r.stream_position());
}
`,
  );
}, 300_000);

const located = `use std::panic::Location;

#[track_caller]
fn here() -> &'static Location<'static> {
    Location::caller()
}

fn main() {
    let a = here();
    let b = here();
    let c = a;
    println!("{a}");
    println!("{a:?}");
    println!("{} {} {}", a.file(), a.line(), a.column());
    println!("{} {}", a == b, a == c);
    println!("{}", b);
    panic!("at the end");
}
`;

test("a `Location` shows, splits and compares as native Rust's", () => {
  same("located", located);
}, 300_000);

// Its file named with a `:`, a module's own, which the remapped prefix
// both gives leaves in its name: a directory's was remapped away, and the
// split was never asked of one (#1979).
test("a `Location` of a file whose path has a `:` splits as native Rust's", () => {
  const dir = fixture("colon");
  mkdirSync(dir, { recursive: true });
  const body = located.slice(located.indexOf("#[track_caller]")).replace("fn main() {", "pub fn run() {");
  writeFileSync(join(dir, "a:b.rs"), `use std::panic::Location;\n\n${body}`);
  const out = same("colon", '#[path = "a:b.rs"]\nmod colon;\n\nfn main() {\n    colon::run();\n}\n', dir);
  expect(out.out).toContain("a:b.rs 9 13");
}, 300_000);

test("an `Err` from main ends the process at once: a host's pending timer never runs", () => {
  const dir = fixture("timer");
  mkdirSync(dir, { recursive: true });
  const source = 'fn main() -> Result<(), String> {\n    Err("bad".to_string())\n}\n';
  writeFileSync(join(dir, "native.rs"), source);
  const js = join(dir, "main.js");
  run([compiler, join(dir, "native.rs"), "-o", js, "--", ...flags, "--crate-type", "bin", "-Awarnings"]);
  installStd(dir);
  // Set as the program writes its \`Err\`, as it ends: one set before would
  // run while its modules load, before \`main\` does.
  writeFileSync(
    join(dir, "timer.mjs"),
    'const write = process.stderr.write.bind(process.stderr);\nprocess.stderr.write = (...given) => {\n  setTimeout(() => console.log("late"), 0);\n  return write(...given);\n};\n',
  );
  for (const runtime of ["node", "bun"]) {
    expect(ran([runtime, "--import", "./timer.mjs", js], dir)).toEqual({ status: 1, out: 'Error: "bad"\n' });
  }
}, 300_000);
