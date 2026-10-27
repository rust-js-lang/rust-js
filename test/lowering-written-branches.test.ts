// What the compiler writes of an `if` the program wrote, as it wrote it:
// its test as written, not negated, its branches in the order written, an
// `else if` chain kept a chain past an arm that leaves, and a `continue`
// the program wrote where it wrote it, with its comment. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** `rust` compiled in a fixture of `name`, against the js crate: its JS,
 * and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; lib: Record<string, (...args: never[]) => unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// React's describeObjectForErrorMessage (shared/ReactSerializationErrors.js):
// `if (__DEV__ && jsxChildrenParents.has(objectOrArray)) { .. } else { ..
// }`, each branch a loop. MIR jumps from the `&&`'s tests to the `else`, so
// its `then` is the block's; the `if` is as written, not `if (!__DEV__ ||
// !jsxChildrenParents.has(..)) { .. } else { .. }`.
test("an if of an && keeps its test and its branches' order, each a loop", async () => {
  const { js, lib } = await compiled("written-and-branches", `use std::collections::HashSet;

pub const DEV: bool = true;

pub fn described(parents: &HashSet<u32>, key: u32, items: &[u32]) -> String {
    let mut str;
    if DEV && parents.contains(&key) {
        // Print JSX Children
        str = "<".to_string();
        for &item in items {
            let substr = if item > 3 { format!("{{{item}}}") } else { item.to_string() };
            if substr.len() < 15 {
                str += &substr;
            } else {
                str += "{...}";
            }
        }
        str += ">";
    } else {
        // Print Array
        str = "[".to_string();
        for &item in items {
            str += &item.to_string();
        }
        str += "]";
    }
    str
}
`);
  expect(js).toContain('  if (DEV && parents.has(key)) {\n    // Print JSX Children\n    str = "<";\n');
  expect(js).toContain('  } else {\n    // Print Array\n    str = "[";\n');
  expect(js).not.toContain("!DEV");
  const both = new Set([1]);
  expect([lib.described(both as never, 1 as never, [1, 5] as never), lib.described(both as never, 2 as never, [1, 5] as never)]).toEqual(["<1{5}>", "[15]"]);
});

// The same function's `else if` chain: an element, a client reference that
// returns, JSX, an object. An arm that leaves keeps the chain's `else`
// where an arm before it doesn't leave (ADR 0237, amended), as ESLint's
// `no-else-return` allows; and so does a chain whose arms each leave up to
// it, where statements follow it (ADR 0879, amended).
test("an else-if chain keeps its else past an arm that returns", async () => {
  const { js, lib } = await compiled("written-else-if-chains", `use std::collections::HashSet;

pub const DEV: bool = true;

pub fn described(parents: &HashSet<u32>, key: u32) -> String {
    let mut str;
    if key == 1 {
        str = "element".to_string();
    } else if key == 2 {
        return "client".to_string();
    } else if DEV && parents.contains(&key) {
        // Print JSX
        str = "<jsx>".to_string();
    } else {
        // Print Object
        str = "{".to_string();
        str += "}";
    }
    str
}

pub fn guarded(key: u32) -> u32 {
    if key == 1 {
        return 10;
    } else if key == 2 {
        return 20;
    } else if key > 5 {
        println!("big");
    }
    key
}
`);
  expect(js).toContain(
    '  if (key === 1) {\n    str = "element";\n  } else if (key === 2) {\n    return "client";\n  } else if (DEV && parents.has(key)) {\n    // Print JSX\n',
  );
  expect(js).toContain('  } else {\n    // Print Object\n    str = "{";\n');
  // Each arm up to the last leaves, and the function goes on after the
  // chain: the chain as written (ADR 0879, amended).
  expect(js).toContain("  if (key === 1) {\n    return 10;\n  } else if (key === 2) {\n    return 20;\n  } else if (key > 5) {\n");
  const parents = new Set([3]);
  expect([1, 2, 3, 4].map((key) => lib.described(parents as never, key as never))).toEqual(["element", "client", "<jsx>", "{}"]);
  expect([1, 2, 3].map((key) => lib.guarded(key as never))).toEqual([10, 20, 3]);
});

// React's isSimpleObject: `if (!descriptor.enumerable) { if ((name ===
// 'key' || name === 'ref') && typeof descriptor.get === 'function') { //
// ..\n continue; } return false; }`. The `continue` the program wrote is
// where it wrote it, its comment before it, and its `if`'s test as written;
// not `if ((name !== "key" && name !== "ref") || ..) { return false; }`, a
// `let … else`'s guard, with the comment gone.
test("a continue the program wrote keeps its if, its test and its comment", async () => {
  const { js, lib } = await compiled("written-continues", `use std::collections::HashMap;

pub struct Descriptor {
    pub enumerable: Option<bool>,
    pub get: Option<u32>,
}

pub fn is_simple(names: &[String], descriptors: &HashMap<String, Descriptor>) -> bool {
    for name in names {
        let Some(descriptor) = descriptors.get(name) else {
            return false;
        };
        if !descriptor.enumerable.unwrap_or_default() {
            if (name == "key" || name == "ref") && descriptor.get.is_some() {
                // React adds key and ref getters to props objects to issue warnings.
                // Those getters will not be transferred to the client, but that's ok,
                // so we'll special case them.
                continue;
            }
            return false;
        }
    }
    true
}

pub fn simple(names: Vec<String>, enumerable: Vec<bool>, getters: Vec<bool>) -> bool {
    let descriptors = (names.iter().cloned())
        .zip(enumerable.into_iter().zip(getters))
        .map(|(name, (enumerable, getter))| {
            let descriptor = Descriptor { enumerable: Some(enumerable), get: getter.then_some(1) };
            (name, descriptor)
        })
        .collect();
    is_simple(&names, &descriptors)
}
`);
  expect(js).toContain(
    '      if ((name === "key" || name === "ref") && descriptor.get != null) {\n        // React adds key and ref getters to props objects to issue warnings.\n',
  );
  expect(js).toContain("        // so we'll special case them.\n        continue;\n      }\n      return false;\n    }\n");
  const simple = (names: string[], enumerable: boolean[], getters: boolean[]) => lib.simple(names as never, enumerable as never, getters as never);
  expect([
    simple(["a", "key"], [true, false], [false, true]),
    simple(["a", "key"], [true, false], [false, false]),
    simple(["ref", "b"], [false, false], [true, true]),
    simple(["a"], [true], [false]),
  ]).toEqual([true, false, false, true]);
});

