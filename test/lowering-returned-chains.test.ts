// A function's value of an `else if` chain: one nested conditional where
// every arm is a plain value, a string or a number literal or a name, and
// otherwise the whole chain, each arm returning its own, never half a chain
// and half a conditional (ADR 0969, amended, the user's decision of
// React's rd23). Each test is a program of its own, compiled and run.

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

// React DOM's ReactDOMUnknownPropertyHook describes a value by a chain of
// four arms, the last a template: `return thing === null ? .. : ..`
// upstream, whose arms the user decided by what they are. One that's more
// than a plain value keeps the whole chain, each arm its `return`; every arm
// plain is one conditional; two arms are one conditional as before.
test("a function's value of an else-if chain is one conditional of plain arms, or the whole chain", async () => {
  const { js, lib } = await compiled(
    "returned-chains",
    `use js::{Kind, Unknown};

pub fn describe(thing: Option<&Unknown>) -> String {
    if matches!(js::classify(thing), Kind::Null) {
        "\`null\`".to_owned()
    } else if matches!(js::classify(thing), Kind::Undefined) {
        "\`undefined\`".to_owned()
    } else if matches!(js::classify(thing), Kind::String("")) {
        "an empty string".to_owned()
    } else {
        format!("something with type \\"{}\\"", js::type_of(thing))
    }
}

pub fn named(n: i32) -> &'static str {
    if n == 0 {
        "zero"
    } else if n == 1 {
        "one"
    } else {
        "many"
    }
}

pub fn signed(n: i32, fallback: i32) -> i32 {
    if n < 0 {
        -1
    } else if n > 9 {
        fallback
    } else if n == 5 {
        5
    } else {
        0
    }
}

pub fn first(n: i32) -> String {
    if n < 0 {
        format!("{n} below")
    } else if n == 0 {
        "zero".to_owned()
    } else if n == 1 {
        "one".to_owned()
    } else {
        "many".to_owned()
    }
}

pub fn three(n: i32) -> String {
    if n < 0 {
        format!("{n} below")
    } else if n == 0 {
        "zero".to_owned()
    } else {
        "many".to_owned()
    }
}

pub fn two(n: i32) -> String {
    if n == 0 {
        "zero".to_owned()
    } else {
        format!("many {n}")
    }
}

pub fn nested(n: i32, m: i32) -> i32 {
    if n == 0 {
        1
    } else if m == 1 {
        if n > 3 { 2 } else { 3 }
    } else {
        4
    }
}
`,
  );
  expect(js).toContain(`export function describe(thing) {
  if (thing === null) {
    return "\`null\`";
  } else if (thing === undefined) {
    return "\`undefined\`";
  } else if (thing === "") {
    return "an empty string";
  } else {
    return \`something with type "\${typeof thing}"\`;
  }
}`);
  expect(js).toContain(`export function named(n) {
  return n === 0 ? "zero" : n === 1 ? "one" : "many";
}`);
  expect(js).toContain(`export function signed(n, fallback) {
  return n < 0 ? -1 : n > 9 ? fallback : n === 5 ? 5 : 0;
}`);
  expect(js).toContain(`export function first(n) {
  if (n < 0) {
    return \`\${n} below\`;
  } else if (n === 0) {
    return "zero";
  } else if (n === 1) {
    return "one";
  } else {
    return "many";
  }
}`);
  expect(js).toContain(`export function three(n) {
  if (n < 0) {
    return \`\${n} below\`;
  } else if (n === 0) {
    return "zero";
  } else {
    return "many";
  }
}`);
  expect(js).toContain(`export function two(n) {
  return n === 0 ? "zero" : \`many \${n}\`;
}`);
  expect(js).toContain(`export function nested(n, m) {
  if (n === 0) {
    return 1;
  } else if (m === 1) {
    return n > 3 ? 2 : 3;
  } else {
    return 4;
  }
}`);
  const describe = lib.describe as (thing: unknown) => string;
  expect([null, undefined, "", 3].map(describe)).toEqual([
    "`null`",
    "`undefined`",
    "an empty string",
    'something with type "number"',
  ]);
  expect([0, 1, 2].map(lib.named as (n: number) => string)).toEqual(["zero", "one", "many"]);
  expect([-3, 12, 5, 4].map((n) => (lib.signed as (n: number, f: number) => number)(n, 7))).toEqual([-1, 7, 5, 0]);
  expect([-2, 0, 1, 3].map(lib.first as (n: number) => string)).toEqual(["-2 below", "zero", "one", "many"]);
  expect([-2, 0, 3].map(lib.three as (n: number) => string)).toEqual(["-2 below", "zero", "many"]);
  expect([0, 4].map(lib.two as (n: number) => string)).toEqual(["zero", "many 4"]);
  expect([0, 5, 2, 7].map((n, i) => (lib.nested as (n: number, m: number) => number)(n, i === 3 ? 0 : 1))).toEqual([1, 2, 3, 4]);
});

