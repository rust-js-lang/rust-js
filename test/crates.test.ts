// Separate crates (ADR 0100): each compiled once, by rust-js, to JS of its
// own and the metadata rustc reads of it, and a crate using it told what it
// needs by the manifest beside them.
//
//   test/crates/validation ◄── models ◄── frontend
//
// Natively, the same crates are rlibs, and `main` a program's. The JS
// must print what the native program does.

import { beforeAll, describe, expect, test } from "bun:test";
import { chmodSync, cpSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, buildReact, buildSerde, compiler, fixture, root, run, target } from "./support";
import { printed } from "./crates";
import { node } from "./programs";

beforeAll(buildCompiler, 600_000);

// serde and serde_json, which `models` derives and `frontend` calls, as
// natively built libraries, and as the metadata rust-js reads.
const serde = { rlib: () => buildSerde("rlib"), rmeta: () => buildSerde("rmeta") };

// Each crate, and the ones it uses, dependencies first.
const graph: [string, string[]][] = [
  ["validation", []],
  ["models", ["validation"]],
  ["frontend", ["models", "validation"]],
];

/** The crates, copied to be edited. */
function sources(dir: string): string {
  const src = join(dir, "src");
  cpSync(join(root, "test", "crates"), src, { recursive: true });
  return src;
}

/** The native program's output: each crate an rlib, and `frontend::main` a binary's `main`. */
function native(dir: string, src: string): string {
  const out = join(dir, "native");
  mkdirSync(out, { recursive: true });
  const externs = (uses: string[]) => uses.flatMap((u) => ["--extern", `${u}=${join(out, `lib${u}.rlib`)}`]);
  for (const [name, uses] of graph) {
    run(["rustc", "--edition=2024", "-Coverflow-checks=off", "--crate-type=rlib", "--crate-name", name, join(src, name, "lib.rs"),
      "-o", join(out, `lib${name}.rlib`), ...externs(uses), "-L", `dependency=${out}`, ...serde.rlib()]);
  }
  const main = join(out, "main.rs");
  writeFileSync(main, "fn main() {\n    frontend::main();\n}\n");
  run(["rustc", "--edition=2024", "-Coverflow-checks=off", main, "-o", join(out, "program"), ...externs(["frontend"]), "-L", `dependency=${out}`, ...serde.rlib()]);
  return run([join(out, "program")]);
}

const js = (dir: string, name: string) => join(dir, "js", name);

/** The command rust-js compiles `name` with: a library writes its metadata too. */
function command(dir: string, src: string, name: string, uses: string[]): string[] {
  const externs = uses.flatMap((u) => ["--extern", `${u}=${join(js(dir, u), `lib${u}.rmeta`)}`]);
  const dependencies = uses.flatMap((u) => ["--dependency", join(js(dir, u), "lib.manifest.json")]);
  const library = name === "frontend" ? [] : ["--library"];
  const metadata = name === "frontend" ? [] : [`--emit=metadata=${join(js(dir, name), `lib${name}.rmeta`)}`];
  return [compiler, join(src, name, "lib.rs"), "-o", join(js(dir, name), "lib.js"), ...library, "--manifest", join(js(dir, name), "lib.manifest.json"),
    ...dependencies, "--", "--edition=2024", "--crate-name", name, ...metadata, ...externs, ...uses.flatMap((u) => ["-L", `dependency=${js(dir, u)}`]), ...serde.rmeta()];
}

/** Each crate compiled by rust-js on its own, dependencies first, into `<dir>/js/<crate>/`. */
function compile(dir: string, src: string): string {
  for (const [name, uses] of graph) run(command(dir, src, name, uses));
  return join(js(dir, "frontend"), "lib.js");
}


test("an app using two crates, each compiled on its own, prints what native Rust does", () => {
  const dir = fixture("crates");
  const src = sources(dir);
  const app = compile(dir, src);
  expect(printed(app)).toBe(native(dir, src));
  // Each crate imports the crates it uses, where they are.
  expect(readFileSync(join(js(dir, "models"), "lib.js"), "utf8")).toContain('from "../validation/lib.js"');
  expect(readFileSync(app, "utf8")).toContain('from "../models/lib.js"');
}, 300_000);

// A crate the others use, edited: what was made from the old one is
// refused, and once each is rebuilt in order, the app is the edited program.
test("an edited library is what its consumers use once they're rebuilt, and refused before", () => {
  const dir = fixture("crates-edited");
  const src = sources(dir);
  const app = compile(dir, src);
  const before = printed(app);
  const validation = join(src, "validation", "lib.rs");
  writeFileSync(validation, readFileSync(validation, "utf8").replace("isn't an email address", "is no email address"));
  run(command(dir, src, "validation", []));
  const previous = readFileSync(app, "utf8");
  // `models` was made from the old `validation`, which its manifest says.
  expect(() => run(command(dir, src, "frontend", ["models", "validation"]))).toThrow("dependency artifact changed");
  expect(readFileSync(app, "utf8")).toBe(previous);
  const after = printed(compile(dir, src));
  expect(after).toBe(native(dir, src));
  expect(after).not.toBe(before);
}, 300_000);

// A library's JS and the metadata its consumer's rustc reads must be one
// build's: the metadata its manifest lists is fingerprinted, and what rustc
// loads, given another build's by `--extern`, is checked by its crate hash.
test("a library's manifest beside the metadata of another build is refused", () => {
  const dir = fixture("crates-mixed");
  const src = sources(dir);
  const app = compile(dir, src);
  const other = join(dir, "other");
  const again = command(dir, src, "models", ["validation"]).map((arg) =>
    arg.startsWith("--emit=metadata=") ? `--emit=metadata=${join(other, "libmodels.rmeta")}` : arg.startsWith(js(dir, "models")) ? arg.replace(js(dir, "models"), other) : arg);
  run([...again, "-Cmetadata=another-build"]);
  const previous = readFileSync(app, "utf8");
  const frontend = command(dir, src, "frontend", ["models", "validation"]).map((arg) =>
    arg === `models=${join(js(dir, "models"), "libmodels.rmeta")}` ? `models=${join(other, "libmodels.rmeta")}` : arg);
  expect(() => run(frontend)).toThrow("isn't of the build its JS was made from");
  expect(readFileSync(app, "utf8")).toBe(previous);
}, 300_000);