// isSimpleObject itself, of the js crate's reflection: its `let … else`
// guard, then its `if` of a `continue`, which ends the loop's body. Its
// `else`, MIR's `continue` the loop's turn ends with anyway, is no `else`
// (ADR 0684), not `} else { continue; }`, left past the `return` as a
// `continue;` the program didn't write.
test("a loop's body that ends in an if of a continue has no continue after it", async () => {
  const { js, lib } = await compiled("written-continues-js", `use js::Unknown;

pub fn is_simple_object(object: &Unknown) -> bool {
    let names = js::object::get_own_property_names(object);
    for name in &names {
        let Some(descriptor) = js::object::get_own_property_descriptor(object, name) else {
            return false;
        };
        if !descriptor.enumerable.unwrap_or_default() {
            if (name == "key" || name == "ref") && descriptor.get.is_some() {
                // React adds key and ref getters to props objects to issue warnings.
                continue;
            }
            return false;
        }
    }
    true
}
`);
  expect(js).toContain(
    '      if ((name === "key" || name === "ref") && descriptor.get) {\n        // React adds key and ref getters to props objects to issue warnings.\n        continue;\n      }\n      return false;\n    }\n  }\n  return true;\n',
  );
  const props = Object.defineProperty({ a: 1 }, "key", { get: () => 1, enumerable: false });
  const hidden = Object.defineProperty({}, "b", { value: 1, enumerable: false });
  expect([lib.isSimpleObject(props as never), lib.isSimpleObject(hidden as never), lib.isSimpleObject({ a: 1 } as never)]).toEqual([true, false, true]);
});

// ADR 0701: of `if a || v[i] != w[j] { return 1; }`, an index in bounds
// MIR reads into a temporary between the tests, the read is the test's
// alone where nothing after it reads it (`unread_bound`, as `and_tests`
// leaves one): `if (j === 0 || v[i] !== w[j])`, no `const value = v[i];`
// left after it.
test("a read a test takes in place is gone where nothing else reads it", async () => {
  const { js, lib } = await compiled(
    "unread-bound-read",
    `pub fn differs(v: &[i32], w: &[i32], i: usize, j: usize) -> usize {
    if i < v.len() && j < w.len() {
        if j == 0 || v[i] != w[j] {
            return 1;
        }
    }
    0
}
`,
  );
  expect(js).toContain("    if (j === 0 || v[i] !== w[j]) {\n      return 1;\n    }\n  }\n  return 0;\n");
  expect(js).not.toContain("const value");
  const differs = lib.differs as unknown as (v: number[], w: number[], i: number, j: number) => number;
  expect([differs([1, 2], [1, 2], 1, 1), differs([1, 2], [1, 3], 1, 1), differs([1], [1], 0, 0), differs([], [1], 0, 0)]).toEqual([0, 1, 1, 0]);
});

// ADR 0744: an `if` of two values, the second the value the first tests,
// `if s.is_empty() { "Object" } else { &s }`, is `s || "Object"`, not `!s ?
// "Object" : s`, reading `s` once. React's `addValueToProperties`
// (shared/ReactPerformanceTrackProperties.js) writes `insertedEntry[1] ||
// 'Object'`.
test("a value or what it falls back to is ||", async () => {
  const { js, lib } = await compiled("value-or-fallback", `pub fn wrapped(properties: &mut [(String, String)], idx: usize) {
    let inserted_entry = &mut properties[idx];
    inserted_entry.1 = "Promise<".to_string()
        + if inserted_entry.1.is_empty() {
            "Object"
        } else {
            &inserted_entry.1
        }
        + ">";
}
`);
  expect(js).toContain('insertedEntry[1] = "Promise<" + (insertedEntry[1] || "Object") + ">";');
  const properties: [string, string][] = [["a", ""], ["b", "x"]];
  (lib.wrapped as (p: [string, string][], i: number) => void)(properties, 0);
  (lib.wrapped as (p: [string, string][], i: number) => void)(properties, 1);
  expect(properties).toEqual([["a", "Promise<Object>"], ["b", "Promise<x>"]]);
});

// ADR 0842: React's lazy() reads `ioInfo.value` once where `__DEV__` holds,
// `if (__DEV__) { const value = ioInfo.value; if (value != null) { .. } }`;
// clippy's `collapsible_if` has the Rust write a let chain, `if debug &&
// let Some(value) = info.value.get()`, which read `if (debug &&
// info.value) { const value = info.value; .. }`, the place twice. A chain
// whose first test is a bool of its own, and whose binding the body reads
// or its tests read again, is nested as written; one that takes apart what
// its first test tested, `if (value && value[0] < 10)`, is joined, as
// `examples/options.rs` has it.
test("a let chain after a test of its own reads its place once", async () => {
  const { js, lib } = await compiled("chain-place-once", `use std::cell::Cell;

use js::Unknown;

pub struct Info {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Cell<Option<&'static Unknown>>,
}

pub fn settle(info: &Info, debug: bool) {
    if debug && let Some(value) = info.value.get() {
        js::set(value, "status", "fulfilled");
    }
}

pub struct Counts {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub count: Cell<Option<u32>>,
}

pub fn many(counts: &Counts, debug: bool) -> bool {
    if debug && let Some(count) = counts.count.get() && count > 3 {
        return true;
    }
    false
}
`);
  expect(js).toContain("  if (debug) {\n    const value = info.value;\n    if (value != null) {\n      value.status = \"fulfilled\";\n    }\n  }\n");
  expect(js).toContain("  if (debug) {\n    const count = counts.count;\n    if (count != null && count > 3) {\n      return true;\n    }\n  }\n  return false;\n");
  const settle = lib.settle as unknown as (info: { value?: { status?: string } }, debug: boolean) => void;
  const [on, off, none] = [{ value: {} as { status?: string } }, { value: {} as { status?: string } }, {}];
  settle(on, true);
  settle(off, false);
  settle(none, true);
  expect([on.value.status, off.value.status, none]).toEqual(["fulfilled", undefined, {}]);
  const many = lib.many as unknown as (counts: { count?: number }, debug: boolean) => boolean;
  expect([many({ count: 4 }, true), many({ count: 4 }, false), many({ count: 2 }, true), many({}, true)]).toEqual([true, false, false, false]);
});

