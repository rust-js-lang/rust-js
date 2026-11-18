// An exported `Result` is declared the value its JS is, `{ TAG: "Ok"; _0:
// T } | { TAG: "Err"; _0: E }`, as an enum's is (ADR 0033), of its type
// arguments, a `js::JsError` what it's declared everywhere, `any`, whatever
// JS threw (#1835): Codex's probe, Intl's `getCanonicalLocales` that keeps
// what it threw.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { es2022, type Setting, typeChecked } from "./type-checked";

beforeAll(buildReact, 600_000);

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, types: [] };

test("an exported Result is declared its tagged union, of its type arguments", async () => {
  const dir = fixture("declarations-results");
  writeFileSync(join(dir, "lib.rs"), `pub fn result(value: bool) -> Result<Vec<String>, &'static js::JsError> {
    if value { Ok(vec![String::from("ok")]) } else { Err(js::error::new(Some("no"), None)) }
}

pub fn generic<T>(value: T, ok: bool) -> Result<T, u32> {
    if ok { Ok(value) } else { Err(1) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export function result(value: boolean): {\n    TAG: "Ok";\n    _0: string[];\n} | {\n    TAG: "Err";\n    _0: unknown;\n};');
  expect(declarations).toContain('export function generic<T>(value: T, ok: boolean): {\n    TAG: "Ok";\n    _0: T;\n} | {\n    TAG: "Err";\n    _0: number;\n};');
  const { result } = await import(join(dir, "lib.js"));
  expect(result(true)).toEqual({ TAG: "Ok", _0: ["ok"] });
  expect(result(false).TAG).toBe("Err");
  writeFileSync(join(dir, "use.ts"), `import { generic, result } from "./lib.js";
const made = result(true);
if (made.TAG === "Ok") {
  const locales: string[] = made._0;
  console.log(locales);
} else {
  // What was thrown is \`unknown\`, narrowed before it's read (#1905).
  const thrown: unknown = made._0;
  if (thrown instanceof Error) console.log(thrown.message);
}
const kept = generic("en", true);
export const tag: "Ok" | "Err" = kept.TAG;
if (kept.TAG === "Ok") {
  const text: string = kept._0;
  console.log(text);
}
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
  // A `Result` isn't its `Ok` value.
  writeFileSync(join(dir, "wrong.ts"), `import { generic } from "./lib.js";
export const text: string = generic("en", true);
`);
  expect(typeChecked(dir, "wrong.ts", checking)).toEqual([expect.stringContaining("error TS2322")]);
});

// A field always `undefined`, a `()`'s, is optional, `_0?: undefined`: the
// JS rust-js makes leaves its key out (ADR 0280), and a JS caller may give
// it, `{ TAG: "Done", _0: undefined }`; a value of each shape is a `Step`
// under `exactOptionalPropertyTypes`, given back to Rust (#1851).
test("a field always undefined is declared optional, true of its key there or not", async () => {
  const dir = fixture("declarations-unit-fields");
  writeFileSync(join(dir, "lib.rs"), `pub enum Step {
    Done(()),
    Next(u32),
}

pub fn step(done: bool) -> Step {
    if done { Step::Done(()) } else { Step::Next(1) }
}

pub fn taken(step: &Step) -> u32 {
    match step {
        Step::Done(()) => 0,
        Step::Next(n) => *n,
    }
}

pub struct Seen;

pub struct Marked {
    pub unit: (),
    pub seen: Seen,
    pub n: u32,
}

pub fn marked(n: u32) -> Marked {
    Marked { unit: (), seen: Seen, n }
}

pub fn finished(ok: bool) -> Result<(), String> {
    if ok { Ok(()) } else { Err("no".to_string()) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('export type Step = {\n    TAG: "Done";\n    _0?: undefined;\n} | {\n    TAG: "Next";\n    _0: number;\n};');
  // A unit struct of no name is `undefined` too, its key there or not.
  expect(declarations).toContain("export interface Marked {\n    unit?: undefined;\n    seen?: any;\n    n: number;\n}");
  expect(declarations).toContain('TAG: "Ok";\n    _0?: undefined;');
  const { step, taken } = await import(join(dir, "lib.js"));
  // The key left out of what rust-js makes; either shape taken back.
  expect(Object.keys(step(true))).toEqual(["TAG"]);
  expect([taken({ TAG: "Done" }), taken({ TAG: "Done", _0: undefined }), taken({ TAG: "Next", _0: 3 })]).toEqual([0, 0, 3]);
  writeFileSync(join(dir, "use.ts"), `import { type Step, step, taken, marked, finished } from "./lib.js";
const left: Step = { TAG: "Done" };
const kept: Step = { TAG: "Done", _0: undefined };
export const both: number = taken(left) + taken(kept) + taken(step(true));
export const unit: undefined = marked(1).unit;
const result = finished(true);
if (result.TAG === "Ok") {
  const nothing: undefined = result._0;
  console.log(nothing);
}
`);
  expect(typeChecked(dir, "use.ts", checking)).toEqual([]);
});
