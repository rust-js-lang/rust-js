// The js crate's collections and constructors as TypeScript types them: a
// `Set` and a `Map` of any value, made of what they start with, a
// `FinalizationRegistry` the host may lack, and `instanceof` of a
// constructor held as a value. Each test is a program of its own, compiled
// and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { node } from "./programs";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

function compiled(name: string, source: string): { dir: string; js: string; dts: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  return { dir, js: readFileSync(join(dir, "lib.js"), "utf8"), dts: readFileSync(join(dir, "lib.d.ts"), "utf8") };
}

function ran(dir: string, script: string, ...args: string[]): string {
  writeFileSync(join(dir, "run.mjs"), script);
  const result = runSync([node!, "--expose-gc", join(dir, "run.mjs"), ...args], root, 30_000);
  expect(result.stderr).toBe("");
  return result.stdout;
}

// ADRs 0260, 0597: a `FinalizationRegistry`'s callback is one JS calls
// again, a `Box<dyn Fn(T)>`, so its constructor's every type is named and
// `is_defined` asks of it: React's ReactTaint makes one only where the host
// has it, `typeof FinalizationRegistry !== "undefined" ? new
// FinalizationRegistry(cleanup) : ..`.
test("whether the host has a FinalizationRegistry is is_defined of its constructor", () => {
  const { dir, js } = compiled("js-finalization-registry", `use js::{FinalizationRegistry, JsObject};
use std::cell::RefCell;

thread_local! {
    static CLEANED_UP: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn cleanup(held: String) {
    CLEANED_UP.with_borrow_mut(|cleaned| cleaned.push(held));
}

// If FinalizationRegistry doesn't exist, we assume that objects live forever.
thread_local! {
    static FINALIZATION_REGISTRY: Option<&'static FinalizationRegistry<String>> =
        js::is_defined(js::finalization_registry::new::<String>)
            .then(|| js::finalization_registry::new(Box::new(cleanup)));
}

pub fn register(target: &JsObject, held: String) -> bool {
    if let Some(registry) = FINALIZATION_REGISTRY.with(|&registry| registry) {
        registry.register(target, held, None);
        return true;
    }
    false
}

pub fn cleaned() -> Vec<String> {
    CLEANED_UP.with_borrow(|cleaned| cleaned.clone())
}
`);
  expect(js).toContain(
    'const finalizationRegistry =\n  typeof FinalizationRegistry !== "undefined" ? new FinalizationRegistry(cleanup) : undefined;\n',
  );
  expect(js).toContain("    finalizationRegistry.register(target, held);\n");
  const script = `if (process.argv[2] === "none") delete globalThis.FinalizationRegistry;
const lib = await import("./lib.js");
const registered = lib.register({}, "a");
if (registered) {
  for (let i = 0; i < 20 && lib.cleaned().length === 0; i++) {
    globalThis.gc();
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}
console.log(JSON.stringify([registered, lib.cleaned()]));
`;
  expect(ran(dir, script, "has")).toBe('[true,["a"]]\n');
  expect(ran(dir, script, "none")).toBe("[false,[]]\n");
});