// A conditional of a `bool` and a value's negated truth, `!js::truthy(v)`,
// is a `bool` part by part: `!!v && f()`, never `v && f()`, whose `v` is
// the object, not `false`; `!!v`, not `v`, as a value; and `!!v || f()`,
// of a chain each part's. Of an `if` the program wrote, the conditional it
// wrote, `!v ? false : f()` (ADR 0879).
test("a value's truth or else a bool is a bool part by part", async () => {
  const { js, lib } = await compiled("negated-truth-bool", `use js::Unknown;

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.supportsIt")]
#[allow(unused_variables)]
fn supports(what: &str) -> bool {
    unreachable!()
}

pub fn and_then(v: Option<&Unknown>) -> bool {
    if !js::truthy(v) { false } else { supports("a") }
}

pub fn and_then_written(v: Option<&Unknown>) -> bool {
    js::truthy(v) && supports("a")
}

pub fn value(v: Option<&Unknown>) -> bool {
    if !js::truthy(v) { false } else { true }
}

pub fn or_else(v: Option<&Unknown>) -> bool {
    if !js::truthy(v) { supports("b") } else { true }
}

pub fn or_else_written(v: Option<&Unknown>) -> bool {
    js::truthy(v) || supports("b")
}

pub fn present_or(o: Option<&Vec<u32>>, f: &dyn Fn() -> bool) -> bool {
    match o {
        None => f(),
        Some(_) => true,
    }
}

pub fn unless(v: Option<&Unknown>, n: u32) -> bool {
    if !js::truthy(v) && n != 0 { supports("d") } else { true }
}

pub fn unless_written(v: Option<&Unknown>, n: u32) -> bool {
    js::truthy(v) || n == 0 || supports("d")
}

pub fn parts(v: Option<&Unknown>, n: u32) -> u32 {
    if n != 0 && js::truthy(v) { 1 } else { 2 }
}
`);
  expect(js).toContain('return !v ? false : globalThis.supportsIt("a");');
  expect(js).toContain('return !!v && globalThis.supportsIt("a");');
  expect(js).toContain("return !!v;");
  expect(js).toContain('return !v ? globalThis.supportsIt("b") : true;');
  expect(js).toContain('return !!v || globalThis.supportsIt("b");');
  // A `match`'s, MIR's test of `None` first: `!!o`, not the array.
  expect(js).toContain("return !!o || f();");
  expect(js).toContain('return !v && n ? globalThis.supportsIt("d") : true;');
  // A chain's part by part, `!!v || n === 0`, not `!!(v || n === 0)`.
  expect(js).toContain('return !!v || n === 0 || globalThis.supportsIt("d");');
  // A test reads each part by its truth, `v`, not `!!v`.
  expect(js).toContain("return n && v ? 1 : 2;");
  const globals = globalThis as typeof globalThis & { supportsIt?: (what: string) => boolean };
  globals.supportsIt = () => true;
  const object = {};
  expect([lib.andThen(object as never), lib.value(object as never), lib.orElse(object as never), lib.andThen(0 as never)]).toEqual([true, true, true, false]);
  expect([lib.andThenWritten(object as never), lib.orElseWritten(object as never), lib.unlessWritten(object as never, 1 as never), lib.andThenWritten(0 as never), lib.presentOr([1] as never, (() => false) as never)]).toEqual([true, true, true, false, true]);
});

// React's startGestureTransition: `if (!enableGestureTransition) { throw
// .. }`, then `// $FlowFixMe[constant-condition]` and `if
// (enableGestureTransition) { currentTransition.gesture = provider; }`.
// The second test, which the first decides, is the program's: kept, and
// its comment with it, not folded to its branch (ADR 0564, amended). One
// MIR made, a std drop's, still goes.
test("an if the program wrote stays where an earlier test decides it", async () => {
  (globalThis as Record<string, unknown>).__EXPERIMENTAL__ = true;
  const { js, lib } = await compiled("written-if-decided", `unsafe extern "Rust" {
    safe static __EXPERIMENTAL__: bool;
}

thread_local! {
    pub static ENABLED: bool = __EXPERIMENTAL__;
}

pub fn gesture(provider: Option<u32>) -> u32 {
    if !ENABLED.with(|&enabled| enabled) {
        panic!("off");
    }
    let mut gesture = 0;
    // $FlowFixMe[constant-condition]
    if ENABLED.with(|&enabled| enabled) {
        gesture = provider.unwrap_or(1);
    }
    gesture
}
`);
  expect(js).toContain("  let gesture = 0;\n  // $FlowFixMe[constant-condition]\n  if (ENABLED) {\n    gesture = provider ?? 1;\n  }\n");
  expect([lib.gesture(5 as never), lib.gesture(undefined as never)]).toEqual([5, 1]);
});

// React's ReactEventPriorities: `if a == 0 || a > b { a } else { b }` is
// upstream's `a === 0 || a > b ? a : b`. MIR's branches give `b` where
// both tests fail, so the function returned `b` first, of the negated
// chain, `a !== 0 && !(a > b) ? b : a`: its value is as written, the
// test and its branches (ADR 0879, amended).
test("a returned value of an if of an || keeps its test and its branches' order", async () => {
  const { js, lib } = await compiled("written-or-value", `pub fn lower_event_priority(a: i32, b: i32) -> i32 {
    if a == 0 || a > b { a } else { b }
}

pub fn higher_event_priority(a: i32, b: i32) -> i32 {
    if a != 0 && a < b { a } else { b }
}

pub fn unless_greater(a: i32, b: i32) -> i32 {
    if !(a > b) { a } else { b }
}
`);
  expect(js).toContain("  return a === 0 || a > b ? a : b;\n");
  expect(js).toContain("  return a && a < b ? a : b;\n");
  expect(js).toContain("  return a <= b ? a : b;\n");
  const pairs = [[0, 2], [2, 0], [2, 8], [8, 2], [3, 3]];
  expect(pairs.map(([a, b]) => lib.lowerEventPriority(a as never, b as never))).toEqual([0, 2, 8, 8, 3]);
  expect(pairs.map(([a, b]) => lib.higherEventPriority(a as never, b as never))).toEqual([2, 0, 2, 2, 3]);
  expect(pairs.map(([a, b]) => lib.unlessGreater(a as never, b as never))).toEqual([0, 0, 2, 2, 3]);
});

