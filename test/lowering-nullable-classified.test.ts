// An `Option<&Unknown>` a `match` or `if let` of `js::classify` of it
// proved there, on an arm only a value neither `undefined` nor `null`
// takes, `Kind::Function(_)`'s, is given as it is: its `unwrap()` no
// `$unwrap`, a nullable exit no `?? null` (#1933, `proved_present`,
// MIR's one answer of it). React's Flight register's `if let
// Kind::Function(_) = js::classify(exports) { register(exports.unwrap()) }`.
// Not on an arm `Kind::Null` or `Kind::Undefined` shares, nor one past a
// write, nor of a test of another value; `otherwise` where both are named
// elsewhere. A `js::cast` of a nullable field, which in JS is the field
// itself, held in a `let`: tested as the field is, `=== null`
// (`gives_its_option`), as React's `updateSuspenseOffscreenState` reads
// `prevCachePool`. Each compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const compile = (name: string, source: string) => {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const counts = join(dir, "decisions.tsv");
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], 120_000, { RUST_JS_DECISIONS: counts });
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const fn = (name: string) => js.slice(js.indexOf(`export function ${name}(`), js.indexOf("\n}\n", js.indexOf(`export function ${name}(`)) + 2);
  const tidied = readFileSync(counts, "utf8")
    .split("\n")
    .map((line) => line.split("\t"))
    .filter((cells) => cells[0]!.startsWith("tidy-") && Number(cells[4]) > 0)
    .map((cells) => `${cells[0]} ${cells[1]}`);
  return { dir, fn, tidied };
};

test("an Option a classify arm neither undefined nor null takes is given as it is", async () => {
  const { dir, fn, tidied } = compile(
    "nullable-classified",
    `use js::{Kind, Unknown};

fn kept(f: &'static Unknown) -> Vec<&'static Unknown> {
    vec![f]
}

pub fn function_arm(x: Option<&'static Unknown>) -> Vec<&'static Unknown> {
    if let Kind::Function(_) = js::classify(x) {
        return kept(x.unwrap());
    }
    Vec::new()
}

pub fn matched_arms(x: Option<&'static Unknown>) -> Vec<&'static Unknown> {
    match js::classify(x) {
        Kind::Function(_) => kept(x.unwrap()),
        Kind::String(_) => kept(x.unwrap()),
        _ => Vec::new(),
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn object_arm(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if let Kind::Object(_) = js::classify(x) {
        return x;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn otherwise_arm(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match js::classify(x) {
        Kind::Undefined | Kind::Null => None,
        _ => x,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn shared_with_null(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match js::classify(x) {
        Kind::Function(_) | Kind::Null => x,
        _ => None,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn undefined_left(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match js::classify(x) {
        Kind::Null => None,
        _ => x,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(info: &'static Unknown, again: bool) -> Option<&'static Unknown> {
    let mut x = js::get(Some(info), "value");
    if let Kind::Function(_) = js::classify(x) {
        if again {
            x = js::get(Some(info), "other");
        }
        return x;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn another(x: Option<&'static Unknown>, y: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if let Kind::Function(_) = js::classify(y) {
        return x;
    }
    None
}
`,
  );
  expect(tidied).toEqual([]);
  for (const name of ["functionArm", "matchedArms"]) expect([name, fn(name).includes("kept(x)"), fn(name).includes("$unwrap")]).toEqual([name, true, false]);
  expect(fn("objectArm")).toContain("return x;");
  expect(fn("otherwiseArm")).not.toContain("?? null");
  for (const name of ["sharedWithNull", "undefinedLeft", "rewritten", "another"]) expect([name, fn(name).includes("?? null")]).toEqual([name, true]);
  const lib = await import(join(dir, "lib.js"));
  const f = () => 7;
  const o = { a: 1 };
  expect([lib.functionArm(f), lib.functionArm(null), lib.functionArm(undefined), lib.functionArm(o)]).toEqual([[f], [], [], []]);
  expect([lib.matchedArms(f), lib.matchedArms("a"), lib.matchedArms(null), lib.matchedArms(undefined), lib.matchedArms(3)]).toEqual([[f], ["a"], [], [], []]);
  expect([lib.objectArm(o), lib.objectArm(f), lib.objectArm(undefined), lib.objectArm(null)]).toEqual([o, null, null, null]);
  expect([lib.otherwiseArm(o), lib.otherwiseArm(undefined), lib.otherwiseArm(null)]).toEqual([o, null, null]);
  expect([lib.sharedWithNull(f), lib.sharedWithNull(null), lib.sharedWithNull(undefined)]).toEqual([f, null, null]);
  expect([lib.undefinedLeft(o), lib.undefinedLeft(undefined), lib.undefinedLeft(null)]).toEqual([o, null, null]);
  expect([lib.rewritten({ value: f }, false), lib.rewritten({ value: f }, true), lib.rewritten({}, false)]).toEqual([f, null, null]);
  expect([lib.another(o, f), lib.another(undefined, f), lib.another(o, null)]).toEqual([o, null, null]);
});

test("a js::cast of a nullable field held in a let is tested as the field is", async () => {
  const { dir, fn } = compile(
    "nullable-cast-held",
    `use js::Unknown;

pub struct Pool {
    pub n: u32,
}

pub struct State {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pool: Option<&'static Unknown>,
}

fn pooled(state: &State) -> u32 {
    let pool: Option<&Pool> = unsafe { js::cast(state.pool) };
    if let Some(pool) = pool { pool.n } else { 0 }
}

pub fn run(pool: Option<&'static Unknown>) -> u32 {
    let state = State { pool };
    pooled(&state)
}
`,
  );
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("pool !== null");
  expect(fn("run")).not.toContain("== null");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.run({ n: 4 }), lib.run(undefined), lib.run(null)]).toEqual([4, 0, 0]);
});