// ADR 0878: `value instanceof C` of a constructor held as a value, React's
// typed arrays' shared one, `getPrototypeOf(Uint32Array.prototype)
// .constructor`, and of the one class the js crate names, `DataView`: false
// of another realm's typed array, as upstream's is, which
// `ArrayBuffer.isView` would call one.
test("instanceof of a constructor value is js::instance_of", () => {
  const { dir, js } = compiled("js-instance-of", `use js::Unknown;

thread_local! {
    // This is the shared constructor of all typed arrays.
    #[cfg_attr(rust_js, rust_js::name = "TypedArrayConstructor")]
    static TYPED_ARRAY_CONSTRUCTOR: Option<&'static Unknown> = js::get(
        js::object::get_prototype_of(js::unknown(js::uint32_array::prototype())),
        "constructor",
    );
}

pub fn is_binary(value: Option<&Unknown>) -> bool {
    js::instance_of(value, TYPED_ARRAY_CONSTRUCTOR.with(|&constructor| constructor))
        || js::data_view::is(value)
}

pub fn is_view(value: Option<&Unknown>) -> bool {
    js::array_buffer::is_view(value)
}

pub fn of_nothing(value: Option<&Unknown>) -> bool {
    js::instance_of(value, None::<&Unknown>)
}
`);
  expect(js).toContain("const TypedArrayConstructor = Object.getPrototypeOf(Uint32Array.prototype).constructor;\n");
  expect(js).toContain("  return value instanceof TypedArrayConstructor || value instanceof DataView;\n");
  const out = ran(dir, `import vm from "node:vm";
const lib = await import("./lib.js");
const other = vm.runInNewContext("new Uint8Array(2)");
const values = [new Uint8Array(1), new Float64Array(1), new DataView(new ArrayBuffer(1)), new ArrayBuffer(1), {}, "s", undefined, other];
console.log(JSON.stringify([values.map(lib.isBinary), lib.isView(other)]));
try {
  lib.ofNothing({});
} catch (e) {
  console.log(e.constructor.name);
}
`);
  expect(out).toBe("[[true,true,true,false,false,false,false,false],true]\nTypeError\n");
});

// ADR 0875: a `js::Set` is TypeScript's `Set<T>` of any value, as a
// `js::Map` is a `Map<K, V>` of any key, each made of what it starts with,
// `new Map([[frozen, true]])`, `new Set([frozen])`, or of nothing, `None`,
// as React's BadMapPolyfill asks of a frozen key.
test("a Set holds any value, and a Set and a Map are made of what they start with", () => {
  const { dir, js, dts } = compiled("js-set", `use js::{Dict, Map, Set, Unknown};

pub fn has_bad_map_polyfill() -> bool {
    let frozen = js::object::freeze(js::dict::new::<u32>());
    let map: &Map<Dict<u32>, bool> = js::map::new(Some(&[(frozen, true)]));
    let set: &Set<Dict<u32>> = js::set::new(Some(&[frozen]));
    !(map.has(frozen) && set.has(frozen) && map.get(frozen) == Some(true))
}

pub fn counted(values: &[Option<&'static Unknown>]) -> (u32, bool, bool, u32) {
    let set: &Set<Option<&'static Unknown>> = js::set::new(None);
    for value in values {
        set.add(value);
    }
    let size = set.size();
    let had = set.delete(&values[0]);
    let has = set.has(&values[0]);
    set.clear();
    (size, had, has, set.size())
}

pub fn texts() -> Vec<bool> {
    let set: &Set<str> = js::set::new(Some(&["a", "b", "a"]));
    vec![set.has("a"), set.has("c"), set.size() == 2]
}

pub fn kept() -> &'static Set<str> {
    js::set::new(Some(&["x"]))
}
`);
  expect(js).toContain("  const map = new Map([[frozen, true]]);\n  const set = new Set([frozen]);\n");
  expect(js).toContain("  const set = new Set();\n");
  expect(js).toContain('  const set = new Set(["a", "b", "a"]);\n');
  expect(dts).toContain("export function kept(): Set<string>;");
  const out = ran(dir, `const lib = await import("./lib.js");
const o = {};
console.log(JSON.stringify([lib.hasBadMapPolyfill(), lib.counted([o, o, {}, undefined, null, NaN, NaN, 0, -0]), lib.texts(), [...lib.kept()]]));
`);
  expect(out).toBe('[false,[6,true,false,0],[true,false,true],["x"]]\n');
});