// React's ReactFiberLane: `if (... && !(disableLegacyMode && root.tag ===
// LegacyRoot))`. MIR's branches test each part, so the chain came back
// De Morgan'd, `(!DISABLE || root.tag !== 0)`; a `!( .. )` the program
// wrote of an `&&` or an `||` stays one, in an `if`, a `while` and a
// conditional, and a chain it wrote of negations stays that (ADR 0879,
// amended).
test("a written negation of an && or an || stays one", async () => {
  const { js, lib } = await compiled("written-negations", `pub const DISABLE_LEGACY_MODE: bool = false;

pub struct Root {
    pub tag: u32,
}

pub fn upgraded(root: &Root, lanes: i32) -> bool {
    if lanes & 2 != 0 && !(DISABLE_LEGACY_MODE && root.tag == 0) {
        return true;
    }
    false
}

pub fn neither(a: u32, b: u32) -> u32 {
    if !(a == 1 || b == 0) { a } else { b }
}

pub fn counted(mut n: u32, limit: u32) -> u32 {
    while !(n >= limit || n == 7) {
        n += 1;
    }
    n
}

pub fn joined(lanes: i32, tag: u32, disabled: bool) -> bool {
    if lanes & 2 != 0 && !(disabled || tag == 7) {
        return true;
    }
    false
}

pub fn as_written(a: u32, b: u32) -> bool {
    if a != 1 || b != 0 {
        return true;
    }
    false
}
`);
  expect(js).toContain("  if (lanes & 2 && !(DISABLE_LEGACY_MODE && root.tag === 0)) {\n");
  expect(js).toContain("  return !(a === 1 || b === 0) ? a : b;\n");
  expect(js).toContain("  while (!(n >= limit || n === 7)) {\n");
  expect(js).toContain("  if (a !== 1 || b) {\n");
  // MIR joins the negation's parts into the chain around it, `.. && !disabled && tag !== 7`.
  expect(js).toContain("  if (lanes & 2 && !(disabled || tag === 7)) {\n");
  expect([lib.joined(2 as never, 1 as never, false as never), lib.joined(2 as never, 7 as never, false as never)]).toEqual([true, false]);
  expect([lib.upgraded({ tag: 0 } as never, 2 as never), lib.upgraded({ tag: 1 } as never, 4 as never)]).toEqual([true, false]);
  expect([lib.neither(2 as never, 3 as never), lib.neither(1 as never, 3 as never), lib.neither(2 as never, 0 as never)]).toEqual([2, 3, 0]);
  expect([lib.counted(0 as never, 10 as never), lib.counted(8 as never, 10 as never), lib.counted(12 as never, 10 as never)]).toEqual([7, 10, 12]);
  expect([lib.asWritten(1 as never, 0 as never), lib.asWritten(2 as never, 0 as never)]).toEqual([false, true]);
});

// React's getComponentNameFromFiber, its `Throw` case: `if (__DEV__) {
// .. } return null;`. The `if` the program wrote with its body inside stays
// so, not `if (!__DEV__) { return null; } ..`, its body after it, the
// guard a `let … else` is (ADR 0879, amended).
test("an if the program wrote before a return keeps its body inside", async () => {
  const { js, lib } = await compiled("written-if-before-return", `pub const DEV: bool = true;

pub fn name(kind: u32, names: &[&str], parent: Option<&str>) -> Option<String> {
    if DEV {
        // The innermost name, of the last one long enough.
        for name in names.iter().rev() {
            if name.starts_with('x') && kind > 1 {
                return Some(name.to_string());
            }
        }
        let parent = parent?;
        return Some(format!("{parent}!"));
    }
    None
}

pub struct Fiber {
    pub kind: u32,
    pub parent: Option<Box<Fiber>>,
}

// Its \`?\`'s \`return None\` and the one after the \`if\`, MIR's one.
pub fn thrown(fiber: &Fiber) -> Option<u32> {
    match fiber.kind {
        0 => Some(0),
        1 => {
            if DEV {
                if fiber.kind > 3 {
                    return Some(3);
                }
                let parent = fiber.parent.as_ref()?;
                return thrown(parent);
            }
            None
        }
        _ => None,
    }
}

pub fn footer(href: Option<&str>) -> String {
    let Some(href) = href else {
        return "none".to_string();
    };
    if href.starts_with("https://") {
        return format!("ext {href}");
    }
    format!("next {href}")
}
`);
  expect(js).toContain("  if (DEV) {\n    // The innermost name, of the last one long enough.\n");
  expect(js).not.toContain("!DEV");
  // A \`let … else\`'s is its guard first, and an \`if\` MIR turned is as written.
  expect(js).toContain('  if (href == null) {\n    return "none";\n  }\n  if (href.startsWith("https://")) {\n');
  expect([lib.name(2 as never, ["a", "xb", "xc"] as never, "p" as never), lib.name(1 as never, ["x"] as never, "p" as never), lib.name(5 as never, [] as never, undefined as never)]).toEqual(["xc", "p!", undefined]);
  expect(js).toContain("      if (DEV) {\n        if (fiber.kind > 3) {\n");
  const root = { kind: 0, parent: undefined };
  expect([lib.thrown({ kind: 1, parent: root } as never), lib.thrown({ kind: 1, parent: undefined } as never), lib.thrown({ kind: 2, parent: root } as never)]).toEqual([0, undefined, undefined]);
  expect([lib.footer(undefined as never), lib.footer("https://x" as never), lib.footer("/y" as never)]).toEqual(["none", "ext https://x", "next /y"]);
});

