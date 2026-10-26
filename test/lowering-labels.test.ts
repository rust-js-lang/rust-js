// What the compiler writes of a block the program labeled, `'desc: { ..
// }`: its label where JS keeps one, by the name the program gave it (ADR
// 0742). Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, performanceTrack, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = () => ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// ADR 0742: a JS block MIR's breaks leave, that a `break` the program
// wrote of a block it labeled goes to the end of, is the program's label,
// `desc: { .. break desc; }`, not one MIR names by its block, `bb41`.
// React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
// leaves its `'desc` block from deep inside.
test("a block the program labeled keeps its label", async () => {
  const dir = fixture("labeled-block-names");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>, indent: usize, out: &mut Vec<String>) -> String {
    let desc = if let Some(value) = value
        && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(value))
    {
        'desc: {
            let keys = js::object::keys(value);
            if keys.is_empty() {
                break 'desc "empty".to_string();
            }
            let name = keys.join(",");
            if name.len() > 10 {
                break 'desc name;
            }
            if name == "a,b" {
                if indent > 4 {
                    break 'desc "deep".to_string();
                } else if indent > 3 {
                    out.push(name);
                    return String::new();
                }
            }
            out.push(name);
            return String::new();
        }
    } else {
        "other".to_string()
    };
    desc
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toMatch(/\n\s+desc: \{\n/);
  expect(js).toMatch(/desc = "deep";\n\s+break desc;\n/);
  expect(js).not.toMatch(/\bbb\d+/);
  const lib = await import(join(dir, "lib.js"));
  const out: string[] = [];
  const got = [{}, { abcdef: 1, ghijkl: 2 }, { a: 1, b: 2 }, { c: 1 }, null].map((v) => lib.describe(v, 5, out));
  expect(got).toEqual(["empty", "abcdef,ghijkl", "deep", "", "other"]);
  expect(out).toEqual(["c"]);
});