// Whose traits get dictionaries, and whose types drop, is decided by which
// crates are rust-js's (ADR 0100): a consumer must be told of every library
// its libraries were compiled against, or it would decide otherwise.
test("a consumer given a library but not the libraries it uses is refused", () => {
  const dir = fixture("crates-partial");
  const src = sources(dir);
  compile(dir, src);
  const full = command(dir, src, "frontend", ["models", "validation"]);
  const partial = full.filter((arg, i) => !(arg.endsWith(join("validation", "lib.manifest.json")) && full[i - 1] === "--dependency")
    && !(arg === "--dependency" && full[i + 1]?.endsWith(join("validation", "lib.manifest.json"))));
  expect(partial.length).toBe(full.length - 2);
  expect(() => run(partial)).toThrow("`models` was compiled against `validation`, a rust-js library");
}, 300_000);

// Two crates at a time (`test/crates/pairs/<case>/`): a library, `dep`, and an
// `app` using it, for each thing crossing between them that a consumer must
// be told, or must assume. Found in review: each had printed another answer.
// Three at a time (`test/crates/triples/<case>/`): `dep`, `mid` using it, and
// `app` using both, sharing what one makes through the others (ADR 0523).
const pairs = join(root, "test", "crates", "pairs");
const triples = join(root, "test", "crates", "triples");

/** `crates` of `cases/name`, each a library of the ones before it but the
 * last, the app, compiled natively and by rust-js: the app's JS prints what
 * the native program does; where each crate's JS is. */
function chain(cases: string, name: string, crates: string[]): string {
  const dir = fixture(`${crates.length === 2 ? "pair" : "triple"}-${name}`);
  const native = join(dir, "native"), out = join(dir, "js");
  mkdirSync(native, { recursive: true });
  const rustc = ["rustc", "--edition=2024", "-Coverflow-checks=off"];
  crates.forEach((crate, i) => {
    const externs = crates.slice(0, i).flatMap((used) => ["--extern", `${used}=${join(native, `lib${used}.rlib`)}`]);
    run([...rustc, "--crate-type=rlib", "--crate-name", crate, join(cases, name, `${crate}.rs`), "-o", join(native, `lib${crate}.rlib`), ...externs, "-L", `dependency=${native}`]);
  });
  const app = crates[crates.length - 1];
  writeFileSync(join(native, "main.rs"), `fn main() {\n    ${app}::main();\n}\n`);
  run([...rustc, join(native, "main.rs"), "-o", join(native, "program"), "--extern", `${app}=${join(native, `lib${app}.rlib`)}`, "-L", `dependency=${native}`]);
  crates.forEach((crate, i) => {
    const used = crates.slice(0, i);
    const library = i < crates.length - 1
      ? ["--library", "--manifest", join(out, crate, "lib.manifest.json")]
      : [];
    const metadata = i < crates.length - 1 ? [`--emit=metadata=${join(out, crate, `lib${crate}.rmeta`)}`] : [];
    run([compiler, join(cases, name, `${crate}.rs`), "-o", join(out, crate, "lib.js"), ...library,
      ...used.flatMap((u) => ["--dependency", join(out, u, "lib.manifest.json")]),
      "--", "--crate-name", crate, ...metadata,
      ...used.flatMap((u) => ["--extern", `${u}=${join(out, u, `lib${u}.rmeta`)}`, "-L", `dependency=${join(out, u)}`])]);
  });
  expect(printed(join(out, app, "lib.js"))).toBe(run([join(native, "program")]));
  return out;
}

for (const name of readdirSync(pairs).sort()) {
  test(`two crates: ${name}`, () => {
    chain(pairs, name, ["dep", "app"]);
  }, 300_000);
}
for (const name of readdirSync(triples).sort()) {
  test(`three crates: ${name}`, () => {
    chain(triples, name, ["dep", "mid", "app"]);
  }, 300_000);
}

// ADR 0693, amended: a library's function, which nothing assigns, returned
// past a guard's drop is returned after its `finally`, as the crate's own is.
test("a library's function returned past a guard's drop is returned after its finally", () => {
  const out = chain(pairs, "returned_item", ["dep", "app"]);
  expect(readFileSync(join(out, "app", "lib.js"), "utf8")).toContain("    depth = previous;\n  }\n  return noop;\n}");
}, 300_000);

