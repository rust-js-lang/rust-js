// An `Option` a test of it proved there, read where every path took the
// test's edge `None` never takes, is given as it is: no `?? null` at a
// nullable exit, no `$unwrap` of its `unwrap()` (#1933, `proved_present`,
// MIR's one answer of it). The tests: `is_some()`'s true edge,
// `is_none()`'s false one, a `match` or `if let` of it, `Some`'s arm,
// `js::type_of(x) == "function"` and `js::truthy` (test/lowering-nullable-
// truthy.test.ts). Each beside a second condition, `&& flag`, which no
// test of the printed JS reads as saying it. Each compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// Compiled counting its decisions: what tidy's tests of the printed JS
// answered of these, `tidy-present-in` and `tidy-nulled-as-they-are`,
// which MIR's answer leaves nothing to (#1933's step 0 counters).
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

test("an Option is_some, is_none, a match and a typeof of \"function\" proved there is given as it is", async () => {
  const compiled = compile(
    "nullable-tests",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn some(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if value.is_some() && flag {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn none(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if value.is_none() || !flag {
        return None;
    }
    value
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn matched(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if let Some(_) = value
        && flag
    {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn arms(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    match value {
        None => None,
        _ if flag => value,
        _ => None,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn function(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if js::type_of(value) == "function" && flag {
        return value;
    }
    None
}

pub fn unwrapped(value: Option<&'static Unknown>, flag: bool) -> &'static Unknown {
    if js::truthy(value) && flag {
        return value.unwrap();
    }
    js::unknown(&0.0)
}
`,
  );
  const { dir, fn } = compiled;
  const { tidied } = compiled;
  expect(tidied).toEqual([]);
  for (const name of ["some", "none", "matched", "arms", "function"]) expect([name, fn(name).includes("??")]).toEqual([name, false]);
  expect(fn("unwrapped")).not.toContain("$unwrap");
  const lib = await import(join(dir, "lib.js"));
  const one = { id: 1 };
  const f = () => 1;
  for (const name of ["some", "none", "matched", "arms"]) {
    expect([name, lib[name](one, true), lib[name](one, false), lib[name](undefined, true), lib[name](null, true)]).toEqual([name, one, null, null, null]);
  }
  expect([lib.function(f, true), lib.function(f, false), lib.function(one, true), lib.function(undefined, true), lib.function(null, true)]).toEqual([f, null, null, null, null]);
  expect([lib.unwrapped(one, true), lib.unwrapped(one, false), lib.unwrapped(undefined, true)]).toEqual([one, 0, 0]);
});

// What a test says of its edge alone: `is_none()`'s true edge, a typeof of
// a kind `null` has too, `"object"`, and a value written again after the
// test, keep their `?? null`, as each may be `None` there.
test("an Option's is_none true edge, typeof of \"object\" and a value written after the test aren't proved there", async () => {
  const { dir, fn } = compile(
    "nullable-tests-not",
    `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn none(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if value.is_none() && flag {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn object(value: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    if js::type_of(value) == "object" && flag {
        return value;
    }
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn rewritten(first: Option<&'static Unknown>, second: Option<&'static Unknown>, flag: bool) -> Option<&'static Unknown> {
    let mut found = first;
    if found.is_some() && flag {
        found = second;
        return found;
    }
    None
}
`,
  );
  for (const name of ["none", "object", "rewritten"]) expect([name, fn(name).includes("?? null")]).toEqual([name, true]);
  const lib = await import(join(dir, "lib.js"));
  const one = { id: 1 };
  expect([lib.none(undefined, true), lib.none(null, true), lib.none(one, true)]).toEqual([null, null, null]);
  expect([lib.object(null, true), lib.object(one, true), lib.object(undefined, true)]).toEqual([null, one, null]);
  expect([lib.rewritten(one, undefined, true), lib.rewritten(one, one, true)]).toEqual([null, one]);
});

// A read a test proved there is so only there: one tidy moves past the
// test, `x == null ? null : x` made `x ?? null` or two arms alike made one,
// is what it is where it's read now, its `None` `null` at a nullable exit
// (#1942: `if js::truthy(x) { x } else { x }` returned `undefined`). Of the
// program's build and the library's.
test("what a test proved of a read isn't moved past the test", async () => {
  const source = `use js::Unknown;

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn matched(deps: Option<&'static Unknown>) -> Option<&'static Unknown> {
    match deps {
        None => js::null(),
        deps => deps,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn some(deps: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if deps.is_some() { deps } else { js::null() }
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn alike(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let kept = if js::truthy(x) { x } else { x };
    kept
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn alike_some(x: Option<&'static Unknown>) -> Option<&'static Unknown> {
    let kept = if x.is_some() { x } else { x };
    kept
}
`;
  for (const library of [false, true]) {
    const dir = fixture(library ? "nullable-tests-moved-library" : "nullable-tests-moved");
    writeFileSync(join(dir, "lib.rs"), source);
    const mode = library ? ["--library", "--manifest", join(dir, "lib.manifest.json")] : [];
    run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...mode, "--", "--crate-name", "moved", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
    const lib = await import(join(dir, "lib.js"));
    const one = { id: 1 };
    for (const name of ["matched", "some", "alike", "alikeSome"]) {
      expect([library, name, lib[name](undefined), lib[name](null), lib[name](one)]).toEqual([library, name, null, null, one]);
    }
  }
});

// What a test proved of a read is no other knowledge of it: an `Option`
// made of it where it's tested, `kind.object()`'s `t ? v : undefined`, is
// there where `t` holds, its test `t` (ADR 0910), as of any read (React
// DOM's hydration of an object `src`).
test("a proved read's object, made where it's tested, is tested by that test", async () => {
  const { dir, fn } = compile(
    "nullable-tests-object",
    `use js::Unknown;

fn used(_: &Unknown) {}

pub fn shown(value: Option<&'static Unknown>, flag: bool) -> &'static str {
    if value.is_some() {
        if flag && let Some(object) = js::classify(value).object() {
            used(js::unknown(object));
            return "object";
        }
    }
    "other"
}
`,
  );
  expect(fn("shown")).toContain(`if (flag && typeof value === "object" && value !== null) {`);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.shown({}, true), lib.shown(null, true), lib.shown(1, true), lib.shown({}, false)]).toEqual(["object", "other", "other", "other"]);
});