// ADR 0745: React's addObjectDiffToProperties is upstream's chain of `if`s,
// each arm's `continue` where it's written, with no block MIR made; and
// addValueToProperties' `'desc` block, which ends a case, is the case's
// `break`s, not merged into a block MIR made (ADR 0790).
test("React's addObjectDiffToProperties has no block MIR made", () => {
  const of = performanceTrack();
  const diff = of("addObjectDiffToProperties");
  expect(diff).not.toMatch(/\bbb\d+/);
  expect(diff).toContain('          properties.push([REMOVED + line, "…"], [ADDED + line, "…"]);\n          isDeeplyEqual = false;\n          continue;\n        }\n');
  expect(diff).toContain("              if (!nestedEqual) {\n                isDeeplyEqual = false;\n              } else if (");
  expect(of("addValueToProperties")).not.toMatch(/\bbb\d+|desc: \{/);
});

// React's getArrayKind (shared/ReactPerformanceTrackProperties.js) is a
// loop of an `if`/`else` chain whose arms return. Its port's `match`, whose
// arm of `return` MIR goes on to from some paths of the arms before it and
// leaves past from others, is that `return` in each path that goes on to
// it, and no block MIR made (ADR 0754).
test("a match's return some arms go on to is in each, not a block's", async () => {
  const dir = fixture("labeled-block-exits");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

#[derive(Clone, Copy, PartialEq)]
pub enum ArrayKind {
    Empty,
    Complex,
    Primitive,
    Entries,
}

pub fn get_array_kind(array: &[Option<&Unknown>]) -> ArrayKind {
    let mut kind = ArrayKind::Empty;
    for &value in array.iter().take(100) {
        match js::classify(value) {
            Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => {
                if kind != ArrayKind::Empty && kind != ArrayKind::Entries {
                    return ArrayKind::Complex;
                }
                kind = ArrayKind::Entries;
            }
            Kind::Object(_) | Kind::Array(_) => return ArrayKind::Complex,
            Kind::Function(_) => return ArrayKind::Complex,
            Kind::String(value) if value.encode_utf16().count() > 3 => {
                return ArrayKind::Complex;
            }
            _ if kind != ArrayKind::Empty && kind != ArrayKind::Primitive => {
                return ArrayKind::Complex;
            }
            Kind::BigInt(_) => return ArrayKind::Complex,
            _ => kind = ArrayKind::Primitive,
        }
    }
    kind
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+/);
  expect(js).toContain('      if (typeof value === "object" && value !== null) {\n        return "Complex";\n      }\n    } else if (value.length > 3) {\n      return "Complex";\n    }\n');
  const lib = await import(join(dir, "lib.js"));
  const kinds = [[], [1, "ab", true], [["a", 1], ["b", 2]], [1, ["a", 1]], [["a", 1], 1], [{}], [[1, 2]], [() => 1], ["abcd"], [1n], [null, undefined]];
  expect(kinds.map((array) => lib.getArrayKind(array))).toEqual([
    "Empty", "Primitive", "Entries", "Complex", "Complex", "Complex", "Complex", "Complex", "Complex", "Complex", "Primitive",
  ]);
});

// ADR 0754: a `return` of more than one value, a call's, isn't written in
// each path that goes on to it, but once, where the block MIR made ends.
test("a match's return of a call some arms go on to is written once", async () => {
  const dir = fixture("labeled-block-call-exits");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn complex(of: usize) -> u32 {
    if of > 1 { 2 } else { 1 }
}

pub fn get_kind(array: &[Option<&Unknown>]) -> u32 {
    let mut kind = 0;
    for &value in array {
        match js::classify(value) {
            Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => {
                kind = 3;
            }
            Kind::Object(_) | Kind::Array(_) => return complex(array.len()),
            Kind::String(value) if value.encode_utf16().count() > 3 => {
                return 4;
            }
            _ => kind = 5,
        }
    }
    kind
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js.split("return complex(array.length);").length).toBe(2);
  const lib = await import(join(dir, "lib.js"));
  const kinds = [[], ["ab"], [["a", 1]], [{}], [1, [1, 2]], ["abcd"]];
  expect(kinds.map((array) => lib.getKind(array))).toEqual([0, 5, 3, 1, 2, 4]);
});

// React's addValueToProperties (shared/ReactPerformanceTrackProperties.js)
// returns from its Promise case, `if (value.status === 'fulfilled') { ..
// return; } else if (value.status === 'rejected') { .. }`. Its port's
// `return`s, past a `String`'s drop, are one block MIR made that's followed
// by `return;`: each `break` of it that `return;`, and the chain of the
// status's strings is the chain alone, no `typeof status === "string"`
// around it, which no string literal's `===` needs (ADR 0755).
test("a block MIR made followed by a return is each break's return", async () => {
  const dir = fixture("labeled-block-returns");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>, name: &str, out: &mut Vec<String>) {
    let desc = if let Some(value) = value
        && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(value))
    {
        'desc: {
            let mut object_name = name.to_string();
            if out.len() > 5 {
                break 'desc "view".to_string();
            }
            if object_name == "Array"
                && let Kind::Array(array) = js::classify(Some(value))
            {
                if array.is_empty() {
                    break 'desc "empty".to_string();
                } else if array.len() > 3 {
                    for &entry in array.iter().take(2) {
                        describe(entry, "entry", out);
                    }
                    return;
                }
            }
            if object_name == "Promise" {
                match js::classify(js::get(value, "status")) {
                    Kind::String("fulfilled") => {
                        let idx = out.len();
                        describe(js::get(value, "value"), "value", out);
                        if out.len() > idx {
                            out[idx] = "Promise<".to_string() + &out[idx] + ">";
                            return;
                        }
                    }
                    Kind::String("rejected") => {
                        let idx = out.len();
                        describe(js::get(value, "reason"), "reason", out);
                        if out.len() > idx {
                            out[idx] = "Rejected<".to_string() + &out[idx] + ">";
                            return;
                        }
                    }
                    _ => {}
                }
                out.push("Promise".to_string());
                return;
            }
            if object_name == "Object" {
                object_name = "Thing".to_string();
            }
            out.push(object_name);
            return;
        }
    } else {
        "other".to_string()
    };
    out.push(desc);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+/);
  expect(js).not.toContain("typeof status");
  expect(js).toMatch(/\n\s+if \(status === "fulfilled"\) \{\n/);
  expect(js).toMatch(/\n\s+out\[idx\] = "Promise<" \+ out\[idx\] \+ ">";\n\s+return;\n/);
  const lib = await import(join(dir, "lib.js"));
  const of = (value: unknown, name: string, out: string[] = []) => (lib.describe(value, name, out), out);
  expect(of({ status: "fulfilled", value: 1 }, "Promise")).toEqual(["Promise<other>"]);
  expect(of({ status: "fulfilled", value: {} }, "Promise")).toEqual(["Promise<value>"]);
  expect(of({ status: "rejected", reason: {} }, "Promise")).toEqual(["Rejected<reason>"]);
  expect(of({ status: "pending" }, "Promise")).toEqual(["Promise"]);
  expect(of({ status: 1 }, "Promise")).toEqual(["Promise"]);
  expect(of([], "Array")).toEqual(["empty"]);
  expect(of([1, 2, 3, 4], "Array")).toEqual(["other", "other"]);
  expect(of({}, "Object")).toEqual(["Thing"]);
  expect(of({}, "Map", ["a", "b", "c", "d", "e", "f"])).toEqual(["a", "b", "c", "d", "e", "f", "view"]);
});

