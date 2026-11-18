// A Rust validator as a TypeScript type predicate (#1828, ADR 1155):
// `js::Is<T>`, made only by `js::Is::when`, which tests the value is a `T`
// before its own test, declared `value is T` of the parameter its MIR
// proves it's given, one TypeScript types `unknown`. Upstream
// (vercel-labs/ui@220c1a0, packages/core/src/foundation/date/plain-date.ts)
// is `function isISODate(value: unknown): value is ISODate`, and Codex's
// consumer (rust-js-lang/vercel-ui-sdk-in-rust@da281b6,
// tests/types/calendar.ts) type-checks against the port's.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { es2022, type Setting, typeChecked } from "./type-checked";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

// What this file's checks of its declarations are made under (`typeChecked`).
const checking: Setting = { ...es2022, types: [] };

const program = `#[cfg_attr(rust_js, rust_js::name = "ISODate")]
pub type IsoDate = String;

/// Reads a date's year, or None for an invalid date.
pub fn year(date: Option<&str>) -> Option<f64> {
    let date = date?;
    let (year, rest) = date.split_once('-')?;
    (rest.len() == 5).then(|| year.parse().ok()).flatten()
}

/// Whether a value is an ISO date.
#[cfg_attr(rust_js, rust_js::name = "isISODate")]
pub fn is_iso_date(value: Option<&js::Unknown>) -> js::Is<IsoDate> {
    js::Is::when(value, |text| year(Some(text)).is_some())
}

pub fn counted(values: &[Option<&js::Unknown>]) -> usize {
    values.iter().filter(|value| is_iso_date(**value).holds()).count()
}
`;

test("a js::Is is upstream's predicate, its JS and its declaration", async () => {
  const dir = fixture("js-predicates");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('export function isISODate(value) {\n  return typeof value === "string" && year(value) != null;\n}');
  expect(js).toContain("isISODate(value)");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function isISODate(value?: unknown): value is ISODate;");
  // Rust's answers: each a date of a string, never of another kind.
  const { counted, isISODate } = await import(join(dir, "lib.js"));
  expect([isISODate("2024-02-29"), isISODate("2024-2-29"), isISODate(2024), isISODate(undefined), isISODate(null)]).toEqual([true, false, false, false, false]);
  expect(counted(["2024-02-29", 3, "x", "1999-12-31", null])).toBe(2);
  // Codex's consumer, as written but its import.
  writeFileSync(join(dir, "calendar.ts"), `import {isISODate, type ISODate} from "./lib.js";
declare const value: unknown;
if (isISODate(value)) {
  const date: string = value;
  console.log(date);
}
const typed: ISODate = "2024-02-29";
console.log(typed);
`);
  expect(typeChecked(dir, "calendar.ts", checking)).toEqual([]);
  // The false branch narrows nothing of an `unknown`: what isn't a date
  // may be any value, a string too, so it isn't a number there.
  writeFileSync(join(dir, "otherwise.ts"), `import {isISODate} from "./lib.js";
declare const value: unknown;
if (!isISODate(value)) {
  const text: string = value;
  console.log(text);
}
`);
  expect(typeChecked(dir, "otherwise.ts", checking)).toEqual([expect.stringContaining("error TS2322")]);
});

// Each `js::Is::when` is given a parameter of its function, as it was
// passed, one TypeScript types `unknown`, or it's refused saying what to
// write: of another value a TypeScript predicate's false branch would say
// what isn't so.
test("a js::Is of what isn't an unknown parameter as passed is refused, saying what to write", () => {
  const dir = fixture("js-predicates-refused");
  const refusals: [string, string][] = [
    // Of a `&str` made `unknown`: its false branch, `string | number` less
    // `string`, would say a non-date string is a number.
    [`pub fn is_date(value: &str) -> js::Is<String> {
    js::Is::when(Some(js::unknown(value)), |text: &str| !text.is_empty())
}`, "`js::Is::when` guards a parameter of the function that gives it"],
    // Of a parameter changed first.
    [`pub fn is_date(mut value: Option<&'static js::Unknown>) -> js::Is<String> {
    if value.is_none() {
        value = Some(js::unknown("x"));
    }
    js::Is::when(value, |text: &str| !text.is_empty())
}`, "`js::Is::when` guards a parameter of the function that gives it"],
    // Of two parameters.
    [`pub fn is_date(a: Option<&js::Unknown>, b: Option<&js::Unknown>) -> js::Is<String> {
    if js::Is::<String>::when(a, |text: &str| text.is_empty()).holds() {
        return js::Is::when(b, |text: &str| !text.is_empty());
    }
    js::Is::when(a, |text: &str| !text.is_empty())
}`, "`js::Is::when` guards a parameter of the function that gives it"],
    // Given by another function: its declaration couldn't say of which.
    [`fn made(value: Option<&js::Unknown>) -> js::Is<String> {
    js::Is::when(value, |text: &str| !text.is_empty())
}

pub fn is_date(value: Option<&js::Unknown>) -> js::Is<String> {
    made(value)
}`, "a function that gives a `js::Is` makes it here"],
  ];
  for (const [body, refusal] of refusals) {
    writeFileSync(join(dir, "lib.rs"), `${body}\n`);
    const refused = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], { cwd: root, stderr: "pipe", timeout: 60_000 });
    const stderr = refused.stderr.toString();
    expect([body, refused.exitCode === 0, stderr.includes(refusal)]).toEqual([body, false, true]);
    expect(stderr).toContain("a `bool`, declared `boolean`");
  }
});

test("a trusted predicate on strings narrows both branches", async () => {
  const dir = fixture("js-typed-predicate");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::types = "\\"primary\\" | \\"secondary\\"")]
pub type Selection = &'static str;

const SELECTIONS: &[&str] = &["primary", "secondary"];

pub fn is_selection(value: &str) -> js::Is<Selection> {
    // SAFETY: this list includes exactly the two declared names.
    unsafe { js::Is::assert(value, |input| SELECTIONS.contains(&input)) }
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("export function isSelection(value: string): value is Selection;");
  expect(readFileSync(join(dir, "lib.js"), "utf8")).not.toContain("typeof value");
  const {isSelection} = await import(join(dir, "lib.js"));
  expect(["primary", "secondary", "other", null].map(isSelection)).toEqual([true, true, false, false]);
  writeFileSync(join(dir, "consumer.ts"), `import {isSelection, type Selection} from "./lib.js";
declare const value: "primary" | "secondary" | "other";
if (isSelection(value)) {
  const selection: Selection = value;
} else {
  const other: "other" = value;
}
`);
  expect(typeChecked(dir, "consumer.ts", checking)).toEqual([]);
});