// React's getComponentNameFromFiber, its `OffscreenComponent` case: `if
// (fiber.return !== null) { return getComponentNameFromFiber(fiber.return);
// } return null;`. The port's `if let Some(r) = parent { return f(&r); }
// return None;` is MIR's one `return` of a value each branch sets; it stays
// the `if` of `return`s the program wrote, not `return parent != null ?
// f(parent) : null;` (ADR 0657, amended). A value the program wrote with an
// `if` is still a conditional.
test("an if of returns the program wrote stays an if", async () => {
  (globalThis as { __DEV__?: boolean }).__DEV__ = true;
  const { js, lib } = await compiled("written-if-of-returns", `use std::cell::RefCell;
use std::rc::Rc;

pub struct Fiber {
    pub depth: u32,
    pub parent: RefCell<Option<Rc<Fiber>>>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn root_name(fiber: &Fiber) -> Option<String> {
    match fiber.depth {
        0 => return Some("root".to_string()),
        1 => {
            let parent = fiber.parent.borrow().clone();
            if let Some(parent) = parent {
                return root_name(&parent);
            }
            return None;
        }
        2 => return Some("two".to_string()),
        _ => {}
    }
    None
}

pub fn depth_or(fiber: &Fiber) -> u32 {
    if let Some(parent) = &*fiber.parent.borrow() { parent.depth } else { 0 }
}

unsafe extern "Rust" {
    pub safe static __DEV__: bool;
}

thread_local! {
    static DID_WARN_ABOUT_ELEMENT_REF: &'static js::Dict<bool> = js::dict::new();
}

// React's elementRefGetterWithDeprecationWarning, as its port writes it: a
// \`return\` of an \`if\` in \`if __DEV__ { .. }\`, after a warning given once,
// the function's tail \`None\`. The value the program wrote is the
// conditional returned.
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn ref_of(props: Option<&'static js::Unknown>, name: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    if __DEV__ {
        let component_name: Option<String> = unsafe { js::cast(name) };
        let key = component_name.as_deref().unwrap_or("null");
        if !DID_WARN_ABOUT_ELEMENT_REF.with(|did_warn_about_element_ref| {
            js::dict::get(did_warn_about_element_ref, key).is_some_and(|&warned| warned)
        }) {
            DID_WARN_ABOUT_ELEMENT_REF.with(|did_warn_about_element_ref| {
                js::dict::set(did_warn_about_element_ref, key, true);
            });
            println!("Accessing element.ref was removed in React 19.");
        }

        // An undefined \`element.ref\` is coerced to \`null\` for
        // backwards compatibility.
        let ref_prop = js::get(props, "ref");
        return if !js::strict_equals(ref_prop, None::<&js::Unknown>) {
            ref_prop
        } else {
            None
        };
    }
    None
}

// And an \`if\` that returns where its test shows its value defined.
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn ref_kept(props: Option<&'static js::Unknown>, other: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    let ref_prop = js::get(props, "ref");
    if !js::strict_equals(ref_prop, None::<&js::Unknown>) {
        return ref_prop;
    }
    other
}
`);
  expect(js).toContain("      const parent = fiber.parent;\n      if (parent != null) {\n        return rootName(parent);\n      }\n      return null;\n");
  expect(js).toContain("  return parent != null ? parent.depth : 0;\n");
  // The \`?? null\` a nullable function's result has, gone where the test
  // shows the value isn't \`undefined\` (ADR 0620, amended).
  expect(js).toContain("    const refProp = props.ref;\n    return refProp !== undefined ? refProp : null;\n  }\n  return null;\n");
  expect(js).toContain("  if (refProp !== undefined) {\n    return refProp;\n  }\n  return other ?? null;\n");
  const root = { depth: 0, parent: null };
  const leaf = { depth: 1, parent: root };
  expect([lib.rootName(leaf as never), lib.rootName({ depth: 1, parent: null } as never), lib.rootName({ depth: 2, parent: root } as never), lib.depthOr(leaf as never), lib.depthOr(root as never)]).toEqual(["root", null, "two", 0, 0]);
  expect([lib.refOf({ ref: 3 } as never, "A" as never), lib.refOf({ ref: null } as never, "A" as never), lib.refOf({} as never, undefined as never)]).toEqual([3, null, null]);
  expect([lib.refKept({ ref: 3 } as never, 4 as never), lib.refKept({ ref: null } as never, 4 as never), lib.refKept({} as never, undefined as never)]).toEqual([3, null, null]);
});

// React's runWithFiberInDEV, ported: `if let Some(fiber) = &fiber && let
// Some(debug_task) = fiber.debug_task.get() { return debug_task.run(move
// || callback(arg0)); }`. The chain's test reads `fiber._debugTask` in the
// binding's place, and the body reads it first, before anything runs that
// may change it: there it's read again, upstream's `fiber._debugTask.run(
// ..)`, no `const debugTask` (ADR 0866, amended). Where the body runs code
// first, or reads it twice, the binding stays, as Rust's value is the one
// the test read.
test("a let chain's binding of a field the test read is read again where the body reads it first", async () => {
  const dir = fixture("chain-binding-read-again");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::rc::Rc;

use webapi::ConsoleTask;

pub struct Fiber {
    #[cfg_attr(rust_js, rust_js::name = "_debugTask")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub debug_task: Cell<Option<&'static ConsoleTask>>,
}

#[cfg_attr(rust_js, rust_js::nullable(fiber))]
pub fn run_in_task(fiber: Option<Rc<Fiber>>, callback: impl FnOnce(u32) -> u32, arg0: u32) -> u32 {
    if let Some(fiber) = &fiber
        && let Some(debug_task) = fiber.debug_task.get()
    {
        return debug_task.run(move || callback(arg0));
    }
    callback(arg0)
}

/// Code runs before the binding's read, which may change the field.
pub fn cleared_first(fiber: Option<&Fiber>, callback: impl Fn(u32) -> u32) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.debug_task.get()
    {
        fiber.debug_task.set(None);
        return task.run(move || callback(1));
    }
    2
}

/// Read twice: the second read is after the first's code ran.
pub fn twice(fiber: Option<&Fiber>, callback: impl Fn(u32) -> u32) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.debug_task.get()
    {
        return task.run(|| callback(1)) + task.run(|| callback(2));
    }
    3
}

fn probe(fiber: &Fiber) -> bool {
    fiber.debug_task.set(None);
    true
}

/// Code runs before the binding's read, in the statement that reads it.
pub fn probed_before(fiber: Option<&Fiber>, callback: impl Fn(u32) -> u32) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.debug_task.get()
    {
        return u32::from(probe(fiber)) + task.run(|| callback(1));
    }
    5
}

/// The test runs code after it reads the field.
pub fn probed(fiber: Option<&Fiber>, callback: impl Fn(u32) -> u32) -> u32 {
    if let Some(fiber) = fiber
        && let Some(task) = fiber.debug_task.get()
        && probe(fiber)
    {
        return task.run(|| callback(1));
    }
    4
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (fiber != null && fiber._debugTask) {\n    return fiber._debugTask.run(() => callback(arg0));\n  }\n");
  expect(js.match(/const task = fiber\._debugTask;/g)).toHaveLength(4);
  expect(js).not.toContain("const debugTask");
  const lib = await import(join(dir, "lib.js"));
  const task = { run: (f: () => number) => f() + 100 };
  expect([lib.runInTask({ _debugTask: task } as never, (n: number) => n * 2, 4 as never), lib.runInTask(null as never, (n: number) => n * 2, 4 as never), lib.runInTask({} as never, (n: number) => n * 2, 5 as never)]).toEqual([108, 8, 10]);
  const fiber: { _debugTask: unknown } = { _debugTask: task };
  expect(lib.clearedFirst(fiber as never, (n: number) => n)).toBe(101);
  expect(fiber._debugTask).toBe(null);
  expect(lib.twice({ _debugTask: task } as never, (n: number) => n)).toBe(203);
  // A task whose run clears the field: the field read again would be null.
  const clearing: { _debugTask: unknown } = { _debugTask: null };
  clearing._debugTask = { run: (f: () => number) => ((clearing._debugTask = null), f()) };
  expect(lib.twice(clearing as never, (n: number) => n)).toBe(3);
  clearing._debugTask = task;
  expect(lib.probed(clearing as never, (n: number) => n)).toBe(101);
  clearing._debugTask = task;
  expect(lib.probedBefore(clearing as never, (n: number) => n)).toBe(102);
});