// ADR 0790: React's addValueToProperties is a `switch (typeof value)`,
// leaving its case early, `desc = ..; break;`. Its port's one `match
// js::classify(value)`: the object arm's guard, `if let Some(value) =
// value`, which holds where the arm's matched, tests nothing, so the
// `match` is a `switch`; and each `break 'desc` the program wrote is where
// it's written, the case's `break;`, an `||`'s too, and one of a `match`'s
// value; no label.
test("a block the program labeled that ends a case is the case's breaks", async () => {
  const dir = fixture("labeled-block-cases");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>, indent: usize, out: &mut Vec<String>) -> String {
    let desc = match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Object(_) | Kind::Array(_) if let Some(value) = value => 'desc: {
            if js::has(value, "type") {
                let type_name = js::string(js::get(value, "type"));
                let keys = js::object::keys(value);
                if keys.len() == 1 {
                    break 'desc "<".to_string() + &type_name + " />";
                }
                if indent < 3 || (keys.len() == 2 && keys[0] == "children") {
                    break 'desc "<".to_string() + &type_name + " \\u{2026} />";
                }
                out.push("<".to_string() + &type_name);
                return String::new();
            }
            let name = js::string(Some(value));
            if js::has(value, "length") {
                break 'desc match js::classify(js::get(value, "length")) {
                    Kind::Number(length) => format!("{name}({})", js::shown(length)),
                    _ => name,
                };
            }
            out.push(name);
            return String::new();
        }
        Kind::Function(_) => "() => {}".to_string(),
        Kind::Undefined => "undefined".to_string(),
        _ => js::string(value),
    };
    desc
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/^\s*\w+: \{$/m);
  expect(js).not.toMatch(/break \w+;/);
  expect(js).not.toContain("value$1");
  // The object arm's `if` the program wrote with no `else`, as written
  // (ADR 0805): not `} else if ("type" in value) { .. } else { .. }`.
  expect(js).toContain(
    '  switch (typeof value) {\n    case "object":\n      if (value === null) {\n        desc = "null";\n      } else {\n        if ("type" in value) {\n',
  );
  expect(js).toContain('          if (keys.length === 1) {\n            desc = "<" + typeName + " />";\n            break;\n          }\n');
  expect(js).toContain(
    '          if (indent < 3 || (keys.length === 2 && keys[0] === "children")) {\n            desc = "<" + typeName + " … />";\n            break;\n          }\n          out.push("<" + typeName);\n          return "";\n        }\n        const name = String(value);\n',
  );
  expect(js).toMatch(/\n\s+if \("length" in value\) \{\n\s+const length = value.length;\n\s+desc = typeof length === "number" \? `\$\{name\}\(\$\{length\}\)` : name;\n\s+break;\n/);
  const lib = await import(join(dir, "lib.js"));
  const of = (value: unknown, indent = 0) => {
    const out: string[] = [];
    return [lib.describe(value, indent, out), ...out];
  };
  expect(of(null)).toEqual(["null"]);
  expect(of(undefined)).toEqual(["undefined"]);
  expect(of(() => 1)).toEqual(["() => {}"]);
  expect(of(5)).toEqual(["5"]);
  expect(of({ type: "a" })).toEqual(["<a />"]);
  expect(of({ type: "a", b: 1 })).toEqual(["<a … />"]);
  expect(of({ type: "a", b: 1 }, 3)).toEqual(["", "<a"]);
  expect(of({ children: 1, type: "a" }, 3)).toEqual(["<a … />"]);
  expect(of([1, 2])).toEqual(["1,2(2)"]);
  expect(of({ length: "x" })).toEqual(["[object Object]"]);
  expect(of({ b: 1 })).toEqual(["", "[object Object]"]);
});