// ADR 0884: a `js::Map`'s or a `js::WeakMap`'s value is any value, as
// TypeScript's `Map<K, V>` takes any `V`, as a `Dict`'s is (ADR 0225): of
// an `Option`, `get` is `Some(None)` of a key whose value is `undefined` or
// `null`, as `has` tells, and `None` of a key it lacks. React's
// BadMapPolyfill keys a frozen object to `null`, `Kind::Null`.
test("a Map's value is any value, and get tells a nullish one from a missing key", () => {
  const { dir, js, dts } = compiled("js-map-options", `use js::{Dict, Kind, Map, Unknown, WeakMap};

pub fn has_bad_map_polyfill() -> bool {
    let frozen_object = js::object::freeze(js::dict::new::<Kind>());
    js::map::new(Some(&[(frozen_object, Kind::Null)]));
    js::set::new(Some(&[frozen_object]));
    let map = js::map::new(Some(&[(frozen_object, Kind::Null)]));
    !matches!(map.get(frozen_object), Some(Kind::Null))
}

pub fn options() -> Vec<String> {
    let map: &Map<str, Option<u32>> = js::map::new(Some(&[("none", None), ("one", Some(1))]));
    map.set("zero", Some(0));
    map.set("later", None);
    ["none", "one", "zero", "later", "absent"]
        .iter()
        .map(|key| format!("{key} {:?} {}", map.get(key), map.has(key)))
        .collect()
}

pub fn given(map: &Map<str, Option<&'static Unknown>>, key: &str) -> &'static str {
    match map.get(key) {
        None => "absent",
        Some(None) => "nullish",
        Some(Some(_)) => "value",
    }
}

pub fn weak(key: &Dict<u32>) -> (bool, bool, bool) {
    let map: &WeakMap<Dict<u32>, Option<&str>> = js::weak_map::new();
    map.set(key, None);
    let other = js::dict::new();
    let held: &Map<Dict<u32>, Option<&str>> = js::map::new(Some(&[(key, None)]));
    (map.get(key) == Some(None), map.get(other).is_none(), held.get(key) == Some(None))
}
`);
  expect(js).toContain("  new Map([[frozenObject, null]]);\n  new Set([frozenObject]);\n");
  expect(js).toContain("$someGet(map, key)");
  expect(dts).toContain("export function given(map: Map<string, unknown | null | undefined>, key: string): string;");
  const out = ran(dir, `const lib = await import("./lib.js");
const given = new Map([["undefined", undefined], ["null", null], ["zero", 0]]);
console.log(JSON.stringify([lib.hasBadMapPolyfill(), lib.options(), ["undefined", "null", "zero", "absent"].map((key) => lib.given(given, key)), lib.weak({})]));
`);
  expect(out).toBe(
    '[false,["none Some(None) true","one Some(Some(1)) true","zero Some(Some(0)) true","later Some(None) true","absent None false"],["nullish","nullish","value","absent"],[true,true,true]]\n',
  );
});

// ADR 0875, amended: a `Set`'s `forEach` of a closure that borrows what's around it,
// as React's ReactFiberLane's `getTransitionsForLanes` pushes each
// transition, and its iterators; and a set's and a map's own methods,
// which run nothing of what they hold, so a borrow held across them, of
// `transitionLanes` as React's `addTransitionToLanesMap` holds it, isn't
// counted.
test("a Set's forEach and iterators, and a borrow held across its own methods", () => {
  const { dir, js, dts } = compiled("js-set-steps", `use js::{Set, Unknown};
use std::cell::RefCell;

pub struct Root {
    pub transition_lanes: RefCell<Vec<Option<&'static Set<Unknown>>>>,
}

pub fn root() -> Root {
    Root { transition_lanes: RefCell::new(vec![None; 3]) }
}

pub fn add_transition(root: &Root, transition: &Unknown, index: usize) {
    let mut transition_lanes_map = root.transition_lanes.borrow_mut();
    let transitions = transition_lanes_map[index].unwrap_or_else(|| js::set::new(None));
    transitions.add(transition);
    transition_lanes_map[index] = Some(transitions);
}

pub fn transitions_for(root: &Root, index: usize) -> Option<Vec<&'static Unknown>> {
    let mut transitions_for_lanes = Vec::new();
    let transitions = root.transition_lanes.borrow()[index];
    if let Some(transitions) = transitions {
        transitions.for_each(|transition| {
            transitions_for_lanes.push(transition);
        });
    }
    if transitions_for_lanes.is_empty() {
        return None;
    }
    Some(transitions_for_lanes)
}

pub fn listed(set: &Set<str>) -> Vec<String> {
    let mut out: Vec<String> = set.values().map(str::to_uppercase).collect();
    for (key, value) in set.entries() {
        out.push(format!("{key}={value}"));
    }
    out.extend(set.keys().map(|key| key.len().to_string()));
    out
}
`);
  expect(js).toContain(
    "  const transitionLanesMap = root.transitionLanes;\n  const transitions = $index(transitionLanesMap, index) ?? new Set();\n  transitions.add(transition);\n",
  );
  expect(js).toContain("    transitions.forEach((transition) => {\n      transitionsForLanes.push(transition);\n    });\n");
  expect(js).not.toContain("$borrow");
  expect(dts).toContain("export function listed(set: Set<string>): string[];");
  const out = ran(dir, `const lib = await import("./lib.js");
const root = lib.root();
const a = {}, b = {};
lib.addTransition(root, a, 1);
lib.addTransition(root, b, 1);
lib.addTransition(root, a, 1);
const found = lib.transitionsFor(root, 1);
console.log(JSON.stringify([found.length, found[0] === a, found[1] === b, lib.transitionsFor(root, 0) ?? null, lib.listed(new Set(["a", "bc"]))]));
`);
  expect(out).toBe('[2,true,true,null,["A","BC","a=a","bc=bc","1","2"]]\n');
});