// React's ReactFiberHydrationDiffs, ported: `for (const propName in props)
// { if (!props.hasOwnProperty(propName)) { continue; } if (propName ===
// 'children') { // Ignored.\n continue; } .. }`. Each `continue` the program
// wrote is where it wrote it, its comment before it, in a loop that breaks
// too; a comment inside a let-chain's test is before its operand; and a
// `for mut` binding is the loop's `let`, no other declared (ADR 0684,
// amended).
test("a for…in's continues are as written, their comments in place, its mut binding its own", async () => {
  const { js, lib } = await compiled(
    "written-continues-in-keys",
    `use js::Unknown;

fn own(props: &Unknown, name: &str) -> bool {
    js::get(props, name).is_some()
}

pub fn element(props: &Unknown, mut max_length: f64) -> String {
    let mut content = String::new();
    for prop_name in js::keys_in(props) {
        if !own(props, prop_name.as_str()) {
            continue;
        }
        if prop_name == "children" {
            // Ignored.
            continue;
        }
        let prop_value = prop_name.repeat(2);
        max_length -= prop_name.len() as f64 + prop_value.len() as f64 + 2.0;
        if max_length < 0.0 {
            content += " ...";
            break;
        }
        content += &(" ".to_owned() + &prop_name + "=" + &prop_value);
    }
    "<".to_owned() + &content + ">"
}

pub fn diffed(client: &Unknown, a: Option<&str>, b: Option<&str>) -> String {
    let mut content = String::new();
    for prop_name in js::keys_in(client) {
        if !own(client, prop_name.as_str()) {
            continue;
        }
        if prop_name == "children" {
            // Handled below.
            continue;
        }
        if let Some(a) = a
            && let Some(b) = b
            && a.len() > 1
            // Only do the diff if the object has a lot of keys or was shortened.
            && (b.len() > 2 || a.contains("..."))
        {
            content += a;
        } else {
            content += &prop_name;
        }
    }
    content
}

pub fn described(value: Option<&Unknown>, on: bool) -> String {
    match js::type_of(value).as_str() {
        "string" => "text".to_string(),
        "object" => {
            let Some(value) = value else {
                return "null".to_string();
            };
            if on {
                let mut properties = String::new();
                for mut prop_name in js::keys_in(value) {
                    if prop_name.len() > 3 {
                        prop_name = format!("{prop_name:?}");
                    }
                    properties += &prop_name;
                }
                return "{".to_owned() + &properties + "}";
            }
            String::new()
        }
        _ => "other".to_string(),
    }
}
`,
  );
  expect(js).toContain(`  for (const propName in props) {
    if (!own(props, propName)) {
      continue;
    }
    if (propName === "children") {
      // Ignored.
      continue;
    }
    const propValue = propName.repeat(2);
`);
  expect(js).toContain(`    if (propName === "children") {
      // Handled below.
      continue;
    }
    if (
      a != null &&
      b != null &&
      $byteLen(a) > 1 &&
      // Only do the diff if the object has a lot of keys or was shortened.
      ($byteLen(b) > 2 || a.includes("..."))
    ) {
`);
  expect(js).toContain(`        let properties = "";
        for (let propName in value) {
`);
  expect(js.match(/let propName;/g)).toBeNull();
  expect(lib.element({ a: 1, children: 2, bb: 3 } as never, 100 as never)).toBe("< a=aa bb=bbbb>");
  expect(lib.element({ a: 1, bbbbbb: 3 } as never, 10 as never)).toBe("< a=aa ...>");
  expect(lib.diffed({ x: 1, children: 2, y: 3 } as never, "ab" as never, "c" as never)).toBe("xy");
  expect(lib.diffed({ x: 1 } as never, "ab" as never, "cde" as never)).toBe("ab");
  expect(lib.diffed({ x: 1 } as never, undefined as never, "cde" as never)).toBe("x");
  expect(lib.described({ ab: 1, abcd: 2 } as never, true as never)).toBe('{ab"abcd"}');
  expect([lib.described(undefined as never, true as never), lib.described({ ab: 1 } as never, false as never)]).toEqual(["other", ""]);
});

// React's ReactFiberThenable checks `thenable.status !== 'fulfilled' &&
// thenable.status !== 'rejected'`; its port's `!matches!(status,
// Some("fulfilled" | "rejected"))` was `!(status === "fulfilled" || status
// === "rejected")`. The program wrote no `!( .. )` of an `&&` or an `||`,
// which stays as written (ADR 0879, amended): a `!` of a chain MIR made of
// a pattern's alternatives is each one's negated test, De Morgan's, as a
// person writes it. One written `!(a == 1 || b == 0)` stays, in a value too.
test("a negated matches! of alternatives is each one's negated test", async () => {
  const { js, lib } = await compiled("negated-matches", `pub fn pending(status: Option<&str>) -> bool {
    !matches!(status, Some("fulfilled" | "rejected"))
}

pub fn neither(a: u32, b: u32) -> bool {
    !(a == 1 || b == 0)
}

pub fn small(n: u32) -> bool {
    !matches!(n, 1 | 2 | 3)
}

pub fn outside(x: f64) -> bool {
    !matches!(x, 1.0..=3.0)
}
`);
  expect(js).toContain('  return status !== "fulfilled" && status !== "rejected";\n');
  expect(js).toContain("  return !(a === 1 || b === 0);\n");
  expect(js).toContain("  return n !== 1 && n !== 2 && n !== 3;\n");
  expect([lib.pending("fulfilled" as never), lib.pending("rejected" as never), lib.pending("pending" as never), lib.pending(undefined as never)]).toEqual([false, false, true, true]);
  expect([lib.neither(2 as never, 3 as never), lib.neither(1 as never, 3 as never)]).toEqual([true, false]);
  expect([lib.small(2 as never), lib.small(4 as never)]).toEqual([false, true]);
  // A range's tests stay negated as one, as a float's `NaN` is neither.
  expect(js).toContain("  return !(x >= 1 && x <= 3);\n");
  expect([lib.outside(2 as never), lib.outside(4 as never), lib.outside(NaN as never)]).toEqual([false, true, true]);
});