// ADR 0805: React's JSX branch, `if (indent < 3 || (propsLength === 1 &&
// propsKeys[0] === 'children' && key == null)) { desc = ..; break; }`, then
// a loop: the `||` the program wrote whose branch is a `break` as written,
// however many parts it has and whatever its other branch ends with; and
// the arm's `if`, written with no `else`, none, its `then` leaving.
test("an || before a loop whose branch leaves a case is as written", async () => {
  const dir = fixture("labeled-block-or-loops");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn describe(value: Option<&Unknown>, indent: usize, out: &mut Vec<String>) -> String {
    let desc = match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Object(_) | Kind::Array(_) if let Some(value) = value => 'desc: {
            if js::has(value, "type") {
                let type_name = js::string(js::get(value, "type"));
                let keys = js::object::keys(value);
                if keys.is_empty() {
                    break 'desc "<".to_string() + &type_name + " />";
                }
                if indent < 3 || (keys.len() == 2 && keys[0] == "children" && keys[1] == "type") {
                    break 'desc "<".to_string() + &type_name + " \\u{2026} />";
                }
                out.push("<".to_string() + &type_name);
                for key in keys {
                    out.push(key);
                }
                out.push("/>".to_string());
                return String::new();
            }
            let name = js::string(Some(value));
            if js::has(value, "length") {
                break 'desc name + "[]";
            }
            out.push(name);
            return String::new();
        }
        Kind::Undefined => "undefined".to_string(),
        _ => js::string(value),
    };
    desc
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/^\s*\w+: \{$/m);
  expect(js).toContain('        desc = "null";\n      } else {\n        if ("type" in value) {\n');
  expect(js).toContain(
    '          if (indent < 3 || (keys.length === 2 && keys[0] === "children" && keys[1] === "type")) {\n            desc = "<" + typeName + " … />";\n            break;\n          }\n          out.push("<" + typeName);\n          for (const key of keys) {\n',
  );
  expect(js).toContain('          out.push("/>");\n          return "";\n        }\n        const name = String(value);\n');
  const lib = await import(join(dir, "lib.js"));
  const of = (value: unknown, indent = 0) => {
    const out: string[] = [];
    return [lib.describe(value, indent, out), ...out];
  };
  expect(of(null)).toEqual(["null"]);
  expect(of(undefined)).toEqual(["undefined"]);
  expect(of(5)).toEqual(["5"]);
  expect(of({ type: "a" })).toEqual(["<a … />"]);
  expect(of({ type: "a" }, 3)).toEqual(["", "<a", "type", "/>"]);
  expect(of({ children: 1, type: "a" }, 3)).toEqual(["<a … />"]);
  expect(of({ type: "a", children: 1 }, 3)).toEqual(["", "<a", "type", "children", "/>"]);
  expect(of([1, 2])).toEqual(["1,2[]"]);
  expect(of({ b: 1 })).toEqual(["", "[object Object]"]);
});

// ADR 0805: an `else` the program wrote stays where the block it's in ends
// a case, its `else if` a chain (ADR 0683); and one whose `then` goes on,
// a `match`'s, stays, or both its branches would run.
test("an else the program wrote in a case, or after a branch that goes on, stays", async () => {
  const dir = fixture("labeled-block-written-elses");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn written(value: Option<&Unknown>, out: &mut Vec<String>) -> String {
    let desc = match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Object(_) | Kind::Array(_) if let Some(value) = value => 'desc: {
            if js::has(value, "type") {
                let name = js::string(js::get(value, "type"));
                if name.is_empty() {
                    break 'desc "<>".to_string();
                }
                out.push(name);
                return String::new();
            } else {
                let name = js::string(Some(value));
                if js::has(value, "length") {
                    break 'desc name + "[]";
                }
                out.push(name);
                return String::new();
            }
        }
        Kind::Undefined => "none".to_string(),
        _ => js::string(value),
    };
    desc
}

