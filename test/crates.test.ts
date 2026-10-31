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

// ADR 0914, amended: a consumer that grows a clone of a library's `Tree`
// calls the library's own `grow`: the `Box` optimized MIR copies to read
// through is no copy of what it holds, so `grow` copies nothing the
// consumer changes, and isn't compiled again.
test("a consumer that grows a library's boxed tree calls the library's method", () => {
  const out = chain(pairs, "tree", ["dep", "app"]);
  expect(readFileSync(join(out, "app", "lib.js"), "utf8")).toContain('import { Tree } from "../dep/lib.js";\nimport * as Dep from "../dep/lib.js";\n');
}, 300_000);

// ADR 0693, amended: a library's function, which nothing assigns, returned
// past a guard's drop is returned after its `finally`, as the crate's own is.
test("a library's function returned past a guard's drop is returned after its finally", () => {
  const out = chain(pairs, "returned_item", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("    depth = previous;\n  }\n  return Dep.noop;\n}");
  // One its `use` names, by that name.
  expect(app).toContain("    depth = previous;\n  }\n  return idle;\n}");
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
  expect(dep).toContain("export function sharedCell(value) {\n  return { value };\n}");
  expect(dep).not.toContain("strong");
  expect(readFileSync(join(out, "app", "lib.js"), "utf8")).not.toContain("strong");
  expect(contract(out).counted_rcs).toBeUndefined();
  expect(() => counting(out, "pub fn main() {\n    let context = dep::create_context(1);\n    println!(\"{}\", std::rc::Rc::strong_count(&context));\n}\n"))
    .toThrow("this crate counts an `Rc<dep::Context>`, whose counts it reads, whose last owner runs a destructor, or that may be one that does, but `dep`, a rust-js library that counts none of them, hands it one");
  const holder = chain(pairs, "rc_generic_holder", ["dep", "app"]);
  expect(contract(holder).counted_rcs).toEqual([{}]);
}, 300_000);

// #374, ADR 0960: a library checks its hooks' `RefCell<Vec<Rc<dyn Fn()>>>`,
// borrowed while they run; its consumer's `RefCell<Vec<Rc<Node>>>` isn't one
// that may be it, a `Vec`'s `dyn` no `Node`, so its borrows aren't checked.
test("a library's checked cell of a Vec of dyn Fns is no consumer's cell of a Vec of its own", () => {
  const out = chain(pairs, "cell_dyn_apart", ["dep", "app"]);
  // Checked: the held borrow counted, the momentary one checked alone (3901c815a).
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("  $borrowMut(hooks.list).push(hook);\n");
  expect(dep).toContain("  const borrow = $borrow(hooks.list, true);\n");
  expect(readFileSync(join(out, "app", "lib.js"), "utf8")).toContain("function push(siblings, child) {\n  siblings.push(child);\n}");
}, 300_000);

// ADR 0852: a library counts the `Rc`s of each type apart, and its contract
// says which. React's context, read by `use_context<T>` of any `T`, is the
// value itself beside a generic `Rc<Slot<T>>` it counts, as is its
// consumer's; the consumer counts the library's `Slot`s, and is refused
// counting its context, or a `String` a library that counts others hands it.
test("a library counts each type's Rcs apart, and its consumer as it does", () => {
  const out = chain(pairs, "rc_per_type", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("export function createContext(defaultValue) {\n  const context = {\n    value: defaultValue,\n    // These are circular\n    provider: null,\n    consumer: null,\n  };\n  context.provider = context;\n");
  expect(dep).toContain("export function useContext(context) {\n  const consumer = context.consumer;\n");
  expect(dep).toContain("  const slot = { value: { value }, strong: 1, weak: 0 };\n");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("  const context = Dep.createContext(1);\n  console.log(`${Dep.useContext(context)} ${Dep.useContext(context)}`);\n  const again = context;\n");
  expect(app).toContain("  again.value = 2;\n");
  expect(app).toContain("console.log(`${a.value.value[0]} ${a.strong}`);");
  expect(contract(out).counted_rcs).toEqual([{ of: expect.any(String), args: [{}] }]);
  expect(() => counting(out, "pub fn main() {\n    let context = dep::create_context(1);\n    println!(\"{}\", std::rc::Rc::strong_count(&context));\n}\n"))
    .toThrow("this crate counts an `Rc<dep::Context<i32>>`");
  const shared = chain(triples, "rc_shared", ["dep", "mid", "app"]);
  expect(() => counting(shared, "pub fn main() {\n    println!(\"{}\", std::rc::Rc::strong_count(&dep::tag(\"t\")));\n}\n"))
    .toThrow("this crate counts an `Rc<std::string::String>`");
}, 300_000);

// ADR 0477, amended (ep1b): a `From` impl's method nothing of its crate
// calls is written nowhere in its JS, React's reconciler's `EventPriority`
// of a lane; a consumer that calls it compiles its own (ADR 0914), and of a
// `#[repr(transparent)]` struct's field, by the library's contract, it's
// the value it's given (ADR 0584, amended, rd62).
test("a library's From impl nothing of its own calls is a consumer's to compile", () => {
  const out = chain(pairs, "from_impl_unused", ["dep", "app"]);
  expect(readFileSync(join(out, "dep", "lib.js"), "utf8")).not.toContain("_from");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("  const priority = 8;\n  const into = 32;\n");
  expect(app).not.toContain("_from");
}, 300_000);

// ADR 0852, amended (rd57): a library generic over another library's type,
// React's reconciler reading `react`'s context of any `T: Clone`, counts its
// `Rc`s as that library does, none, though its own consumers are unseen: a
// consumer giving it a context of what drops is refused, as is the library
// where its own call gives one.
test("a library counts another library's Rcs as that library does, refusing what would count one", () => {
  const out = chain(triples, "rc_context_read_generically", ["dep", "mid", "app"]);
  const mid = readFileSync(join(out, "mid", "lib.js"), "utf8");
  expect(mid).toContain("export function readContext(context) {\n  const value = context.currentValue;\n  return value;\n}");
  expect(mid).not.toContain("strong");
  expect(JSON.parse(readFileSync(join(out, "mid", "lib.manifest.json"), "utf8")).library.counted_rcs).toBeUndefined();
  const loud = "#[derive(Clone)]\npub struct Loud(pub u32);\n\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!(\"drop {}\", self.0);\n    }\n}\n\n";
  const read = "pub fn main() {\n    let context = std::rc::Rc::new(dep::Context {\n        current_value: std::cell::RefCell::new(Loud(1)),\n        provider: std::cell::Cell::new(std::rc::Weak::new()),\n    });\n    println!(\"{}\", mid::read_context(&context).0);\n}\n";
  expect(() => counting(out, `${loud}${read}`, ["dep", "mid"])).toThrow("this crate counts an `Rc<dep::Context<Loud>>`");
  const own = join(out, "mid_reading_loud.rs");
  const fresh = "pub fn read_fresh<T: Clone + 'static>(value: T) -> T {\n    let context = Rc::new(Context {\n        current_value: RefCell::new(value),\n        provider: std::cell::Cell::new(std::rc::Weak::new()),\n    });\n    read_context(&context)\n}\n\npub fn main() {\n    println!(\"{}\", read_fresh(Loud(1)).0);\n}\n";
  writeFileSync(own, `${readFileSync(join(triples, "rc_context_read_generically", "mid.rs"), "utf8")}\n${loud}${fresh}`);
  const dep = join(out, "dep");
  expect(() => run([compiler, own, "-o", join(out, "own", "lib.js"), "--library", "--manifest", join(out, "own", "lib.manifest.json"),
    "--dependency", join(dep, "lib.manifest.json"), "--", "--crate-name", "mid", "--extern", `dep=${join(dep, "libdep.rmeta")}`, "-L", `dependency=${dep}`]))
    .toThrow("this crate counts an `Rc<dep::Context<T>>`");
}, 300_000);

// ADRs 0289, amended, 0914: a library's generic function of a `T: Copy`
// takes the copies its own crate's calls give it, none here: its JS is
// the program's, which JS calls as its declarations say, as React's JS
// calls `createContext(defaultValue)`. Its consumer, which gives it a
// `Point` it changes, compiles its own from the library's MIR, given a
// copy where it gives one, and none of a number, its own copy.
test("a library's Copy bound takes a copy only where one is given, and JS calls it as declared", () => {
  const out = chain(pairs, "copy_given", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("export function createContext(defaultValue) {\n");
  expect(dep).toContain("    currentValue: defaultValue,\n");
  expect(dep).not.toContain("TCopy");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("\nfunction createContext(defaultValue, TCopy) {\n");
  expect(app).not.toMatch(/import \{[^}]*createContext/);
  // The copy it gives twice is one const at its top (ADR 0979).
  expect(app).toContain("  const pointCopy = { copy: (value) => ({ ...value }) };\n");
  expect(app).toContain("createContext({ x: 1 }, pointCopy)");
  expect(app).toContain("createContext(7)");
  expect(app).toContain("twice(8)");
  expect(app).toContain("\nfunction currentOr(_current, fallback, TCopy) {\n");
  const lib = JSON.stringify(join(out, "dep", "lib.js"));
  const called = `const dep = await import(${lib});
const object = { name: "default" };
const context = dep.createContext(object);
console.log(context.currentValue === object, context.currentValue2 === object);
const [a, b] = dep.twice(object);
console.log(a === object, b === object, dep.first([object]) === object, dep.first([]));`;
  expect(run([node ?? "node", "--input-type=module", "--eval", called])).toBe("true true\ntrue true true undefined\n");
}, 300_000);

// ADR 0914: a library's generic function takes the copies and drops its own
// crate's calls give it, as a program's does, so a crate's JS is the same
// built alone or as another's dependency: React's `createContext(
// defaultValue)` and `useState(initialState)`, which its JS calls as
// declared. Its consumer, which gives a `Point` it changes and a value with
// a destructor a panic drops, compiles its own of each from the library's
// MIR, and of the private function one calls, beside its own code: not
// exported, its JS against native Rust's. And (ADR 0914, amended) the trait
// impls it uses that the library's own code doesn't, which its JS hasn't,
// and the functions that copy what it changes in place, which the library
// never changes.
test("a library's generic function is its program's, alone or a dependency, and its consumer compiles what it gives more of", () => {
  const out = chain(pairs, "program_form", ["dep", "app"]);
  const alone = fixture("program-form-alone");
  run([compiler, join(pairs, "program_form", "dep.rs"), "-o", join(alone, "lib.js"), "--", "--crate-name", "dep", "--crate-type=lib"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toBe(readFileSync(join(alone, "lib.js"), "utf8"));
  expect(dep).toContain("export function createContext(defaultValue) {\n  return {\n    currentValue: defaultValue,\n");
  expect(dep).toContain("export function useState(initialState, dispatcher) {\n  dispatcher = resolveDispatcher(dispatcher);\n");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("\nfunction createContext(defaultValue, TCopy) {\n");
  expect(app).toContain("\nfunction useState(initialState, dispatcher, dropS) {\n");
  expect(app).toContain("\nfunction resolveDispatcher(dispatcher) {\n");
  expect(app).toContain("useState([2], undefined, loudDrop_drop)");
  expect(app).toContain("\nfunction pairOf(value, dropped, dropD) {\n");
  // One that takes the copy its consumer gives, as its own crate's calls give
  // one, is the library's.
  expect(app).toContain('import { digitDoubled, digitPartialEq, hourDisplay } from "../dep/lib.js";\nimport * as Dep from "../dep/lib.js";\n');
  // A writer of its it compiles, which calls what may fail, may fail; and an
  // impl's method its JS doesn't export by name is its dictionary's entry.
  expect(app).toContain("$writeTry(() => written([8], [7], loudDrop_drop))");
  expect(app).toContain("text += hourDisplay.fmt(hour);");
  expect(app).toContain("digitDoubled.doubled([4])");
  expect(app).toContain("  const pointCopy = { copy: (value) => ({ ...value }) };\n");
  expect(app).toContain("Dep.twice({ x: 3 }, pointCopy)");
  expect(app).not.toContain("\nfunction twice(");
  // The impls the library's own code never asks for, which its JS hasn't,
  // the consumer compiles where it gives them to the library's generic
  // code (ADR 0914, amended); one its program exports, `digitPartialEq`, it
  // imports. Each reads as the library's own would.
  for (const own of ["keyHash", "keyPartialEq", "digitHash", "tagHash", "tagDebug", "spotDebug"]) {
    expect(dep).not.toContain(own);
    expect(app).toContain(`\nconst ${own} = {`);
  }
  expect(dep).not.toContain("export function digitPartialEq_eq");
  // Only what it uses of an impl, `Label`'s `fmt`, not its dictionary; and
  // as the consumer asks it, of the facts it takes and its writers' options.
  expect(app).not.toContain("labelDebug =");
  expect(app).not.toContain("spotsFirst =");
  expect(app).toContain('labelDebug_fmt(["x"], { width: 9, align: "Right" })');
  expect(app).toContain("footprint([10], 4, loudDrop_drop)");
  expect(app).toContain("\nfunction digitHash_hash(digit, state, HHasher, dropH) {\n  HHasher.writeU32(state, digit[0] % 10);\n}\n");
  expect(app).toContain("\nfunction tagHash_hash(tag, state, HHasher, dropH) {\n");
  // The functions that copy what the consumer changes in place, which the
  // library never changes, are its own, and copy.
  expect(dep).toContain("export function firstSpot(spots) {\n  return $index(spots, 0);\n}");
  expect(app).toContain("\nfunction firstSpot(spots) {\n  return { ...$index(spots, 0) };\n}");
  expect(app).toContain("\nfunction firstPair(pairs) {\n  const value = $index(pairs, 0);\n  return [value[0], value[1]];\n}");
  // An impl's that copies, its own, called through the dictionary the
  // library exports, or by a library function that calls it.
  expect(app).toContain("\nfunction spotsFirst_first(spots) {\n  return { ...$index(spots[0], 0) };\n}");
  expect(app).toContain("\nfunction firstOf(spots) {\n  return spotsFirst_first(spots);\n}");
  // And two that call each other, above one that copies.
  expect(app).toContain("\nfunction inner(spots, n) {\n  return outer(spots, n);\n}");
  const lib = JSON.stringify(join(out, "dep", "lib.js"));
  const called = `const dep = await import(${lib});
const object = { name: "default" };
const context = dep.createContext(object);
console.log(context.currentValue === object, context.currentValue2 === object);
console.log(JSON.stringify(dep.useState(object, "client")));
try { dep.useState(object); } catch (error) { console.log(error.message); }`;
  expect(run([node ?? "node", "--input-type=module", "--eval", called])).toBe(
    'true true\n[{"name":"default"},"client"]\nInvalid hook call.\n',
  );
}, 300_000);

// ADR 0914: a library's function its consumer compiles itself has only
// what's public of its library to read: one that reads its library's
// private thread-local or static, of which its library's code has the one
// there is, is refused, naming it; a public one is imported. So is one that
// calls a binding its library names by a path relative to its own file, and
// an impl's method, a destructor, its JS doesn't export (ADR 0914, amended).
test("a library's function its consumer compiles is refused where it reads its library's private state", () => {
  const dir = fixture("instance-private-state");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  const dep = "use std::cell::Cell;\n\nthread_local! {\n    static COUNT: Cell<u32> = const { Cell::new(0) };\n}\n\npub static NAME: &str = \"dep\";\n\npub fn discard<T>(value: T) -> u32 {\n    COUNT.set(COUNT.get() + 1);\n    drop(value);\n    COUNT.get()\n}\n\npub fn named<T>(value: T) -> &'static str {\n    drop(value);\n    NAME\n}\n\n#[cfg_attr(rust_js, rust_js::link_name = \"./stack.js#size\")]\npub fn size() -> u32 {\n    unreachable!()\n}\n\npub fn sized<T>(value: T) -> u32 {\n    let n = size();\n    drop(value);\n    n\n}\n\nunsafe extern \"Rust\" {\n    #[link_name = \"Infinity\"]\n    pub safe static INFINITE: f64;\n    #[link_name = \"./stack.js#limit\"]\n    pub safe static LIMIT: u32;\n}\n\npub fn unbounded<T>(value: T) -> bool {\n    drop(value);\n    INFINITE > 1.0\n}\n\npub fn limited<T>(value: T) -> u32 {\n    drop(value);\n    LIMIT\n}\n\nthread_local! {\n    pub static LAST: Cell<u32> = const { Cell::new(0) };\n}\n\npub fn remember<T>(tag: u32, value: T) {\n    LAST.set(tag);\n    drop(value);\n}\n\npub fn last() -> u32 {\n    LAST.get()\n}\n\npub struct Counted;\n\nimpl Drop for Counted {\n    fn drop(&mut self) {\n        COUNT.set(COUNT.get() + 1);\n    }\n}\n";
  writeFileSync(join(dir, "dep.rs"), dep);
  writeFileSync(join(lib, "stack.js"), "export function size() {\n  return 1;\n}\n\nexport const limit = 2;\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  const app = (main: string) => {
    writeFileSync(join(dir, "app.rs"), `struct Loud(u32);\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!("dropped {}", self.0);\n    }\n}\npub fn main() {\n    ${main}\n}\n`);
    run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
      "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
    return printed(join(dir, "app.js"));
  };
  expect(() => app("println!(\"{}\", dep::discard(Loud(1)));")).toThrow(
    "this crate compiles `dep::discard` itself, as its library's JS has none it can call: one given a value to copy or drop that it takes nothing for, an impl its library's code never asks for, or one that copies what this crate changes in place; but it reads `dep::COUNT`, private to its library",
  );
  expect(app("println!(\"{}\", dep::named(Loud(2)));")).toBe("dropped 2\ndep\n");
  // A binding's static, a global's, which any crate's JS reads as it is,
  // as React's reconciler's `runWithFiberInDEV` reads `shared`'s `__DEV__`.
  expect(app("println!(\"{}\", dep::unbounded(Loud(5)));")).toBe("dropped 5\ntrue\n");
  // Nor one that sets its library's module variable, `export let last`,
  // which JS lets only its module set: an import can't be assigned.
  expect(() => app("dep::remember(4, Loud(7));\n    println!(\"{}\", dep::last());")).toThrow(
    "but it sets `dep::LAST`, its library's module variable, which JS lets only that module set",
  );
  // But not one its library binds to a module by a path from its own file.
  expect(() => app("println!(\"{}\", dep::limited(Loud(6)));")).toThrow(
    "but it reads `dep::LIMIT`, which its library binds to `./stack.js`, a module by a path relative to that library's own file",
  );
  // Nor one that calls a binding its library names by a path relative to
  // its own file, which isn't the consumer's.
  expect(() => app("println!(\"{}\", dep::sized(Loud(3)));")).toThrow(
    "but it calls `dep::size`, which its library binds to `./stack.js`, a module by a path relative to that library's own file",
  );
  // And an impl's method its library's JS doesn't export, a destructor it
  // runs where it drops one (ADR 0914, amended).
  expect(() => app("let _counted = dep::Counted;")).toThrow(
    "this crate compiles `<dep::Counted as std::ops::Drop>::drop` itself, as its library's JS has none it can call",
  );
}, 300_000);

// A library's generic function its consumer gives `()`s, which may look
// like `None`, is its export where it makes no `Some` of them, as React
// DOM's `runWithFiberInDEV(ancestor, () => { .. })` imports the reconciler's
// (ADR 0914, amended, gap rd75b); one that does, `wrapped(())`, is the
// consumer's own, boxing it. A closure it makes is of its type parameters
// but holds none of their values, so giving it to std makes none either,
// as React's five-argument form reads what it restores by `with_borrow`
// (gap rd85).
test("a library's generic function given what may look like None is its export where it makes no Some of it", () => {
  const out = chain(pairs, "generic_given_unit", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain(
    "import * as Dep from \"../dep/lib.js\";\n",
  );
  expect(app).not.toContain("function runWithFiber(");
  expect(app).not.toContain("function runWithFiberInDev(");
  expect(app).toContain("Dep.runWithFiberInDev(fiber, executeDispatch, \"click\", \"onClick\", \"button\")");
  expect(app).toContain("function wrapped(");
}, 300_000);

// ADR 0864, amended: a library's type equal only to itself, as its
// contract says, keys its consumer's `Set` by itself, and `==` of two is
// `===`, as React's reconciler adds a fiber to its `Transition`'s
// `_updatedFibers`; its `Eq` and `Hash` run nothing, so the borrow held
// while the set adds one isn't checked.
test("a library's type equal only to itself keys its consumer's Set by itself", () => {
  const out = chain(pairs, "identity_keys", ["dep", "app"]);
  expect(contract(out).identities).toEqual([expect.any(String)]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("function schedule(transition, fiber) {\n  if (transition.updatedFibers != null) {\n    transition.updatedFibers.add(fiber);\n");
  // A borrow held while `contains` runs: std's `Borrow` of a `&Fiber` is
  // its blanket impl, none of the library's, so it runs no code of `dep`'s.
  expect(app).toContain("  return transition.updatedFibers != null && transition.updatedFibers.has(fiber);\n");
  expect(app).toContain("${a === b} ${a === a}");
  expect(app).toContain("  const own = new Set();\n  own.add(b);\n");
  expect(app).not.toContain("$borrow");
}, 300_000);

// rd104 (#397): what a consumer compares that holds a library's type equal
// only to itself, React's `previous_instance != Some(instance)` of an
// `Option<&Option<Rc<Fiber>>>`, compares the fibers by `===`, never by
// `$eq`, which walks their fields and overflows the stack on a cycle; and
// a derived struct of the library's holding one, by its parts.
test("a consumer's == of what holds a library's type equal only to itself is never $eq", () => {
  const out = chain(pairs, "library_eq", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("function sameInstance(previous, instance) {\n  const right = $some(instance);\n  return previous == null || right == null\n    ? previous == right\n    : $someValue(previous) == $someValue(right);\n}");
  expect(app).toMatch(/dispatch\.instance == (right\$\d+)\.instance && dispatch\.name === \1\.name/);
  // A local `Option<&Option<Rc<Fiber>>>` made and compared where it's held
  // is its payload, the library's nullable field's `None` `null`, as
  // upstream's `previousInstance = instance` (ADR 1017).
  expect(app).toContain("    if (previousInstance !== instance && stopped) {\n");
  expect(app).toContain("    previousInstance = instance;\n");
  expect(app.match(/\$eq\([^)]*\)/g)).toEqual(["$eq(p, q)", "$eq(p, r)", "$eq(p, q)"]);
  // The library's `Update`, whose `PartialEq` alone is `ptr::eq`, its
  // contract's `eq_by_address`: `===`, never its `eq` compiled again.
  const { identities, eq_by_address } = contract(out);
  expect([identities.length, eq_by_address.length, identities.every((id: string) => eq_by_address.includes(id))]).toEqual([1, 2, true]);
  expect(app).not.toContain("updatePartialEq_eq");
  expect(app).not.toContain("function fiberPartialEq_eq");
}, 300_000);

// #398: a key holding a library's type equal only to itself, a derived
// struct of the library's or the consumer's tuple, keeps each fiber by its
// `$id`, the library's map as its consumer's, never by what's in it.
test("a key holding a library's type equal only to itself finds it by itself", () => {
  const out = chain(pairs, "identity_inside_keys", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  const parts = "$keyMap((key) => ({ ...key, fiber: $id(key.fiber) }))";
  expect(dep).toContain(`new (${parts})()`);
  expect(app).toContain(`new (${parts})()`);
  expect(app).toContain("new ($keySet((key) => [$id(key[0]), key[1]]))(");
  expect(dep + app).not.toContain("$KeyMap");
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
  expect(dep).toContain("  const current = {\n    types: undefined,\n    seen: new Set(),\n  };\n  current.types = prev?.types;\n");
  expect(dep).toContain("    transition.types = [ty];\n  } else if (!types.some((t) => t === ty)) {\n    types.push(ty);\n");
  expect(dep).toContain("    return a === b;\n");
  expect(dep).toContain("    const count = transition.seen.size;\n    transition.seen.clear();\n");
  expect(dep).not.toContain("$borrow");
  // A transparent struct of a `RefCell` of an array, the array, on both
  // sides, as JS makes it (ADRs 0584, 0863).
  expect(dep).toContain("  queue.push(value);\n  return queue.length;\n");
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  // What it changes, a `Vec` through a `Ref` a library's function borrows
  // for a moment, is no `Vec` that function holds: it imports the library's
  // `add_type`, not a copy compiled of its own (ADR 0914, amended).
  expect(app).toContain('import * as Dep from "../dep/lib.js";\n');
  expect(app).toContain('  Dep.addType(outer, "a");\n');
  expect(app).not.toContain("function addType(");
  expect(app).toContain("  const queue = [\"q\"];\n");
  expect(app).toContain("  queue.push(\"s\");\n");
  expect(app).toContain("  const replaced = inner.types;\n  inner.types = undefined;\n");
  expect(app).not.toContain("$borrow");
  // A consumer that only makes the library's struct makes its plain field
  // the value it holds, as the library reads it.
  counting(out, "use std::cell::RefCell;\n\npub fn main() {\n    let made = dep::Transition {\n        types: RefCell::new(None),\n        seen: None,\n    };\n    dep::add_type(&made, \"x\");\n    println!(\"{:?}\", dep::types(&made));\n}\n");
  expect(readFileSync(join(out, "counting", "lib.js"), "utf8")).toContain("  const made = {\n    types: undefined,\n  };\n");
  const facts = contract(out).ref_cells;
  expect([facts.unchecked.length, facts.counted, facts.boxed]).toEqual([3, undefined, undefined]);
  expect(contract(out).fields.map((field: { facts: string[] }) => field.facts)).toEqual([["quiet"]]);
  // A field it lists, borrowed for a moment, is the property its consumer
  // reads, whatever the consumer's own borrows of its type count.
  counting(out, "use std::cell::RefCell;\nuse std::rc::Rc;\n\npub struct Mine {\n    types: RefCell<Option<Rc<RefCell<Vec<String>>>>>,\n}\n\nfn count(mine: &Mine) -> usize {\n    mine.types.borrow().as_ref().map_or(0, |types| types.borrow().len())\n}\n\nfn maybe(mine: &Mine, go: bool) -> usize {\n    if go {\n        count(mine)\n    } else {\n        0\n    }\n}\n\nfn twice(mine: &Mine) -> usize {\n    let held = mine.types.borrow_mut();\n    maybe(mine, false) + held.as_ref().map_or(0, |types| types.borrow().len())\n}\n\npub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let mine = Mine { types: RefCell::new(None) };\n    let n = t.types.borrow().as_ref().map_or(0, |types| types.borrow().len());\n    println!(\"{n} {}\", twice(&mine));\n}\n");
  const reads = readFileSync(join(out, "counting", "lib.js"), "utf8");
  expect([reads.includes("$borrowMut($fieldCell(mine, \"types\"), true)"), reads.includes("$fieldCell(t")]).toEqual([true, false]);
  expect(() => counting(out, "pub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    let held = types.borrow_mut();\n    dep::add_type(&t, \"b\");\n    println!(\"{}\", held.len());\n}\n"))
    .toThrow("is held while code rust-js can't see runs, which may be `dep`'s");
  expect(() => counting(out, "pub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    let n = types.borrow().len() + dep::types(&t).len();\n    println!(\"{n}\");\n}\n"))
    .toThrow("is held while code rust-js can't see runs, which may be `dep`'s");
  expect(() => counting(out, "use std::cell::RefCell;\nuse std::rc::Rc;\n\nfn some(types: &RefCell<Option<Rc<RefCell<Vec<String>>>>>) -> bool {\n    types.borrow().is_some()\n}\n\npub fn main() {\n    let t = dep::start(None);\n    println!(\"{}\", some(&t.types));\n}\n"))
    .toThrow("`dep`, a rust-js library, borrows `dep::Transition::types` unchecked");
  expect(() => counting(out, "use std::cell::RefCell;\n\nfn get<T: Clone>(cell: &RefCell<T>) -> T {\n    cell.borrow().clone()\n}\n\npub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    println!(\"{:?}\", get(&types));\n}\n"))
    .toThrow("as a `{ value }`");
  // A borrow held while the consumer borrows a cell that may be it runs
  // no code rust-js can't see, a cell's borrow running only its own check:
  // counted, not refused (ADR 0863, amended).
  counting(out, "use std::cell::RefCell;\nuse std::rc::Rc;\n\nfn copy(from: &Rc<RefCell<Vec<String>>>, to: &Rc<RefCell<Vec<String>>>) {\n    for t in from.borrow().iter() {\n        to.borrow_mut().push(t.clone());\n    }\n}\n\npub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    let mine = Rc::new(RefCell::new(Vec::new()));\n    copy(&types, &mine);\n    println!(\"{}\", mine.borrow().len());\n}\n");
  // The momentary one checked alone, as 3901c815a makes it, the other counted.
  const copied = readFileSync(join(out, "counting", "lib.js"), "utf8");
  expect(copied).toContain("const borrow = $borrow(from, true);");
  expect(copied).toContain("$borrowMut(to).push(t);");
  // React's scheduleGesture, a type pushed only where it isn't there,
  // which it is where the two are one cell: neither borrow is counted.
  counting(out, "use std::cell::RefCell;\nuse std::rc::Rc;\n\nfn merge(types: &Rc<RefCell<Vec<String>>>, into: &Rc<RefCell<Vec<String>>>) {\n    for t in types.borrow().iter() {\n        if !into.borrow().contains(t) {\n            into.borrow_mut().push(t.clone());\n        }\n    }\n}\n\npub fn main() {\n    let t = dep::start(None);\n    dep::add_type(&t, \"a\");\n    let types = t.types.borrow().clone().unwrap();\n    merge(&types, &types);\n    println!(\"{}\", types.borrow().len());\n}\n");
  expect(readFileSync(join(out, "counting", "lib.js"), "utf8")).toContain("  for (const t of types) {\n    if (!into.includes(t)) {\n      into.push(t);\n");
  // One whose generic code has a `RefCell<T>` of any `T` boxes each, and
  // one that holds a borrow while what it's given runs counts it: its
  // consumer boxes and checks them as it does.
  const counted = chain(pairs, "ref_cells_counted", ["dep", "app"]);
  expect([contract(counted).ref_cells.boxed, contract(counted).ref_cells.counted.length]).toEqual([[{}], 1]);
  const consumer = readFileSync(join(counted, "app", "lib.js"), "utf8");
  // Each momentary borrow checked alone (3901c815a), the library's held one
  // found by the closure's.
  expect(consumer).toContain("    $borrowMut(list.value).value.push(3);\n");
  expect(consumer).toContain("        $borrowMut(list.value).value.push(n);\n");
}, 300_000);

// ADR 0863, amended: a thread-local is a place of its own, as a struct's
// field is. A crate whose generic code makes a `RefCell<T>` of any value,
// `{ value }`, beside thread-locals of the `RefCell` type its library shares
// as the array, as React's `ReactFiberStack` has beside `shared`'s
// `thrownErrors`, isn't refused: each thread-local is its module's variable,
// the array, and only the generic value is a `{ value }`.
test("a thread-local's RefCell is its own, whatever generic code's RefCell<T> is", () => {
  const out = chain(pairs, "thread_local_cell", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("const valueStack = [];\nconst names = [\"first\"];\n");
  expect(app).toContain("export function cellOf(value) {\n  return { value };\n}");
  expect(app).toContain("  valueStack[$at(valueStack, 0)] = undefined;\n");
}, 300_000);

// ADR 0037, amended (#376), and ADR 1006, amended: a thread-local whose
// initializer reads a library's flag, which its contract makes as its
// module loads, is made as its own module loads, React's
// `supportsUserTiming`; and so is one of the count the library sets.
test("a thread-local of a library's flag is made as its module loads", () => {
  const out = chain(pairs, "thread_local_reads_library", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  // Through the library's namespace, as their paths name them (ADR 1018).
  expect(app).toContain("const { ENABLE_PROFILER_TIMER: SUPPORTS_USER_TIMING, count: SEEN } = Dep;\n");
}, 300_000);

// ADR 0330, amended (React port gap rd56): a library's struct field whose
// fn pointers may only be its own Rust functions, React's `Children`, is one
// its contract says so of, and a consumer's call through it leaves out a
// trailing `undefined`, as of the crate's own: `Children.forEach(children,
// fn)`, and a unit struct's `undefined` it's given last, which the function
// never reads, `Dep.children.count("abc")` of `count(Some("abc"), Caller)`.
// One a consumer may give it keeps that, but a trailing `None` is no
// argument of either, `hooks.onEach(1)` (ADR 0443, amended). `two crates:
// fn_pointer_of_a_library` runs it against native Rust.
test("a library's field of its own functions is called as the crate's own", () => {
  const out = chain(pairs, "fn_pointer_of_a_library", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain('  Dep.children.forEach("ab", (i) => {\n');
  expect(app).toContain('${Dep.children.count("abc")}');
  expect(app).toContain("hooks.onEach(1)}");
  expect(contract(out).fields.map((field: { facts: string[] }) => field.facts)).toEqual([["rust_functions"], ["rust_functions"]]);
}, 300_000);

/** The library contract of `out`'s `dep`. */
function contract(out: string) {
  return JSON.parse(readFileSync(join(out, "dep", "lib.manifest.json"), "utf8")).library;
}

/** `source`, a crate using `out`'s `dep`, or the crates `uses` names, compiled. */
function counting(out: string, source: string, uses = ["dep"]) {
  const file = join(out, "counting.rs");
  writeFileSync(file, source);
  run([compiler, file, "-o", join(out, "counting", "lib.js"), ...uses.flatMap((u) => ["--dependency", join(out, u, "lib.manifest.json")]),
    "--", "--crate-name", "counting", ...uses.flatMap((u) => ["--extern", `${u}=${join(out, u, `lib${u}.rmeta`)}`, "-L", `dependency=${join(out, u)}`])]);
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

// A generic function is given a drop for a type parameter only where its
// body drops one, itself or through what it calls (ADR 0300): one that
// moves its value first, or passes it on to one that does, takes none. A
// library's takes those its own crate's calls give it, none here, so its JS
// is the program's; its consumer, which gives values with destructors,
// compiles itself those that may drop one, from the library's MIR, and calls
// the library's others (ADR 0914). `two crates: drops_by_need` runs it
// against native Rust.
test("a library's generic function takes the drops its body uses", () => {
  const out = chain(pairs, "drops_by_need", ["dep", "app"]);
  const js = readFileSync(join(out, "dep", "lib.js"), "utf8");
  const signatures = js.split("\n").filter((line) => line.startsWith("export function"));
  expect(signatures).toEqual([
    "export function keep(value) {",
    "export function pick(a, b) {",
    "export function relay(value) {",
    "export function discard(value) {",
    "export function apply(f, a) {",
    "export function keepBy(value) {",
  ]);
  const items = JSON.parse(readFileSync(join(out, "dep", "lib.manifest.json"), "utf8")).library.items;
  // Each type parameter a function takes a drop of, by its record in `params`.
  type Item = { export: string; params?: { index: number; marks?: string[] }[] };
  const dropsOf = (item: Item) =>
    (item.params ?? []).filter((param) => param.marks?.includes("drop")).map((param) => param.index);
  expect(Object.fromEntries(items.map((item: Item) => [item.export, dropsOf(item)]))).toEqual({
    keep: [], pick: [], relay: [], discard: [], apply: [], keepBy: [],
  });
  // What each call gives: only the drops its callee takes, and a function
  // given as a value is itself.
  expect(js).toContain("return keep(value);");
  expect(js).toContain("pick(undefined, value);");
  expect(js).toContain("return apply(keep, value);");
  // The consumer's own of those that drop what it gives, and the library's
  // of those that drop none.
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain('import * as Dep from "../dep/lib.js";');
  expect(app).toContain("\nfunction pick(a, b, dropB) {\n  dropB?.(b);\n  return a;\n}");
  expect(app).toContain("\nfunction discard(value, dropT) {\n  pick(undefined, value, dropT);\n}");
  // A closure's call drops what it's given by its own code, so `apply` and
  // `keep_by`, which only call one, drop nothing: the library's, by its
  // namespace, not compiled again here.
  expect(app).not.toMatch(/^function (apply|keepBy)\(/m);
  expect(app).toContain("Dep.apply(");
  expect(app).toContain("Dep.keepBy(");
}, 300_000);

// A library's generic function its own crate gives a value with a
// destructor takes a drop (ADR 0914), and its manifest says so: a consumer
// calls it where it is, the library's, with its own drop, not a copy it
// compiles itself.
test("a library's generic function that takes a drop is called with the consumer's", () => {
  const dir = fixture("library-function-drop");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  writeFileSync(join(dir, "dep.rs"), `pub fn second<T>(a: T, b: T) -> T {
    let _first = a;
    println!("second");
    b
}
pub struct Own(pub u32);
impl Drop for Own {
    fn drop(&mut self) {
        println!("own {}", self.0);
    }
}
pub fn own() -> u32 {
    second(Own(1), Own(2)).0
}
`);
  writeFileSync(join(dir, "app.rs"), `struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}
pub fn main() {
    println!("{}", dep::second(Loud(2), Loud(3)).0);
    println!("after");
}
`);
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  expect(readFileSync(join(lib, "lib.js"), "utf8")).toContain("export function second(a, b, dropT) {");
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(lib, "libdep.rmeta")}`]);
  const app = readFileSync(join(dir, "app.js"), "utf8");
  expect(app).toContain('import * as Dep from "./dep/lib.js";');
  expect(app).not.toContain("function second(");
  expect(printed(join(dir, "app.js"))).toBe("second\ndropped 2\n3\ndropped 3\nafter\n");
}, 300_000);

// #378: a consumer compiling a library's generic code from its optimized
// MIR, where a box is read through the pointer rustc makes of it, compiled
// `vec![t]`'s (`generic_boxed_drop`), its drop flags settled (#381), and
// refuses what it can't yet, a generic method given a destructor or a
// value moved out of its box, as an error: it never panics.
test("a consumer refuses a library's generic code it can't compile yet, and never crashes", () => {
  const out = chain(pairs, "generic_boxed_drop", ["dep", "app"]);
  // Its optimized MIR's drop flags settled as rust-js's own: none of `t`
  // after `vec![t]` took it, nor a `try` of a `catch` it emptied (#381).
  const copied = readFileSync(join(out, "app", "lib.js"), "utf8");
  // Each item dropped, the rest too after one that panics (a1a886ee4).
  expect(copied).toContain("  const result = v.length;\n  $dropItems(v, (item) => {\n    dropT?.(item);\n  });\n  return result;\n}");
  expect(copied).toContain("  if (keep) {\n    return held;\n  }\n  try {\n    check();\n  } finally {\n    dropT?.(held);\n  }\n}");
  const dir = fixture("library-boxed-refused");
  writeFileSync(join(dir, "dep.rs"), "pub fn unboxed<T>(t: T) -> T {\n    let boxed = Box::new(t);\n    *boxed\n}\n");
  const lib = join(dir, "dep");
  mkdirSync(lib, { recursive: true });
  run([compiler, join(dir, "dep.rs"), "-o", join(lib, "lib.js"), "--library", "--manifest", join(lib, "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(lib, "libdep.rmeta")}`]);
  const loud = "struct Loud(u32);\nimpl Drop for Loud {\n    fn drop(&mut self) {\n        println!(\"dropped {}\", self.0);\n    }\n}\n";
  const refused = (app: string, against: string, why: string) => {
    writeFileSync(join(dir, "app.rs"), `${loud}pub fn main() {\n${app}}\n`);
    const compile = [compiler, join(dir, "app.rs"), "-o", join(dir, "app.js"), "--dependency", join(against, "lib.manifest.json"),
      "--", "--crate-name", "app", "--crate-type=lib", "--extern", `dep=${join(against, "libdep.rmeta")}`];
    let error = "";
    try {
      run(compile);
    } catch (e) {
      error = String(e);
    }
    expect(error).toContain(why);
    expect(error).not.toContain("panicked");
  };
  refused("    let mut bag = dep::Bag { items: vec![Loud(1)] };\n    bag.clear();\n", join(out, "dep"),
    "giving `dep::Bag::<T>::clear`'s `T` a type with a destructor, where it takes none");
  refused("    println!(\"{}\", dep::unboxed(Loud(1)).0);\n", lib, "rust-js does not support");
}, 300_000);

// #394: a consumer's copy of React's `runWithFiberInDEV`, compiled from its
// optimized MIR, tests each argument's drop flag in its cleanup: settled as
// rust-js's own (#381), none is tested, where each `catch` nested them 2^k
// ways, about 450 lines; it drops what it's given where Rust does.
test("a consumer's copy of a library's generic function settles its cleanup's drop flags", () => {
  const out = chain(pairs, "generic_flags_given", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  const copy = app.slice(app.indexOf("function runWithFiberInDev("), app.indexOf("export function main("));
  expect(copy).not.toContain("if (");
  expect(copy.split("\n").length).toBeLessThan(30);
  // Its thread-local read as its library's `export let`, which can't
  // throw: no `try` of it, `const previous = current;` (#408).
  expect(copy).toContain("  const previous = current;\n  try {\n    setCurrent(fiber);");
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
  expect(js).toContain("export function first(items) {");
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