// ADR 0875, amended: a `Map`'s `forEach` of a closure that borrows what's
// around it, given each value and its key, as React's
// `ReactFiberHydrationDiffs.js` writes `serverPropNames.forEach(propName =>
// ..)`, its `return` the closure's; and its iterators, each a JS iterator.
test("a Map's forEach and iterators", () => {
  const { dir, js, dts } = compiled("js-map-steps", `use js::{Map, Unknown};

pub fn removed_props(server_props: &Unknown) -> String {
    let mut content = String::new();
    let server_prop_names: &Map<str, String> = js::map::new(None);
    for prop_name in js::keys_in(server_props) {
        server_prop_names.set(&prop_name.to_lowercase(), prop_name);
    }
    server_prop_names.for_each(|prop_name, _| {
        if prop_name == "children" {
            // Handled below.
            return;
        }
        content += &("- ".to_owned() + &prop_name + "\\n");
    });
    content
}

pub fn keyed(map: &Map<str, u32>) -> Vec<String> {
    let mut out = Vec::new();
    map.for_each(|value, key| out.push(format!("{key}:{value}")));
    out
}

pub fn listed(map: &Map<str, u32>) -> Vec<String> {
    let mut out: Vec<String> = map.keys().map(str::to_uppercase).collect();
    for (key, value) in map.entries() {
        out.push(format!("{key}={value}"));
    }
    out.extend(map.values().map(|value| value.to_string()));
    out
}
`);
  expect(js).toContain(
    '  serverPropNames.forEach((propName) => {\n    if (propName === "children") {\n      // Handled below.\n      return;\n    }\n',
  );
  expect(js).toContain("  map.forEach((value, key) => {\n");
  expect(js).toContain("  for (const [key, value] of map.entries()) {\n");
  expect(dts).toContain("export function listed(map: Map<string, number>): string[];");
  const out = ran(dir, `const lib = await import("./lib.js");
const map = new Map([["a", 1], ["bc", 2]]);
console.log(JSON.stringify([lib.removedProps({ children: 1, className: "x", ID: 2 }), lib.keyed(map), lib.listed(map)]));
`);
  expect(out).toBe('["- className\\n- ID\\n",["a:1","bc:2"],["A","BC","a=1","bc=2","1","2"]]\n');
});

// React's `[${prevDeps.join(', ')}]`: `js::array::join` of any values, each
// one's text as JS's `String()` gives it, `undefined` and `null` none.
test("js::array::join is an array of any values' join", () => {
  const { dir, js } = compiled("js-array-join", `use js::Unknown;

pub fn text(deps: &[Option<&'static Unknown>]) -> String {
    format!("[{}]", js::array::join(deps, ", "))
}
`);
  expect(js).toContain('  return `[${deps.join(", ")}]`;\n');
  expect(ran(dir, `import { text } from "./lib.js"; console.log(text([1, null, "a", undefined, true]));`)).toBe("[1, , a, , true]\n");
});