pub fn tail_match(value: Option<&Unknown>, out: &mut Vec<String>) -> String {
    let desc = match js::classify(value) {
        Kind::Null => "null".to_string(),
        Kind::Object(_) | Kind::Array(_) if let Some(value) = value => 'desc: {
            if js::has(value, "type") {
                break 'desc "typed".to_string();
            }
            match js::classify(js::get(value, "kind")) {
                Kind::Number(_) => {
                    out.push("number".to_string());
                    "n".to_string()
                }
                _ => {
                    out.push("other".to_string());
                    "o".to_string()
                }
            }
        }
        Kind::Undefined => "none".to_string(),
        _ => js::string(value),
    };
    desc
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('        desc = "null";\n      } else if ("type" in value) {\n');
  expect(js).toContain(
    '        if (typeof value.kind === "number") {\n          out.push("number");\n          desc = "n";\n        } else {\n          out.push("other");\n          desc = "o";\n        }\n',
  );
  const lib = await import(join(dir, "lib.js"));
  const of = (f: (value: unknown, out: string[]) => string, value: unknown) => {
    const out: string[] = [];
    return [f(value, out), ...out];
  };
  expect(of(lib.written, null)).toEqual(["null"]);
  expect(of(lib.written, { type: "" })).toEqual(["<>"]);
  expect(of(lib.written, { type: "a" })).toEqual(["", "a"]);
  expect(of(lib.written, [1])).toEqual(["1[]"]);
  expect(of(lib.written, {})).toEqual(["", "[object Object]"]);
  expect(of(lib.tailMatch, { type: 1 })).toEqual(["typed"]);
  expect(of(lib.tailMatch, { kind: 1 })).toEqual(["n", "number"]);
  expect(of(lib.tailMatch, { kind: "x" })).toEqual(["o", "other"]);
  expect(of(lib.tailMatch, undefined)).toEqual(["none"]);
});