// ADR 0946: React's popHydrationState, an `if` whose condition mixes `&&`,
// `||` and `!`, `tag !== HostRoot && tag !== HostSingleton && !(tag ===
// HostComponent && (!shouldDelete(type) || shouldSetText(type)))`, which
// MIR made `if`s that leave a block (`bb42:`): the one test as written,
// each call made where it was, in the order it was, none twice.
test("an if of && || and ! mixed is its one test", async () => {
  const { js, lib } = await compiled(
    "mixed-skipped-tests",
    `#[cfg_attr(rust_js, rust_js::link_name = "rustJsShouldDelete")]
fn should_delete(r#type: &str) -> bool {
    let _ = r#type;
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsShouldSetText")]
fn should_set_text(r#type: &str) -> bool {
    let _ = r#type;
    unreachable!()
}

pub fn pop(tag: u8, r#type: &str, out: &mut Vec<String>) {
    if tag != 3 && tag != 27 && !(tag == 5 && (!should_delete(r#type) || should_set_text(r#type))) {
        out.push("first".to_string());
    }
    if tag != 3 && (tag != 5 || (should_delete(r#type) && !should_set_text(r#type))) {
        out.push("second".to_string());
    }
    if !(tag == 5 && should_delete(r#type)) || should_set_text(r#type) {
        out.push("c".to_string());
    }
    if tag != 3 && (tag == 27 || !(should_delete(r#type) && should_set_text(r#type))) {
        out.push("d".to_string());
    }
    if !(tag == 5 || should_delete(r#type)) || !should_set_text(r#type) {
        out.push("e".to_string());
    }
    if tag != 3 && (tag == 27 || should_delete(r#type) && !should_set_text(r#type)) {
        out.push("f".to_string());
    }
    if tag != 3 && !(tag == 27 || !should_delete(r#type) && should_set_text(r#type)) {
        out.push("g".to_string());
    }
    out.push("end".to_string());
}
`,
  );
  expect(js).not.toMatch(/\bbb\d+/);
  expect(js).toContain(
    '  if (\n    tag !== 3 &&\n    tag !== 27 &&\n    !(tag === 5 && (!rustJsShouldDelete(type) || rustJsShouldSetText(type)))\n  ) {\n    out.push("first");\n  }\n',
  );
  expect(js).toContain('  if (tag !== 3 && (tag !== 5 || (rustJsShouldDelete(type) && !rustJsShouldSetText(type)))) {\n    out.push("second");\n  }\n');
  const global = globalThis as Record<string, unknown>;
  for (const tag of [3, 27, 5, 1]) {
    for (const del of [false, true]) {
      for (const text of [false, true]) {
        const calls: string[] = [];
        global.rustJsShouldDelete = (t: string) => (calls.push(`delete ${t}`), del);
        global.rustJsShouldSetText = (t: string) => (calls.push(`text ${t}`), text);
        const out: string[] = [];
        lib.pop(tag as never, "div" as never, out as never);
        const expected: string[] = [];
        const want: string[] = [];
        const shouldDelete = () => (expected.push("delete div"), del);
        const shouldSetText = () => (expected.push("text div"), text);
        if (tag !== 3 && tag !== 27 && !(tag === 5 && (!shouldDelete() || shouldSetText()))) want.push("first");
        if (tag !== 3 && (tag !== 5 || (shouldDelete() && !shouldSetText()))) want.push("second");
        if (!(tag === 5 && shouldDelete()) || shouldSetText()) want.push("c");
        if (tag !== 3 && (tag === 27 || !(shouldDelete() && shouldSetText()))) want.push("d");
        if (!(tag === 5 || shouldDelete()) || !shouldSetText()) want.push("e");
        if (tag !== 3 && (tag === 27 || (shouldDelete() && !shouldSetText()))) want.push("f");
        if (tag !== 3 && !(tag === 27 || (!shouldDelete() && shouldSetText()))) want.push("g");
        want.push("end");
        expect([tag, del, text, out, calls]).toEqual([tag, del, text, want, expected]);
      }
    }
  }
});

// ADR 0947: React's isLegacyActEnvironment, `if (__DEV__) { ..; return a
// && b && c; } return false;`: the `return` the program wrote is its value,
// as written, not `if`s whose failing tests go on to the `return false`
// after the `if`.
test("a returned && chain in an if is returned as written", async () => {
  const { js, lib } = await compiled(
    "returned-and-chain",
    `pub fn legacy(dev: bool, a: bool, b: bool, c: u32) -> bool {
    if dev {
        let b_is_defined = b;
        return a && b_is_defined && c != 0;
    }
    false
}

pub fn either(dev: bool, a: bool, b: bool) -> bool {
    if dev {
        return a || !b;
    }
    true
}
`,
  );
  expect(js).toContain("  if (dev) {\n    const bIsDefined = b;\n    return a && bIsDefined && c !== 0;\n  }\n  return false;\n");
  expect(js).toContain("  if (dev) {\n    return a || !b;\n  }\n  return true;\n");
  const bools = [false, true];
  for (const dev of bools) {
    for (const a of bools) {
      for (const b of bools) {
        for (const c of [0, 1]) {
          expect([dev, a, b, c, lib.legacy(dev as never, a as never, b as never, c as never)]).toEqual([dev, a, b, c, !dev ? false : a && b && c !== 0]);
        }
        expect([dev, a, b, lib.either(dev as never, a as never, b as never)]).toEqual([dev, a, b, !dev ? true : a || !b]);
      }
    }
  }
});

// ADR 0657, amended: React's getClassNameByType, `if (className == null) {
// className = match; } else { className += ' ' + match; }`: an `if` one
// branch of which adds to what it assigns is as written, not `className =
// className == null ? m : className + (" " + m)`.
test("an if a branch of which adds to its variable is as written", async () => {
  const { js, lib } = await compiled(
    "if-adding-branch",
    `pub fn joined(matches: &[String]) -> Option<String> {
    let mut class_name: Option<String> = None;
    for m in matches {
        match &mut class_name {
            None => class_name = Some(m.clone()),
            Some(class_name) => *class_name += &(" ".to_string() + m),
        }
    }
    class_name
}
`,
  );
  expect(js).toContain('    if (className == null) {\n      className = m;\n    } else {\n      className += " " + m;\n    }\n');
  expect([lib.joined([] as never), lib.joined(["a"] as never), lib.joined(["a", "b", "c"] as never)]).toEqual([undefined, "a", "a b c"]);
});

// ADR 0954: React's tryToClaimNextHydratableFormMarkerInstance ends with a
// call that never returns, a `-> !` function's: nothing after it, not `let
// result; return result;` of a value no path makes.
test("a call that never returns ends its function", async () => {
  const { js, lib } = await compiled(
    "diverging-call-ends",
    `fn mismatch(id: u32) -> ! {
    panic!("mismatch {id}")
}

pub fn claim(hydrating: bool, id: u32) -> bool {
    if !hydrating {
        return false;
    }
    if id > 10 {
        return id % 2 == 0;
    }
    mismatch(id);
}
`,
  );
  expect(js).toContain("    return id % 2 === 0;\n  }\n  mismatch(id);\n}\n");
  expect(js).not.toContain("let result");
  expect([lib.claim(false as never, 1 as never), lib.claim(true as never, 12 as never), lib.claim(true as never, 13 as never)]).toEqual([false, true, false]);
  expect(() => lib.claim(true as never, 3 as never)).toThrow("mismatch 3");
});