// ADR 0844: a library that reads no count of its `Rc`s counts none, and its
// contract says so: its JS is the value itself, a context's
// create-then-assign, as is its consumer's. A consumer that counts one it
// hands is refused, naming it; one given a library's generic `Rc` of what
// may have a destructor counts, as that library does (`rc_generic_holder`).
test("a library that reads no count counts no Rc, and a crate counting one it hands is refused", () => {
  const out = chain(pairs, "rc_uncounted", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("  context.provider = context;\n  context.consumer = {\n    context,\n  };\n  return context;\n");
  expect(dep).toContain("export function sharedCell(value, TCopy) {\n  return { value: TCopy ? TCopy.copy(value) : value };\n}");
  expect(dep).not.toContain("strong");
  expect(readFileSync(join(out, "app", "lib.js"), "utf8")).not.toContain("strong");
  expect(contract(out).counted_rcs).toBeUndefined();
  expect(() => counting(out, "pub fn main() {\n    let context = dep::create_context(1);\n    println!(\"{}\", std::rc::Rc::strong_count(&context));\n}\n"))
    .toThrow("this crate counts an `Rc<dep::Context>`, whose counts it reads, whose last owner runs a destructor, or that may be one that does, but `dep`, a rust-js library that counts none of them, hands it one");
  const holder = chain(pairs, "rc_generic_holder", ["dep", "app"]);
  expect(contract(holder).counted_rcs).toEqual([{}]);
}, 300_000);

// ADR 0852: a library counts the `Rc`s of each type apart, and its contract
// says which. React's context, read by `use_context<T>` of any `T`, is the
// value itself beside a generic `Rc<Slot<T>>` it counts, as is its
// consumer's; the consumer counts the library's `Slot`s, and is refused
// counting its context, or a `String` a library that counts others hands it.
test("a library counts each type's Rcs apart, and its consumer as it does", () => {
  const out = chain(pairs, "rc_per_type", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("export function createContext(defaultValue, TCopy) {\n  const context = {\n    value: TCopy ? TCopy.copy(defaultValue) : defaultValue,\n    // These are circular\n    provider: null,\n    consumer: null,\n  };\n  context.provider = context;\n");
  expect(dep).toContain("export function useContext(context) {\n  const consumer = context.consumer;\n");
  expect(dep).toContain("  const slot = { value: { value }, strong: 1, weak: 0 };\n");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("  const context = createContext(1);\n  console.log(`${useContext(context)} ${useContext(context)}`);\n  const again = context;\n");
  expect(app).toContain("  again.value = 2;\n");
  expect(app).toContain("console.log(`${a.value.value[0]} ${a.strong}`);");
  expect(contract(out).counted_rcs).toEqual([{ of: expect.any(String), args: [{}] }]);
  expect(() => counting(out, "pub fn main() {\n    let context = dep::create_context(1);\n    println!(\"{}\", std::rc::Rc::strong_count(&context));\n}\n"))
    .toThrow("this crate counts an `Rc<dep::Context<i32>>`");
  const shared = chain(triples, "rc_shared", ["dep", "mid", "app"]);
  expect(() => counting(shared, "pub fn main() {\n    println!(\"{}\", std::rc::Rc::strong_count(&dep::tag(\"t\")));\n}\n"))
    .toThrow("this crate counts an `Rc<std::string::String>`");
}, 300_000);

// ADR 0289, amended: a library's generic function of a `T: Copy` takes a
// copy only from a caller whose value may not be its own copy. JS, whose
// values are, calls it as its declarations say, as React's JS calls
// `createContext(defaultValue)`, and gets what the program's would give;
// its consumer gives a `Point` it changes a copy, and a number none.
test("a library's Copy bound takes a copy only where one is given, and JS calls it as declared", () => {
  const out = chain(pairs, "copy_given", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("export function createContext(defaultValue, TCopy) {\n");
  expect(dep).toContain("    currentValue: TCopy ? TCopy.copy(defaultValue) : defaultValue,\n");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("createContext({ x: 1 }, { copy: (value) => ({ ...value }) })");
  expect(app).toContain("createContext(7)");
  expect(app).toContain("twice(8)");
  const lib = JSON.stringify(join(out, "dep", "lib.js"));
  const called = `const dep = await import(${lib});
const object = { name: "default" };
const context = dep.createContext(object);
console.log(context.currentValue === object, context.currentValue2 === object);
const [a, b] = dep.twice(object);
console.log(a === object, b === object, dep.first([object]) === object, dep.first([]));`;
  expect(run([node ?? "node", "--input-type=module", "--eval", called])).toBe("true true\ntrue true true undefined\n");
}, 300_000);

// ADR 0863: a library's `RefCell`s, as React's `Transition` holds its
// types: a plain field the value it holds, an `Rc`'s `RefCell<Vec<String>>`
// the array, an `Option`'s `RefCell<HashSet<u32>>` the `Set`, none of whose
// borrows is checked, as none can meet another. Its consumer agrees, or is
// refused where it would hold one while the library's code runs, lend the
// plain field, or box what the library hands it as the object it holds.
test("a library's RefCells are the values they hold, and its consumer agrees or is refused", () => {
  const out = chain(pairs, "ref_cells", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("  const current = {\n    seen: new Set(),\n  };\n  current.types = prev?.types;\n");
  expect(dep).toContain("    transition.types = [ty];\n  } else if (!types.some((t) => t === ty)) {\n    types.push(ty);\n");
  expect(dep).toContain("    return a === b;\n");
  expect(dep).toContain("    const count = transition.seen.size;\n    transition.seen.clear();\n");
  expect(dep).not.toContain("$borrow");
  // A transparent struct of a `RefCell` of an array, the array, on both
  // sides, as JS makes it (ADRs 0584, 0863).
  expect(dep).toContain("  queue.push(value);\n  return queue.length;\n");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("  const queue = [\"q\"];\n");
  expect(app).toContain("  queue.push(\"s\");\n");
  expect(app).toContain("  const replaced = inner.types;\n  inner.types = undefined;\n");
  expect(app).not.toContain("$borrow");
  // A consumer that only makes the library's struct makes its plain field
  // the value it holds, as the library reads it.
  counting(out, "use std::cell::RefCell;\n\npub fn main() {\n    let made = dep::Transition {\n        types: RefCell::new(None),\n        seen: None,\n    };\n    dep::add_type(&made, \"x\");\n    println!(\"{:?}\", dep::types(&made));\n}\n");
  expect(readFileSync(join(out, "counting", "lib.js"), "utf8")).toContain("  const made = {};\n");
  const facts = contract(out).ref_cells;
  expect([facts.unchecked.length, facts.plain.length, facts.counted, facts.boxed]).toEqual([3, 1, undefined, undefined]);
  expect(() => counting(out, "pub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    let held = types.borrow_mut();\n    dep::add_type(&t, \"b\");\n    println!(\"{}\", held.len());\n}\n"))
    .toThrow("is held while code rust-js can't see runs, which may be `dep`'s");
  expect(() => counting(out, "pub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    let n = types.borrow().len() + dep::types(&t).len();\n    println!(\"{n}\");\n}\n"))
    .toThrow("is held while code rust-js can't see runs, which may be `dep`'s");
  expect(() => counting(out, "use std::cell::RefCell;\nuse std::rc::Rc;\n\nfn some(types: &RefCell<Option<Rc<RefCell<Vec<String>>>>>) -> bool {\n    types.borrow().is_some()\n}\n\npub fn main() {\n    let t = dep::start(None);\n    println!(\"{}\", some(&t.types));\n}\n"))
    .toThrow("`dep`, a rust-js library, has a field's");
  expect(() => counting(out, "use std::cell::RefCell;\n\nfn get<T: Clone>(cell: &RefCell<T>) -> T {\n    cell.borrow().clone()\n}\n\npub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    println!(\"{:?}\", get(&types));\n}\n"))
    .toThrow("as a `{ value }`");
  // One whose generic code has a `RefCell<T>` of any `T` boxes each, and
  // one that holds a borrow while what it's given runs counts it: its
  // consumer boxes and checks them as it does.
  const counted = chain(pairs, "ref_cells_counted", ["dep", "app"]);
  expect([contract(counted).ref_cells.boxed, contract(counted).ref_cells.counted.length]).toEqual([[{}], 1]);
  const consumer = readFileSync(join(counted, "app", "lib.js"), "utf8");
  expect(consumer).toContain("        const borrowMut = $borrowMut(list.value, true);\n");
  expect(consumer).toContain("      value.value.push(3);\n");
}, 300_000);

/** The library contract of `out`'s `dep`. */
function contract(out: string) {
  return JSON.parse(readFileSync(join(out, "dep", "lib.manifest.json"), "utf8")).library;
}

/** `source`, a crate using `out`'s `dep`, compiled. */
function counting(out: string, source: string) {
  const file = join(out, "counting.rs");
  writeFileSync(file, source);
  run([compiler, file, "-o", join(out, "counting", "lib.js"), "--dependency", join(out, "dep", "lib.manifest.json"),
    "--", "--crate-name", "counting", "--extern", `dep=${join(out, "dep", "libdep.rmeta")}`, "-L", `dependency=${join(out, "dep")}`]);
}

// A library edited to read an `Rc`'s count, its API the same: rebuilt with
// its consumers, they share its counted `Rc`s as before, and print what
// native Rust does (ADR 0523).
test("a library that starts reading a count is what its consumers share once rebuilt", () => {
  const cases = fixture("triples-edited");
  cpSync(join(triples, "rc_shared"), join(cases, "rc_shared"), { recursive: true });
  chain(cases, "rc_shared", ["dep", "mid", "app"]);
  const dep = join(cases, "rc_shared", "dep.rs");
  const read = 'println!("dep drops {} of {}", rc.0, Rc::strong_count(&rc));';
  writeFileSync(dep, readFileSync(dep, "utf8").replace('println!("dep drops {}", rc.0);', read));
  expect(readFileSync(dep, "utf8")).toContain(read);
  chain(cases, "rc_shared", ["dep", "mid", "app"]);
}, 300_000);

// A library's generic function is given a drop for a type parameter only
// where its body drops one, itself or through what it calls (ADR 0300): one
// that moves its value first, or passes it on to one that does, takes none.
// `two crates: drops_by_need` runs it against native Rust.
test("a library's generic function takes the drops its body uses", () => {
  const dir = fixture("drops-by-need");
  run([compiler, join(pairs, "drops_by_need", "dep.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const signatures = js.split("\n").filter((line) => line.startsWith("export function"));
  expect(signatures).toEqual([
    "export function keep(value) {",
    "export function pick(a, b, dropB) {",
    "export function relay(value) {",
    "export function discard(value, dropT) {",
    "export function apply(f, a) {",
    "export function keepBy(value) {",
  ]);
  const items = JSON.parse(readFileSync(join(dir, "lib.manifest.json"), "utf8")).library.items;
  // Each type parameter a function takes a drop of, by its record in `params`.
  type Item = { export: string; params?: { index: number; drop?: boolean }[] };
  const dropsOf = (item: Item) => (item.params ?? []).filter((param) => param.drop).map((param) => param.index);
  expect(Object.fromEntries(items.map((item: Item) => [item.export, dropsOf(item)]))).toEqual({
    keep: [], pick: [1], relay: [], discard: [0], apply: [], keepBy: [],
  });
  // What each call gives: only the drops its callee takes, and a function
  // given as a value is itself.
  expect(js).toContain("return keep(value);");
  expect(js).toContain("pick(undefined, value, dropT);");
  expect(js).toContain("return apply(keep, value);");
}, 300_000);

// A library's generic trait method is given a drop for its own type
// parameters, as its trait declares them (ADR 0163): a consumer's value with
// a destructor is dropped where Rust drops it, at the end of `put`, called on
// the impl or through a dictionary. A library with no destructor of its own
// still takes the drop: its consumers may have one.
test("a library's generic trait method drops a consumer's value where Rust does", () => {
  const dir = fixture("generic-method-drop");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub trait Put {\n    fn put<B>(&self, b: B) -> u32;\n}\npub struct Sink;\nimpl Put for Sink {\n    fn put<B>(&self, _b: B) -> u32 {\n        1\n    }\n}\n");
  writeFileSync(join(dir, "app.rs"), "use dep::Put;\nstruct Loud;\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!(\"dropped\");\n    }\n}\nfn via<P: Put>(p: &P) -> u32 {\n    p.put(Loud)\n}\npub fn main() {\n    println!(\"{}\", dep::Sink.put(Loud));\n    println!(\"{}\", via(&dep::Sink));\n    println!(\"after\");\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("dropped\n1\ndropped\n1\nafter\n");
}, 300_000);

// A library's generic function that drops a std trait's associated type of
// its type parameter, `I::IntoIter`, is given its drop by a consumer, which
// knows what it is (ADR 0421): its manifest lists the drop, and the
// consumer's what's left of a `Vec`'s, and its own iterator's, given as a JS
// iterator, dropped where Rust drops them. A trait's of the library's has its
// drop in its dictionary instead.
test("a library's generic function drops a consumer's iterator where Rust does", () => {
  const dir = fixture("library-item-drops");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub fn first<I: IntoIterator>(items: I) -> Option<I::Item> {\n    let mut it = items.into_iter();\n    it.next()\n}\npub trait Zone {\n    type Offset;\n    fn offset(&self) -> Self::Offset;\n}\npub fn hold<Z: Zone>(zone: &Z) {\n    let _offset = zone.offset();\n}\n");
  writeFileSync(join(dir, "app.rs"), "struct Loud(&'static str);\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!(\"drop {}\", self.0);\n    }\n}\nstruct Count(u32);\nimpl Iterator for Count {\n    type Item = u32;\n    fn next(&mut self) -> Option<u32> {\n        self.0 += 1;\n        Some(self.0)\n    }\n}\nimpl Drop for Count {\n    fn drop(&mut self) {\n        println!(\"count dropped at {}\", self.0);\n    }\n}\npub fn main() {\n    let found = dep::first(vec![Loud(\"a\"), Loud(\"b\")]);\n    println!(\"{}\", found.map_or(\"none\", |l| l.0));\n    println!(\"{:?}\", dep::first(Count(4)));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  const js = readFileSync(join(lib, "lib.js"), "utf8");
  expect(js).toContain("export function first(items, dropIIntoIter) {");
  // A trait's of the crate's, whose dictionary has its drop (ADR 0178), takes none.
  expect(js).toContain("export function hold(zone, ZZone) {");
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("drop b\ndrop a\na\ncount dropped at 5\nSome(5)\n");
}, 300_000);

// A library's generic fold drops the iterator its consumer gives once it's
// done (ADR 0464); its peekable, which gives it back, is std's own, dropped
// with what it holds when the consumer consumes it (ADR 0561).
test("a library's generic fold drops what its consumer gives, and its peekable drops what it holds", () => {
  const dir = fixture("library-no-drops");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub fn total<I: Iterator<Item = u32>>(items: I) -> u32 {\n    items.fold(0, |a, b| a + b)\n}\npub fn peeked<I: Iterator<Item = u32>>(items: I) -> std::iter::Peekable<I> {\n    items.peekable()\n}\npub fn first_long<I: Iterator<Item = B>, B: AsRef<str>>(items: I) -> Option<usize> {\n    for item in items {\n        if item.as_ref().len() > 2 {\n            return Some(item.as_ref().len());\n        }\n    }\n    None\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  const app = (source: string) => {
    writeFileSync(join(dir, "app.rs"), source);
    return Bun.spawnSync([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
      "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  };
  expect(app("pub fn main() {\n    println!(\"{} {:?}\", dep::total([1, 2, 3].into_iter()), dep::first_long([\"a\", \"bcd\"].into_iter()));\n}\n").exitCode).toBe(0);
  expect(printed(join(dir, "app.js"))).toBe("6 Some(3)\n");
  const counter = "pub struct Counter(pub u32);\nimpl Iterator for Counter {\n    type Item = u32;\n    fn next(&mut self) -> Option<u32> {\n        None\n    }\n}\nimpl Drop for Counter {\n    fn drop(&mut self) {\n        println!(\"dropped\");\n    }\n}\n";
  expect(app(`${counter}pub fn main() {\n    println!("{}", dep::total(Counter(0)));\n}\n`).exitCode).toBe(0);
  expect(printed(join(dir, "app.js"))).toBe("dropped\n0\n");
  // A `Peekable` the library gives back, std's own (ADR 0561), drops what
  // it holds when the consumer consumes it, as Rust's `sum` does.
  expect(app(`${counter}pub fn main() {\n    println!("{}", dep::peeked(Counter(0)).sum::<u32>());\n}\n`).exitCode).toBe(0);
  expect(printed(join(dir, "app.js"))).toBe("dropped\n0\n");
}, 300_000);

// A library's writer that fails, as chrono's formatting does (ADR 0187): its
// manifest says so, and a consumer's `write!` of it gives the `fmt::Error`
// back, with what was written before it.
test("a library's writer that may fail is one its consumer catches", () => {
  const dir = fixture("library-fmt-error");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "use std::fmt::{self, Write};\n\npub struct Hours(pub u32);\n\nimpl fmt::Display for Hours {\n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {\n        if self.0 > 23 {\n            return Err(fmt::Error);\n        }\n        write!(f, \"{:02}h\", self.0)\n    }\n}\n\npub fn check(h: &Hours) -> fmt::Result {\n    let mut s = String::new();\n    write!(s, \"[{h}]\")\n}\n");
  writeFileSync(join(dir, "app.rs"), "use std::fmt::Write;\n\npub fn main() {\n    let mut s = String::new();\n    let r = write!(s, \"{} {}\", dep::Hours(5), dep::Hours(50));\n    println!(\"{} {s:?}\", r.is_err());\n    println!(\"{}\", dep::Hours(7));\n    println!(\"{:?} {:?}\", dep::check(&dep::Hours(8)), dep::check(&dep::Hours(80)));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("true \"05h \"\n07h\nOk(()) Err(Error)\n");
}, 300_000);

// A library's generic code that writes to any `fmt::Write` is given a
// consumer's `Formatter`, as bitflags' macro hands its `to_writer` the one
// its `Debug` is given (ADR 0180): the library's, compiled on its own.
test("a library's generic writer is given a consumer's Formatter", () => {
  const dir = fixture("library-fmt-writer");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "use std::fmt::{self, Write};\n\npub fn to_writer<W: Write>(n: u8, mut w: W) -> fmt::Result {\n    write!(w, \"<{}>\", n)\n}\n");
  writeFileSync(join(dir, "app.rs"), "use std::fmt;\n\nstruct Perms(u8);\n\nimpl fmt::Debug for Perms {\n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {\n        dep::to_writer(self.0, f)\n    }\n}\n\npub fn main() {\n    let mut s = String::new();\n    dep::to_writer(9, &mut s).unwrap();\n    println!(\"{:?} {s}\", Perms(3));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("<3> <9>\n");
}, 300_000);

// Whether a writer takes its `Formatter`'s options is its own crate's, which
// a library's manifest says (ADR 0137): a consumer that gives options gives
// none to a library's writer that takes none, whose `write!` ignores a width
// as Rust's does, and one that gives none still gives a library's writer
// that takes them its place, before the dictionaries after it.
test("a library's writer is given its Formatter's options as its own crate takes them", () => {
  const dir = fixture("library-fmt-options");
  for (const [name, source] of [
    ["plain", "use std::fmt;\n\npub struct Kebab<T: AsRef<str>>(pub T);\n\nimpl<T: AsRef<str>> fmt::Display for Kebab<T> {\n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {\n        write!(f, \"{}\", self.0.as_ref().replace(' ', \"-\"))\n    }\n}\n"],
    ["padded", "use std::fmt;\n\npub struct Pad<T: AsRef<str>>(pub T);\n\nimpl<T: AsRef<str>> fmt::Display for Pad<T> {\n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {\n        f.pad(self.0.as_ref())\n    }\n}\n\npub fn show<T: AsRef<str>>(text: &T, f: &mut fmt::Formatter) -> fmt::Result {\n    f.pad(text.as_ref())\n}\n"],
  ]) {
    mkdirSync(join(dir, name), { recursive: true });
    writeFileSync(join(dir, `${name}.rs`), source);
    run([compiler, join(dir, `${name}.rs`), "-o", join(dir, name, "lib.js"), "--library", "--manifest", join(dir, name, "lib.manifest.json"),
      "--", "--crate-name", name, `--emit=metadata=${join(dir, name, `lib${name}.rmeta`)}`]);
  }
  const app = (name: string, source: string) => {
    writeFileSync(join(dir, `${name}.rs`), source);
    run([compiler, join(dir, `${name}.rs`), "-o", join(dir, `${name}.js`),
      "--dependency", join(dir, "plain", "lib.manifest.json"), "--dependency", join(dir, "padded", "lib.manifest.json"),
      "--", "--crate-name", name, "--crate-type=lib",
      "--extern", `plain=${join(dir, "plain", "libplain.rmeta")}`, "--extern", `padded=${join(dir, "padded", "libpadded.rmeta")}`]);
    return printed(join(dir, `${name}.js`));
  };
  expect(app("widths", "pub fn main() {\n    println!(\"[{:>9}] [{:>4}]\", plain::Kebab(\"a b\"), padded::Pad(\"x\"));\n}\n")).toBe("[a-b] [   x]\n");
  expect(app("none", "use std::fmt;\n\nstruct Shown(String);\n\nimpl fmt::Display for Shown {\n    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {\n        padded::show(&self.0, f)\n    }\n}\n\npub fn main() {\n    println!(\"[{}] [{}] [{}]\", plain::Kebab(\"a b\"), padded::Pad(String::from(\"x\")), Shown(String::from(\"y\")));\n}\n")).toBe("[a-b] [x] [y]\n");
}, 300_000);

// A library's generic code may ask a consumer that instantiates it for an
// impl of a type of its own the consumer can't name, as base64's
// `EncoderStringWriter` asks for its private `Utf8SingleCodeUnitWriter`'s
// `io::Write`: every trait impl is exported (ADR 0100).
test("a library's private type's impl is given to its generic code by a consumer", () => {
  const dir = fixture("library-private-impl");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub trait Sink {\n    fn put(&mut self, s: &str);\n}\n\nstruct Upper(String);\n\nimpl Sink for Upper {\n    fn put(&mut self, s: &str) {\n        self.0.push_str(&s.to_uppercase());\n    }\n}\n\npub struct Writer<S> {\n    sink: S,\n}\n\nimpl<S: Sink> Writer<S> {\n    pub fn write(&mut self, s: &str) {\n        self.sink.put(s);\n    }\n}\n\npub fn shouting() -> Writer<impl Sink> {\n    Writer { sink: Upper(String::new()) }\n}\n\nimpl Writer<Upper> {\n    pub fn text(&self) -> &str {\n        &self.sink.0\n    }\n}\n\npub fn shout(words: &[&str]) -> String {\n    let mut w = Writer { sink: Upper(String::new()) };\n    for word in words {\n        w.write(word);\n    }\n    w.text().to_string()\n}\n");
  writeFileSync(join(dir, "app.rs"), "pub fn main() {\n    let mut w = dep::shouting();\n    w.write(\"hi \");\n    w.write(\"there\");\n    println!(\"{}\", dep::shout(&[\"a\", \"b\"]));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("AB\n");
}, 300_000);

// A library's `Hash` impls, derived and its own, and its hasher, are given
// to its consumer's code, and the consumer's type to the library's generic
// code, as any impl is (ADR 0463): each hash std's, as native Rust's.
test("a library's Hash impls and hasher hash in its consumer", () => {
  const dir = fixture("library-hash");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "use std::hash::{DefaultHasher, Hash, Hasher};\n\n#[derive(Hash)]\npub struct Key {\n    pub id: u32,\n    pub name: String,\n}\n\npub struct Folded(pub String);\n\nimpl Hash for Folded {\n    fn hash<H: Hasher>(&self, state: &mut H) {\n        self.0.to_lowercase().hash(state);\n    }\n}\n\n#[derive(Default)]\npub struct Sum(u64);\n\nimpl Hasher for Sum {\n    fn finish(&self) -> u64 {\n        self.0\n    }\n\n    fn write(&mut self, bytes: &[u8]) {\n        for &b in bytes {\n            self.0 = self.0 * 31 + u64::from(b);\n        }\n    }\n}\n\npub fn digest<T: Hash>(value: &T) -> u64 {\n    let mut hasher = DefaultHasher::new();\n    value.hash(&mut hasher);\n    hasher.finish()\n}\n");
  writeFileSync(join(dir, "app.rs"), "use std::hash::{DefaultHasher, Hash, Hasher};\n\n#[derive(Hash)]\nstruct Mine(u8, &'static str);\n\nfn sum<T: Hash>(value: &T) -> u64 {\n    let mut hasher = dep::Sum::default();\n    value.hash(&mut hasher);\n    hasher.finish()\n}\n\npub fn main() {\n    let key = dep::Key { id: 1, name: \"a\".into() };\n    let mut hasher = DefaultHasher::new();\n    key.hash(&mut hasher);\n    println!(\"{} {}\", hasher.finish(), dep::digest(&key));\n    println!(\"{}\", dep::digest(&dep::Folded(\"AbC\".into())) == dep::digest(&dep::Folded(\"abc\".into())));\n    println!(\"{} {}\", dep::digest(&Mine(2, \"b\")), sum(&Mine(2, \"b\")));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("18401673941424689880 18401673941424689880\ntrue\n5997443710218998710 5215\n");
}, 300_000);

// A library's function that asks the size of its type parameter is told it
// by its consumer, as its manifest says (ADR 0145), directly or through the
// consumer's own generic function, and a library's impl's dictionary too.
test("a library's generic function is given the size of its consumer's type", () => {
  const dir = fixture("library-type-facts");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub fn bytes<T>(n: usize) -> usize {\n    std::mem::size_of::<T>() * n\n}\n\npub trait Footprint {\n    fn footprint(&self) -> usize;\n}\n\npub struct Wrap<T>(pub Vec<T>);\n\nimpl<T> Footprint for Wrap<T> {\n    fn footprint(&self) -> usize {\n        bytes::<T>(self.0.len())\n    }\n}\n");
  writeFileSync(join(dir, "app.rs"), "use dep::Footprint;\n\nfn twice<T>(n: usize) -> usize {\n    2 * dep::bytes::<T>(n)\n}\n\nfn total<F: Footprint>(f: &F) -> usize {\n    f.footprint()\n}\n\npub fn main() {\n    let words = dep::Wrap(vec![1u32, 2]);\n    println!(\"{} {} {} {}\", dep::bytes::<u64>(3), twice::<u16>(5), words.footprint(), total(&dep::Wrap(vec![1u8])));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("24 20 8 1\n");
}, 300_000);

// A library that asks a `Vec` its capacity keeps each's, and so does its
// consumer, whose vectors it may be given (ADR 0419): one made with room,
// or shortened, keeps it in the library's eyes.
test("a library that asks a Vec its capacity has its consumer keep each's", () => {
  const dir = fixture("library-capacity");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub fn room(v: &Vec<u32>) -> usize {\n    v.capacity() - v.len()\n}\n");
  writeFileSync(join(dir, "app.rs"), "pub fn main() {\n    let mut v: Vec<u32> = Vec::with_capacity(8);\n    v.push(1);\n    let mut w = vec![1, 2, 3];\n    w.clear();\n    println!(\"{} {}\", dep::room(&v) >= 7, dep::room(&w) >= 3);\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("true true\n");
}, 300_000);

// A library's trait's default is the library's (ADR 0185): a consumer's impl
// that doesn't write it calls the library's, which calls the impl's methods,
// and its overrides, through its dictionary.
test("a library's trait's defaults are what a consumer's impl doesn't write", () => {
  const dir = fixture("library-defaults");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub trait Counter {\n    fn get(&self) -> u32;\n    fn make(n: u32) -> Self\n    where\n        Self: Sized;\n    fn double(&self) -> u32 {\n        self.get() * 2\n    }\n    fn quadruple(&self) -> u32 {\n        self.double() * 2\n    }\n    fn zero() -> Self\n    where\n        Self: Sized,\n    {\n        Self::make(0)\n    }\n    fn label(&self) -> String {\n        format!(\"count {}\", self.get())\n    }\n    fn consume(self) -> u32\n    where\n        Self: Sized,\n    {\n        self.get() + 100\n    }\n    fn bump(&mut self)\n    where\n        Self: Sized,\n    {\n        let next = Self::make(self.get() + 1);\n        *self = next;\n    }\n}\npub fn total<C: Counter>(c: &C) -> u32 {\n    c.quadruple() + c.get()\n}\n");
  writeFileSync(join(dir, "app.rs"), "use dep::Counter;\nstruct One(u32);\nimpl Counter for One {\n    fn get(&self) -> u32 {\n        self.0\n    }\n    fn make(n: u32) -> Self {\n        One(n)\n    }\n}\nstruct Odd(u32);\nimpl Counter for Odd {\n    fn get(&self) -> u32 {\n        self.0\n    }\n    fn make(n: u32) -> Self {\n        Odd(n + 1)\n    }\n    fn double(&self) -> u32 {\n        self.0 * 2 + 1\n    }\n}\nstruct Loud(u32);\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!(\"dropped {}\", self.0);\n    }\n}\nimpl Counter for Loud {\n    fn get(&self) -> u32 {\n        self.0\n    }\n    fn make(n: u32) -> Self {\n        Loud(n)\n    }\n}\nfn via<C: Counter>(c: &C) -> u32 {\n    c.double()\n}\nfn bumped<C: Counter>(mut c: C) -> u32 {\n    c.bump();\n    c.get()\n}\npub fn main() {\n    println!(\"{} {} {}\", One(3).double(), One(3).quadruple(), One(3).label());\n    println!(\"{} {}\", Odd(3).double(), Odd(3).quadruple());\n    println!(\"{} {}\", One::zero().get(), Odd::zero().get());\n    println!(\"{} {}\", via(&One(5)), via(&Odd(5)));\n    println!(\"{} {}\", dep::total(&One(1)), dep::total(&Odd(1)));\n    println!(\"{}\", Loud(7).consume());\n    let mut one = One(8);\n    one.bump();\n    println!(\"{} {}\", one.get(), bumped(Odd(8)));\n    let mut loud = Loud(9);\n    loud.bump();\n    println!(\"{} {}\", loud.get(), bumped(Loud(20)));\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  expect(printed(join(dir, "app.js"))).toBe("6 12 count 3\n7 14\n0 1\n10 11\n5 7\ndropped 7\n107\n9 10\ndropped 9\ndropped 20\ndropped 21\n10 21\ndropped 10\n");
}, 300_000);

// A library's JS and its metadata are one build's (ADR 0100): rustc writes
// the metadata where it's staged, and it's published with the JS, from one
// plan, or neither is. Found in review, each: JS published beside metadata
// that couldn't be written; metadata replaced beside JS that couldn't be; and
// metadata written over a source of the crate.
describe("a library's metadata is published with its JS, or neither is", () => {
  const library = (dir: string) => {
    const source = join(dir, "lib.rs"), output = join(dir, "js", "lib.js");
    writeFileSync(source, "pub mod data;\npub fn one() -> u32 {\n    data::N\n}\n");
    writeFileSync(join(dir, "data.rs"), "pub const N: u32 = 1;\n");
    const build = (metadata: string) =>
      run([compiler, source, "-o", output, "--library", "--manifest", join(dir, "js", "lib.manifest.json"), "--", "--crate-name", "dep", `--emit=metadata=${metadata}`]);
    return { source, output, build, edit: () => writeFileSync(join(dir, "data.rs"), "pub const N: u32 = 2;\n") };
  };

  test("metadata that can't be published publishes nothing", () => {
    const dir = fixture("metadata-blocked");
    const { output, build, edit } = library(dir);
    build(join(dir, "js", "libdep.rmeta"));
    const [js, meta] = [readFileSync(output, "utf8"), readFileSync(join(dir, "js", "libdep.rmeta"))];
    edit();
    writeFileSync(join(dir, "blocker"), "a file, not a directory\n");
    expect(() => build(join(dir, "blocker", "libdep.rmeta"))).toThrow();
    expect(readFileSync(output, "utf8")).toBe(js);
    expect(readFileSync(join(dir, "js", "libdep.rmeta")).equals(meta)).toBe(true);
    expect(readdirSync(join(dir, "js")).filter((f) => f.startsWith(".rust-js"))).toEqual([]);
  }, 300_000);

  // Staged before the crate's settings are read: a build they refuse
  // leaves no staging of its own behind (the second architecture review).
  test("a build its settings refuse leaves nothing staged", () => {
    const dir = fixture("metadata-settings");
    const { build } = library(dir);
    writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "dep"\nversion = "0.0.0"\nedition = "2024"\n[package.metadata.rust-js.format]\nprintWidth = 0\n');
    mkdirSync(join(dir, "js"), { recursive: true });
    expect(() => build(join(dir, "js", "libdep.rmeta"))).toThrow();
    expect(readdirSync(join(dir, "js")).filter((f) => f.startsWith(".rust-js"))).toEqual([]);
  }, 300_000);

  test("JS that can't be published leaves the metadata of its build too", () => {
    const dir = fixture("metadata-readonly-js");
    const { output, build, edit } = library(dir);
    mkdirSync(join(dir, "meta"));
    const metadata = join(dir, "meta", "libdep.rmeta");
    build(metadata);
    const [js, meta] = [readFileSync(output, "utf8"), readFileSync(metadata)];
    edit();
    chmodSync(join(dir, "js"), 0o555);
    try {
      expect(() => build(metadata)).toThrow();
    } finally {
      chmodSync(join(dir, "js"), 0o755);
    }
    expect(readFileSync(output, "utf8")).toBe(js);
    expect(readFileSync(metadata).equals(meta)).toBe(true);
  }, 300_000);

  // rustc's other outputs would be written past rust-js's checks. Found in
  // review: `mir=` beside `metadata=` wrote over a module's source.
  test("rustc's other outputs are refused, however they're spelled", () => {
    const dir = fixture("metadata-and-mir");
    const { source, output } = library(dir);
    const data = join(dir, "data.rs");
    const before = readFileSync(data, "utf8");
    for (const emit of [[`--emit=metadata=${join(dir, "libdep.rmeta")},mir=${data}`], ["--emit", `mir=${data}`]]) {
      expect(() => run([compiler, source, "-o", output, "--library", "--manifest", join(dir, "m.json"), "--", "--crate-name", "dep", ...emit]))
        .toThrow("isn't something rust-js writes");
      expect(readFileSync(data, "utf8")).toBe(before);
    }
  }, 300_000);

  test("metadata asked for where a source of the crate is, is refused", () => {
    const dir = fixture("metadata-over-source");
    const { build } = library(dir);
    const data = join(dir, "data.rs");
    const before = readFileSync(data, "utf8");
    expect(() => build(data)).toThrow("output collision");
    expect(readFileSync(data, "utf8")).toBe(before);
  }, 300_000);
});
// A component another crate declares is JSX's too (ADR 0110): its props
// `macro`, which the consumer expands, may write `#[rust_js::jsx]` on an
// expression, as its own crate's does, on a stable release.
test("a component of another crate is a JSX tag, on a stable release", () => {
  buildReact();
  const dir = fixture("cross-crate-jsx");
  const { RUSTC_BOOTSTRAP: _, ...env } = process.env;
  const react = ["--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target];
  const ui = join(dir, "ui"), app = join(dir, "app");
  mkdirSync(ui, { recursive: true });
  mkdirSync(app, { recursive: true });
  writeFileSync(join(ui, "lib.rs"), "#![allow(non_snake_case)]\nuse react::{JSX, jsx};\npub struct Props {\n    pub label: &'static str,\n}\npub fn Button(p: Props) -> JSX::Element {\n    jsx! { <button>{p.label}</button> }\n}\n");
  writeFileSync(join(app, "lib.rs"), "#![allow(non_snake_case)]\nuse react::{JSX, jsx};\npub fn App() -> JSX::Element {\n    jsx! { <div><ui::Button label=\"go\" /></div> }\n}\n");
  const built = (args: string[]) => {
    const p = Bun.spawnSync([compiler, ...args], { env });
    expect(p.stderr.toString()).toBe("");
    expect(p.exitCode).toBe(0);
  };
  built([join(ui, "lib.rs"), "-o", join(ui, "js", "lib.jsx"), "--library", "--manifest", join(ui, "js", "lib.manifest.json"),
    "--", "--crate-name", "ui", `--emit=metadata=${join(ui, "js", "libui.rmeta")}`, ...react]);
  built([join(app, "lib.rs"), "-o", join(app, "js", "lib.jsx"), "--dependency", join(ui, "js", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `ui=${join(ui, "js", "libui.rmeta")}`, ...react]);
  expect(readFileSync(join(app, "js", "lib.jsx"), "utf8")).toContain('<Button label="go" />');
});

// A `#![no_std]` crate loads no `alloc`, so it has no `ToString`, which
// every trait call was compared with: bitflags and num-traits crashed
// rust-js there (docs/crate-corpus.md).
test("a #![no_std] crate's trait calls compile, though it has no ToString", async () => {
  const dir = fixture("no-std");
  writeFileSync(join(dir, "lib.rs"), [
    "#![no_std]",
    "pub trait Area {",
    "    fn area(&self) -> i32;",
    "}",
    "pub struct Square(pub i32);",
    "impl Area for Square {",
    "    fn area(&self) -> i32 {",
    "        self.0 * self.0",
    "    }",
    "}",
    "pub fn total<T: Area>(items: &[T]) -> i32 {",
    "    items.iter().map(|item| item.area()).sum()",
    "}",
    "pub fn answer() -> i32 {",
    "    total(&[Square(3), Square(4)]) + Square(1).area()",
    "}",
    "",
  ].join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect((await import(join(dir, "lib.js"))).answer()).toBe(26);
}, 300_000);

// In a `#![no_std]` crate std's items are `core's` by path, `core::str::Chars`,
// which recognition took for no std item: num-traits' `chars()` stepped and
// asked `as_str()` was a raw pointer (docs/crate-corpus.md).
test("a #![no_std] crate's std items are std's, though core names them", async () => {
  const dir = fixture("no-std-paths");
  writeFileSync(join(dir, "lib.rs"), [
    "#![no_std]",
    "pub fn shift(src: &str) -> usize {",
    "    let mut chars = src.chars();",
    "    chars.next();",
    "    chars.as_str().len()",
    "}",
    "pub fn left(v: &[u8]) -> usize {",
    "    let mut it = v.iter();",
    "    it.next();",
    "    it.len()",
    "}",
    "pub fn taken() -> u32 {",
    "    let mut n = 7;",
    "    core::mem::take(&mut n) + n",
    "}",
    "",
  ].join("\n"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shift("héllo"), lib.left([1, 2, 3]), lib.taken()]).toEqual([5, 2, 7]);
}, 300_000);

// A library's trait method without a default whose entries take their
// caller's location: its manifest, of functions, can't say so to its
// consumers' calls through its dictionary, so it's refused (ADR 0427).
test("a library's tracked trait method without a default is refused", () => {
  const dir = fixture("library-tracked-method");
  writeFileSync(join(dir, "dep.rs"), "pub trait Tracked {\n    #[track_caller]\n    fn at(&self) -> u32;\n}\n\npub struct Here;\n\nimpl Tracked for Here {\n    fn at(&self) -> u32 {\n        std::panic::Location::caller().line()\n    }\n}\n");
  const refused = Bun.spawnSync([compiler, join(dir, "dep.rs"), "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("a library's trait method without a default that's given its caller's location through its dictionary");
  expect(refused.stderr.toString()).toContain("dep.rs:3:5");
}, 300_000);
