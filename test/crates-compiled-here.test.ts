// What a consumer compiles of its libraries itself (ADR 0914): what its
// calls give more of than a library's export takes, an impl or a function
// its library's JS doesn't export, a writer given more, and what may merge
// what it compares; each placed where its code reaches it, and refused
// where it reads its library's private state. Each crate's JS prints what
// the native program does.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, root, run } from "./support";
import { chain, contract, counting, pairs, printed } from "./crates";
import { node } from "./programs";

beforeAll(buildCompiler, 600_000);

// ADR 0914, amended: a consumer that grows a clone of a library's `Tree`
// calls the library's own `grow`: the `Box` optimized MIR copies to read
// through is no copy of what it holds, so `grow` copies nothing the
// consumer changes, and isn't compiled again.
test("a consumer that grows a library's boxed tree calls the library's method", () => {
  const out = chain(pairs, "tree", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain('import { Tree } from "../dep/lib.js";\nimport * as Dep from "../dep/lib.js";\n');
  // A read through a `Box` of what the library never changes copies none.
  expect(app).toContain("Dep.potSize(planter)");
}, 300_000);

// #1026: a library's writer given as a value, `quoted` to a `fn(&u32, &mut
// Formatter)`, is its import, as the crate's own is its name, not the arrow
// of its call as MIR's.
test("a library's writer given as a value is its import", () => {
  const out = chain(pairs, "library_writers_given_options", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("  return Joined.format(joined, quoted, options);\n");
}, 300_000);

// #1029: a closure a library's instance makes of a function rustc inlined
// into it, `joined`'s in `fold_with`, is placed where the instance is, not
// a panic of no module for it. #1051: the closures the crate gives that
// instance, which hold a value with a destructor, are its to drop, as the
// crate's own function's are (ADR 0470), and `joined`, whose closure holds
// them, is compiled here too, its closure carrying their drop.
test("a closure an instance makes of an inlined function is placed where the instance is", () => {
  const out = chain(pairs, "closure_made_by_an_inlined_function", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("function foldWith(it, f, init, g, dropF, dropG) {\n  const value$1 = joined(f, g, dropF, dropG);\n");
  expect(app).toContain("function joined(f, g, dropFnMut, dropFnMut$1) {\n  return $dropping(\n");
}, 300_000);

// #1052, ADR 0914, amended: a library that compares no function's identity
// may give one function where Rust makes two: an `Rc<dyn Fn>` of a cloned
// closure it doesn't count (ADR 0959), a closure that's the function it
// passes on to (ADRs 0605, 1043). Its manifest lists each such function, and
// each that calls one, its JS as it was; a consumer that compares an
// identity compiles its own of each it reaches, which keeps them apart, and
// one that compares none imports them.
test("a library's functions that may merge what its consumer compares are compiled by that consumer", () => {
  const out = chain(pairs, "library_identities_kept_apart", ["dep", "app"]);
  const dep = readFileSync(join(out, "dep", "lib.js"), "utf8");
  expect(dep).toContain("  const read = () => count.value;\n  const first = read;\n  const second = read;\n");
  expect(dep).toContain("  return { compute: getter };\n");
  expect(dep).toContain("  const first = bump;\n  const second = bump;\n");
  expect(dep).toContain("  return [ping, ping];\n");
  expect(contract(out).merges_identities).toHaveLength(7);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).toContain("  const first = { value: read, strong: 1, weak: 0 };\n  const second = { value: read, strong: 1, weak: 0 };\n");
  expect(app).toContain("\nfunction computed(getter) {\n  return { compute: (previous) => getter(previous) };\n}\n");
  expect(app).toContain("\nfunction doubled() {\n");
  expect(app).toContain("  const first = (by) => Counter.bump(counter, by);\n  const second = (by) => Counter.bump(counter, by);\n");
  expect(app).toContain('import { double, rcStableDeref, total } from "../dep/lib.js";');
  counting(out, "pub fn main() {\n    println!(\"{}\", (dep::doubled().compute)(Some(std::rc::Rc::new(2))));\n}\n");
  const imports = readFileSync(join(out, "counting", "lib.js"), "utf8");
  expect(imports).toContain("  const node = Dep.doubled();\n");
  expect(imports).not.toContain("function doubled");
  // One that reads its library's private state is refused, saying why it's
  // compiled here.
  const native = chain(join(root, "test", "crates", "refused"), "identity_merged_reads_private_state", ["dep", "app"], [
    "this crate compiles `dep::made` itself, as its library's JS has none it can call: one whose JS may give one function where Rust makes two, which this crate compares; but it reads `dep::MADE`",
  ]);
  expect(native).toBe("0 false\n");
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

// ADR 0914, amended (#483): what a consumer compiles of a library is written
// where its code reaches it. A call of a trait's generic default, asked of
// the library, is made through the dictionary the library exports of its
// own impl, `dogShout.shout(..)`: the default and the private function it
// calls, compiled here, are reached by nothing, and written nowhere. One the
// consumer's own impl's dictionary calls is written, and runs as natively.
test("a library's function a consumer compiles is written where the consumer's code reaches it", () => {
  const through = readFileSync(join(chain(pairs, "instance_through_dictionary", ["dep", "app"]), "app", "lib.js"), "utf8");
  expect(through).toContain("dogShout.shout(undefined, \"!\", asRef)");
  expect(through).not.toContain("function shout_shout");
  expect(through).not.toContain("function loud");
  const reached = readFileSync(join(chain(pairs, "unit_struct_of_instance", ["dep", "app"]), "app", "lib.js"), "utf8");
  expect(reached).toContain("lettersEngine.encode(letters, [120, 121, 122], asRef)");
  for (const name of ["engine_encode", "inner", "fill", "markerDebug_fmt"]) expect(reached).toContain(`\nfunction ${name}(`);
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
    // A `PartialEq`'s made by `$partialEq`, which gives std's `ne` (#962).
    expect(app).toContain(own.endsWith("PartialEq") ? `\nconst ${own} = $partialEq({` : `\nconst ${own} = {`);
  }
  expect(dep).not.toContain("export function digitPartialEq_eq");
  // Only what it uses of an impl, `Label`'s `fmt`, not its dictionary; and
  // as the consumer asks it, of the facts it takes and its writers' options.
  expect(app).not.toContain("labelDebug =");
  expect(app).not.toContain("spotsFirst =");
  // However the printer wraps it.
  expect(app).toMatch(/labelDebug_fmt\(\["x"\], \{\s*width: 9,\s*align: "Right",?\s*\}\)/);
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
  // One that copies what the library changes in place too, whose JS copies
  // it as Rust does, is imported (rd606).
  expect(dep).toContain("export function firstNotch(notches) {\n  const notch = { ...$index(notches, 0) };\n");
  expect(app).toContain("Dep.firstNotch(notches)");
  expect(app).not.toContain("\nfunction firstNotch(");
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
  const dep = "use std::cell::Cell;\n\nthread_local! {\n    static COUNT: Cell<u32> = const { Cell::new(0) };\n}\n\npub static NAME: &str = \"dep\";\n\npub fn discard<T>(value: T) -> u32 {\n    COUNT.set(COUNT.get() + 1);\n    drop(value);\n    COUNT.get()\n}\n\npub fn named<T>(value: T) -> &'static str {\n    drop(value);\n    NAME\n}\n\n#[cfg_attr(rust_js, rust_js::link_name = \"./stack.js#size\")]\npub fn size() -> u32 {\n    unreachable!()\n}\n\npub fn sized<T>(value: T) -> u32 {\n    let n = size();\n    drop(value);\n    n\n}\n\nunsafe extern \"Rust\" {\n    #[link_name = \"Infinity\"]\n    pub safe static INFINITE: f64;\n    #[link_name = \"./stack.js#limit\"]\n    pub safe static LIMIT: u32;\n}\n\npub fn unbounded<T>(value: T) -> bool {\n    drop(value);\n    INFINITE > 1.0\n}\n\npub fn limited<T>(value: T) -> u32 {\n    drop(value);\n    LIMIT\n}\n\nthread_local! {\n    pub static LAST: Cell<u32> = const { Cell::new(0) };\n}\n\npub fn remember<T>(tag: u32, value: T) {\n    LAST.set(tag);\n    drop(value);\n}\n\npub fn last() -> u32 {\n    LAST.get()\n}\n\npub struct Counted;\n\nimpl Drop for Counted {\n    fn drop(&mut self) {\n        COUNT.set(COUNT.get() + 1);\n    }\n}\n\npub struct Shown;\n\nimpl std::fmt::Display for Shown {\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n        COUNT.set(COUNT.get() + 1);\n        std::fmt::Display::fmt(&COUNT.get(), f)\n    }\n}\n";
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
  // And a writer given a placeholder's options its library's JS doesn't
  // take, which gives its `Formatter` on, compiled here to take them
  // (#1026), names what it's given.
  expect(() => app("println!(\"[{:>7}]\", dep::Shown);")).toThrow(
    "this crate compiles `<dep::Shown as std::fmt::Display>::fmt` itself, as its library's JS has none it can call: a writer given a placeholder's options, `{:>4}` or `{:#?}`, which its library's JS doesn't take; but it reads `dep::COUNT`, private to its library",
  );
}, 300_000);

// A consumer that reads a `Vec`'s capacity compiles its own of each function
// it reaches of a library that keeps none, so what they grow has std's
// capacity (#1180's (5)); of one that reads its library's private state,
// which a copy can't reach, it calls the library's own, and each `Vec` given
// back or lent has no known capacity: `capacity()` of it throws, naming
// #1180, and the program still compiles.
test("a library that keeps no capacity has its functions compiled by a consumer that reads one", () => {
  const out = chain(pairs, "library_vecs_grown_where_no_capacity_is_kept", ["dep", "app"]);
  expect(readFileSync(join(out, "app", "lib.js"), "utf8")).toContain("function squaresTwice(n) {");
  const thrown = join(root, "test", "crates", "thrown");
  expect(chain(thrown, "capacity_unknown_of_what_a_library_gives_back", ["dep", "app"], undefined, "(#1180)")).toBe("5\n8\n");
  expect(chain(thrown, "capacity_unknown_of_what_a_library_is_lent", ["dep", "app"], undefined, "(#1180)")).toBe("6\n8\n");
}, 300_000);