// ADR 0947, amended, and ADR 0953: React's isFragmentInstanceHostBoundary,
// `return tag === HostComponent || tag === HostRoot || (supportsSingletons
// ? tag === HostSingleton : false);`: a function's last value of `||`s,
// its chain of `if`s and the `return` after them neither the program's, is
// one `return`; and a `bool` binding's static reads as the `boolean` it is,
// `supportsSingletons && ..`, no `!!`.
test("a function's last || of an if's value is one return", async () => {
  const { js, lib } = await compiled(
    "or-of-if-value",
    `unsafe extern "Rust" {
    #[link_name = "rustJsSupports"]
    safe static SUPPORTS: bool;
}

pub const HOST_COMPONENT: u8 = 5;
pub const HOST_ROOT: u8 = 3;
pub const HOST_SINGLETON: u8 = 27;

pub struct Fiber {
    pub tag: std::cell::Cell<u8>,
}

pub fn boundary_if(fiber: &Fiber) -> bool {
    fiber.tag.get() == HOST_COMPONENT
        || fiber.tag.get() == HOST_ROOT
        || (if SUPPORTS { fiber.tag.get() == HOST_SINGLETON } else { false })
}

pub fn boundary_and(fiber: &Fiber) -> bool {
    fiber.tag.get() == HOST_COMPONENT || fiber.tag.get() == HOST_ROOT || (SUPPORTS && fiber.tag.get() == HOST_SINGLETON)
}
`,
  );
  expect(js).toContain(
    "    fiber.tag === HOST_COMPONENT ||\n    fiber.tag === HOST_ROOT ||\n    (rustJsSupports ? fiber.tag === HOST_SINGLETON : false)\n",
  );
  expect(js).toContain(
    "    fiber.tag === HOST_COMPONENT ||\n    fiber.tag === HOST_ROOT ||\n    (rustJsSupports && fiber.tag === HOST_SINGLETON)\n",
  );
  const global = globalThis as Record<string, unknown>;
  for (const supports of [false, true]) {
    global.rustJsSupports = supports;
    for (const tag of [5, 3, 27, 1]) {
      const want = tag === 5 || tag === 3 || (supports && tag === 27);
      expect([supports, tag, lib.boundaryIf({ tag } as never), lib.boundaryAnd({ tag } as never)]).toEqual([supports, tag, want, want]);
    }
  }
});

// The user's decision (2026-10-27), ADR 0879, amended: React's
// insertOrAppendPlacementNode writes `if (isHost) { ..; return; } else if
// (tag === HostPortal) { return; } ..more`. An `else` the program wrote
// after a branch that returns stays, where statements follow the `if`; a
// chain the function ends with is still each `if` after the last (ADR
// 0237), and `if`s written apart stay apart.
test("an else after a branch that returns stays where statements follow", async () => {
  const { js, lib } = await compiled(
    "else-after-return",
    `pub fn place(tag: u8, log: &mut Vec<u8>) {
    let is_host = tag == 5 || tag == 6;
    if is_host {
        log.push(tag);
        return;
    } else if tag == 4 {
        // A portal's children are placed on their own.
        return;
    }
    log.push(0);
}

pub fn counted(key: u32, total: &mut u32) -> u32 {
    if key == 0 {
        return 0;
    } else {
        *total += key;
    }
    *total
}

pub fn turned(key: u32, total: &mut u32) -> u32 {
    if key != 0 {
        *total += key;
    } else {
        return 0;
    }
    *total
}

pub fn apart(key: u32) -> u32 {
    if key == 1 {
        return 10;
    }
    if key == 2 {
        return 20;
    }
    key
}

pub fn last(key: u32) -> u32 {
    if key == 1 {
        return 10;
    } else if key == 2 {
        return 20;
    } else {
        key + 1
    }
}
`,
  );
  expect(js).toContain(
    "  if (isHost) {\n    log.push(tag);\n    return;\n  } else if (tag === 4) {\n    // A portal's children are placed on their own.\n    return;\n  }\n  log.push(0);\n",
  );
  expect(js).toContain("  if (key === 0) {\n    return 0;\n  } else {\n    total.value = (total.value + key) >>> 0;\n  }\n  return total.value;\n");
  expect(js).toContain("  if (key) {\n    total.value = (total.value + key) >>> 0;\n  } else {\n    return 0;\n  }\n  return total.value;\n");
  expect(js).toContain("  if (key === 1) {\n    return 10;\n  }\n  if (key === 2) {\n    return 20;\n  }\n  return key;\n");
  expect(js).toContain("  if (key === 1) {\n    return 10;\n  }\n  if (key === 2) {\n    return 20;\n  }\n  return (key + 1) >>> 0;\n");
  const placed = [5, 4, 3].map((tag) => {
    const log: number[] = [];
    lib.place(tag as never, log as never);
    return log;
  });
  expect(placed).toEqual([[5], [], [0]]);
  const total = { value: 1 };
  expect([lib.counted(0 as never, total as never), lib.counted(2 as never, total as never)]).toEqual([0, 3]);
  expect([lib.turned(0 as never, total as never), lib.turned(4 as never, total as never)]).toEqual([0, 7]);
  expect([1, 2, 3].map((k) => [lib.apart(k as never), lib.last(k as never)])).toEqual([[10, 10], [20, 20], [3, 4]]);
});

// The user's decision (2026-10-27), ADR 0879, amended: React's
// commitLayoutEffectOnFiber's `if (current === null) { if (..) { .. } else
// { try { .. } catch (error) { .. } } } else { .. }`. A function's last
// `match` of arms that give `()` is its arms as written, an `if`'s and a
// `try`'s branches too, with no `return;` MIR made at their ends.
test("a function's last match of arms that give () is its arms, no returns", async () => {
  const { js, lib } = await compiled(
    "tail-match-arms",
    `use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn lifecycles(current: Option<u32>, profile: bool, log: &mut Vec<u32>) {
    match current {
        None => {
            if profile {
                log.push(1);
            } else if let Err(error) = catch_unwind(AssertUnwindSafe(|| {
                if log.len() > 2 {
                    panic!("full");
                }
                log.push(2);
            })) && let Some(error) = error.downcast_ref::<&'static str>()
            {
                log.push(error.len() as u32);
            }
        }
        Some(prev) => log.push(prev),
    }
}
`,
  );
  expect(js).toContain("  if (current == null) {\n    if (profile) {\n      log.push(1);\n    } else ");
  expect(js).toContain("  } else {\n    log.push(current);\n  }\n}");
  expect(js).not.toContain("return;");
  const log: number[] = [];
  lib.lifecycles(undefined as never, true as never, log as never);
  lib.lifecycles(undefined as never, false as never, log as never);
  lib.lifecycles(7 as never, false as never, log as never);
  lib.lifecycles(undefined as never, false as never, log as never);
  expect(log).toEqual([1, 2, 7, 4]);
});