// rd23b, the user's rule (pt3): after a link an `else if let` binds first,
// `const s = ..; if (s != null)`, which ends JS's chain, the rest is a chain
// of its own, one nested conditional of plain arms, else the chain as
// written, not its last two links' conditional after an `if` of the first.
test("the rest of a returned chain after an else if let's binding is a chain of its own", async () => {
  const { js, lib } = await compiled("returned-chains-bound", `pub fn describe(x: Option<&'static str>, n: u32) -> &'static str {
    if n == 0 {
        "zero"
    } else if let Some(s) = x.and_then(|x| x.strip_prefix("#")) {
        s
    } else if n > 5 {
        "big"
    } else {
        "small"
    }
}

pub fn named(x: Option<&'static str>, n: u32) -> String {
    if n == 0 {
        "zero".to_owned()
    } else if let Some(s) = x.and_then(|x| x.strip_prefix("#")) {
        format!("<{s}>")
    } else if n > 5 {
        "big".to_owned()
    } else {
        format!("{n} small")
    }
}`);
  expect(js).toContain('  return s != null ? s : n > 5 ? "big" : "small";\n');
  expect(js).toContain(
    '  if (s != null) {\n    return `<${s}>`;\n  } else if (n > 5) {\n    return "big";\n  } else {\n    return `${n} small`;\n  }\n',
  );
  const describe = lib.describe as (x: string | undefined, n: number) => string;
  const named = lib.named as (x: string | undefined, n: number) => string;
  expect([describe("#a", 0), describe("#a", 9), describe("a", 9), describe(undefined, 2)]).toEqual(["zero", "a", "big", "small"]);
  expect([named("#a", 3), named(undefined, 9), named("b", 2)]).toEqual(["<a>", "big", "2 small"]);
});

// React DOM's isSuspenseInstanceFallback, `return instance.data === "$!" ||
// (instance.data === "$?" && instance.ownerDocument.readyState !==
// "loading")`: each test a getter's read, which MIR holds in a temporary
// apart from the `if (a) { return true; }` it makes when `logical returns`
// runs, read in place after; one value, as written, not `if (..) { return
// true; } return .. && ..;` (rd179).
test("a returned || of getters' reads is returned as written", () => {
  const dir = fixture("returned-or-of-getters");
  writeFileSync(join(dir, "lib.rs"), `use webapi::Comment;

pub const FALLBACK: &str = "$!";
pub const PENDING: &str = "$?";
pub const LOADING: &str = "loading";

pub fn is_fallback(instance: &Comment) -> bool {
    instance.data() == FALLBACK || (instance.data() == PENDING && instance.owner_document().ready_state() != LOADING)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type", "lib", "--extern", `js=${join(target, "libjs.rmeta")}`, "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain(
    "  return (\n    instance.data === FALLBACK ||\n    (instance.data === PENDING && instance.ownerDocument.readyState !== LOADING)\n  );\n",
  );
});