// ADR 0790: a guard `if let Some(v) = value` holds where the arm's kind
// says `value` is there, only while `value` is what it was: of one the body
// sets, it's tested; and of an arm of `undefined`, it fails.
test("a guard of what a kind says is there is tested where it may fail", async () => {
  const dir = fixture("labeled-block-guards");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn kind(value: Option<&Unknown>) -> u32 {
    match js::classify(value) {
        Kind::Undefined if let Some(_) = value => 9,
        Kind::Object(_) | Kind::Array(_) if let Some(v) = value => js::object::keys(v).len() as u32,
        Kind::Function(_) => 1,
        Kind::Null => 2,
        _ => 3,
    }
}

pub fn reset<'a>(mut value: Option<&'a Unknown>, other: Option<&'a Unknown>) -> u32 {
    match js::classify(value) {
        Kind::Object(_) | Kind::Array(_) if let Some(v) = value => {
            value = other;
            js::object::keys(v).len() as u32 + u32::from(value.is_some())
        }
        Kind::Function(_) => 1,
        _ => 3,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
  const lib = await import(join(dir, "lib.js"));
  expect([undefined, null, {}, { a: 1 }, [1, 2, 3], () => 1, "s"].map((v) => lib.kind(v))).toEqual([3, 2, 0, 1, 3, 1, 3]);
  expect(lib.reset({ a: 1, b: 2 }, undefined)).toBe(2);
  expect(lib.reset({ a: 1 }, { b: 1 })).toBe(2);
  expect(lib.reset(undefined, undefined)).toBe(3);
});

// ADR 0944 (the user's decision): React's createFiberFromTypeAndProps,
// `getTag: switch (type) { .. default: { if (..) { switch (type.$$typeof)
// { case X: tag = ..; break getTag; .. } } tag = Throw; } }`. The program's
// `'get_tag: { match .. }` labels the `switch` its block is, by its
// camelCase name, where a `break` of it is in a `switch` inside a case,
// which a plain `break` would leave instead; one in the case itself is the
// case's `break;`. No block MIR made is left, however MIR placed the
// block: around the `switch` (a case's own `break 'get_tag`), around the
// inner one followed by the case's `break` (none), or by a jump past the
// `if` the `switch` ends (an `else` around it).
test("a label the program wrote is the switch's whose inner switch leaves it", async () => {
  const dir = fixture("labeled-switch");
  const source = (strict: boolean, host: boolean) => `use js::{SymbolFor, Unknown};

const REACT_FRAGMENT_TYPE: SymbolFor = SymbolFor("react.fragment");
const REACT_STRICT_MODE_TYPE: SymbolFor = SymbolFor("react.strict_mode");
const REACT_MEMO_TYPE: SymbolFor = SymbolFor("react.memo");
const REACT_LAZY_TYPE: SymbolFor = SymbolFor("react.lazy");
const REACT_CONTEXT_TYPE: SymbolFor = SymbolFor("react.context");

pub fn tag_of(r#type: Option<&Unknown>, strict: bool) -> &'static str {
    let mut tag = "function";
    if js::type_of(r#type) == "function" {
        tag = "class";
    } else ${host ? `if js::type_of(r#type) == "string" {
        tag = "host";
    } else ` : ""}{
        'get_tag: {
            match SymbolFor::from_value(r#type) {
                Some(REACT_FRAGMENT_TYPE) => return "fragment",
                Some(REACT_STRICT_MODE_TYPE) => {
                    ${strict ? `if strict {
                        tag = "strict";
                        break 'get_tag;
                    }
                    ` : "let _ = strict;\n                    "}tag = "mode";
                }
                _ => {
                    if js::type_of(r#type) == "object" && r#type.is_some() {
                        match SymbolFor::from_value(js::get(r#type, "$$typeof")) {
                            Some(REACT_CONTEXT_TYPE) => {
                                tag = "context";
                                break 'get_tag;
                            }
                            Some(REACT_MEMO_TYPE) => {
                                tag = "memo";
                                break 'get_tag;
                            }
                            Some(REACT_LAZY_TYPE) => {
                                tag = "lazy";
                                break 'get_tag;
                            }
                            _ => {}
                        }
                    }
                    tag = "throw";
                }
            }
        }
    }
    tag
}
`;
  const of = (key: string) => ({ $$typeof: Symbol.for(key) });
  const tags = async (file: string) => {
    const lib = await import(join(dir, file));
    return [
      lib.tagOf(Symbol.for("react.fragment"), false),
      lib.tagOf(Symbol.for("react.strict_mode"), true),
      lib.tagOf(Symbol.for("react.strict_mode"), false),
      lib.tagOf(of("react.context"), false),
      lib.tagOf(of("react.memo"), false),
      lib.tagOf(of("react.lazy"), false),
      lib.tagOf(of("react.other"), false),
      lib.tagOf({}, false),
      lib.tagOf(null, false),
      lib.tagOf(() => 1, false),
      lib.tagOf("div", false),
    ];
  };
  for (const [strict, host] of [
    [true, false],
    [false, false],
    [false, true],
  ]) {
    writeFileSync(join(dir, "lib.rs"), source(strict, host));
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs()]);
    const js = readFileSync(join(dir, "lib.js"), "utf8");
    expect([strict, host, js]).toEqual([strict, host, expect.not.stringMatching(/\bbb\d+/)]);
    expect(js).toContain("    getTag: switch (type) {\n");
    if (strict) {
      expect(js).toContain('        if (strict) {\n          tag = "strict";\n          break;\n        }\n        tag = "mode";\n        break;\n');
    }
    expect(js).toContain('            case REACT_CONTEXT_TYPE:\n              tag = "context";\n              break getTag;\n');
    expect(js).toContain('              tag = "lazy";\n              break getTag;\n          }\n        }\n        tag = "throw";\n    }\n  }\n  return tag;\n');
    expect(await tags(`lib.js?${strict}${host}`)).toEqual([
      "fragment",
      strict ? "strict" : "mode",
      "mode",
      "context",
      "memo",
      "lazy",
      "throw",
      "throw",
      "throw",
      "class",
      host ? "host" : "throw",
    ]);
  }
  // The block around the inner `switch` alone, followed by the case's
  // `break;`, where nothing is around the `match`.
  const bare = source(false, false)
    .replace('    let mut tag = "function";\n    if js::type_of(r#type) == "function" {\n        tag = "class";\n    } else {\n', "    let tag;\n")
    .replace("        }\n    }\n    tag\n}", "        }\n    tag\n}");
  writeFileSync(join(dir, "lib.rs"), bare);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "bare.js"), ...withJs()]);
  const js = readFileSync(join(dir, "bare.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+/);
  expect(js).toContain('  let tag;\n  getTag: switch (type) {\n');
  expect(js).toContain('            tag = "lazy";\n            break getTag;\n        }\n      }\n      tag = "throw";\n  }\n  return tag;\n');
  expect(await tags("bare.js")).toEqual(["fragment", "mode", "mode", "context", "memo", "lazy", "throw", "throw", "throw", "throw", "throw"]);
});
