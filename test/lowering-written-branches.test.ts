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
// `no-else-return` allows; a chain whose arms each leave up to it is still
// each `if` after the last.
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
  // Each arm up to the last leaves: each `if` after the last (ADR 0237).
  expect(js).toContain("  if (key === 1) {\n    return 10;\n  }\n  if (key === 2) {\n    return 20;\n  }\n  if (key > 5) {\n");
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
`);
  expect(js).toContain("      const parent = fiber.parent;\n      if (parent != null) {\n        return rootName(parent);\n      }\n      return null;\n");
  expect(js).toContain("  return parent != null ? parent.depth : 0;\n");
  const root = { depth: 0, parent: null };
  const leaf = { depth: 1, parent: root };
  expect([lib.rootName(leaf as never), lib.rootName({ depth: 1, parent: null } as never), lib.rootName({ depth: 2, parent: root } as never), lib.depthOr(leaf as never), lib.depthOr(root as never)]).toEqual(["root", null, "two", 0, 0]);
});
