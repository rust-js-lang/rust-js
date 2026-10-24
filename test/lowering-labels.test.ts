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
  expect(js).not.toMatch(/bb\d+/);
  const lib = await import(join(dir, "lib.js"));
  const out: string[] = [];
  const got = [{}, { abcdef: 1, ghijkl: 2 }, { a: 1, b: 2 }, { c: 1 }, null].map((v) => lib.describe(v, 5, out));
  expect(got).toEqual(["empty", "abcdef,ghijkl", "deep", "", "other"]);
  expect(out).toEqual(["c"]);
});

// ADR 0745: React's addObjectDiffToProperties is upstream's chain of `if`s,
// each arm's `continue` where it's written, with no block MIR made; and
// addValueToProperties' `'desc` block, which ends a branch of one MIR made,
// is still `desc: { .. }`, not merged into that one.
test("React's addObjectDiffToProperties has no block MIR made", () => {
  const of = performanceTrack();
  const diff = of("addObjectDiffToProperties");
  expect(diff).not.toMatch(/bb\d+/);
  expect(diff).toContain('          properties.push([REMOVED + line, "…"], [ADDED + line, "…"]);\n          isDeeplyEqual = false;\n          continue;\n        }\n');
  expect(diff).toContain("              if (!nestedEqual) {\n                isDeeplyEqual = false;\n              } else if (");
  expect(of("addValueToProperties")).toMatch(/\n\s+desc: \{\n/);
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
  expect(js).not.toMatch(/bb\d+/);
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
  expect(js).not.toMatch(/bb\d+/);
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
