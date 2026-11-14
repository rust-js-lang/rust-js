// Rust has one `None`, and where it leaves for JS as `undefined` alone, JS
// tells it from `null` (#1840, #1843; ADR 0030, amended): a native optional
// argument takes `undefined` for none and `null` for a value, `(1.25)
// .toPrecision(null)` a RangeError where `toPrecision()` is "1.25", and a
// typed caller tests what a function JS calls returns `=== undefined`. An
// `Option` JS gives may hold either, as its declaration admits, and so may
// a nullable place, whose `None` is `null`. So each exit, a binding's
// optional argument, a function JS calls's return, a struct's plain
// `Option` field written and an argument a callback JS may have given
// takes, gives `value ?? undefined` where what it's given may hold JS's
// `null` (`nulled_params::JsNones`), and as it is where it holds Rust's.
// Each answers as the literal `None` does, in Bun and in Chromium. An
// `Option` of a value of any shape, `Option<&Unknown>`, holds JS's
// `undefined` and `null` each as a value of its own, `String(null)`'s
// "null", passed on as given.

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";
import { buildReact, compiler, fixture, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `pub fn digits(value: Option<f64>) -> String {
    js::number::to_precision(1.25, value)
}

pub fn digits_none() -> String {
    js::number::to_precision(1.25, None)
}

pub fn cut(text: &str, end: Option<f64>) -> String {
    js::string::slice(text, 0.0, end)
}

fn helper(value: Option<f64>) -> String {
    js::number::to_precision(2.5, value)
}

pub fn through(value: Option<f64>) -> String {
    helper(value)
}

pub fn made() -> String {
    helper(Some(3.0)) + &helper(None)
}

pub fn shown(value: Option<&js::Unknown>) -> String {
    js::string(value)
}

pub struct Given {
    pub value: Option<f64>,
}

pub fn of_field(given: &Given) -> String {
    js::number::to_precision(1.25, given.value)
}

pub fn passed(value: Option<f64>) -> Option<f64> {
    value
}

pub fn fresh(n: f64) -> Option<f64> {
    if n > 0.0 { Some(n) } else { None }
}

pub fn empty(value: Option<f64>) -> bool {
    value.is_none()
}

pub struct Shown {
    pub digits: Option<f64>,
    pub label: String,
}

pub fn make(digits: Option<f64>) -> Shown {
    Shown {
        digits,
        label: "made".to_string(),
    }
}

pub fn set(shown: &mut Shown, digits: Option<f64>) {
    shown.digits = digits;
}

pub fn call(f: &dyn Fn(Option<f64>) -> bool, value: Option<f64>) -> bool {
    f(value)
}

#[cfg_attr(rust_js, rust_js::nullable(value))]
pub fn forwarded(value: Option<f64>) -> String {
    js::number::to_precision(1.25, value)
}

pub fn forwarded_none() -> String {
    forwarded(None)
}

pub trait Digits {
    fn digits(&self, value: Option<f64>) -> String;
}

pub struct Plain;

impl Digits for Plain {
    fn digits(&self, value: Option<f64>) -> String {
        js::number::to_precision(1.25, value)
    }
}

pub fn through_dyn(value: Option<f64>) -> String {
    let digits: &dyn Digits = &Plain;
    digits.digits(value)
}

pub fn through_js_dyn(digits: &dyn Digits, value: Option<f64>) -> String {
    digits.digits(value)
}

pub fn bound(value: Option<f64>) -> String {
    let shown = js::function::bind(js::number::to_precision, None, (1.25, value));
    shown()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.givenDigits")]
fn given_digits() -> Option<f64> {
    unreachable!()
}

pub fn from_js() -> String {
    js::number::to_precision(1.25, given_digits())
}

fn id(value: Option<f64>) -> Option<f64> {
    value
}

pub fn returned(value: Option<f64>) -> String {
    let given = id(value);
    js::number::to_precision(1.25, given)
}

pub fn first(values: Vec<Option<f64>>) -> String {
    js::number::to_precision(1.25, values[0])
}

pub fn from_callback(f: &dyn Fn() -> Option<f64>) -> String {
    js::number::to_precision(1.25, f())
}

fn fill(slot: &mut Option<f64>, value: Option<f64>) {
    *slot = value;
}

pub fn filled(value: Option<f64>) -> String {
    let mut slot = None;
    fill(&mut slot, value);
    js::number::to_precision(1.25, slot)
}

pub fn copied(value: &Option<f64>) -> String {
    js::number::to_precision(1.25, value.as_ref().copied())
}

pub fn stored(object: &js::Unknown, value: Option<f64>) {
    js::set(object, "k", value);
}

pub fn rust_callback(value: Option<f64>) -> bool {
    let shown: &dyn Fn(Option<f64>) -> bool = &|value| value.is_none();
    shown(value)
}

pub fn explicit() -> String {
    js::string::slice("abc", 0.0, js::null())
}

pub fn remade(shown: Shown, digits: Option<f64>) -> Shown {
    Shown { digits, ..shown }
}

pub fn passed_on(value: Option<f64>) -> Option<f64> {
    id(value)
}

pub fn this_of(f: &js::Function, this: Option<&'static js::Unknown>) -> Option<&'static js::Unknown> {
    js::function::call(f, this.map(|this| this as &dyn js::IntoUnknown), ())
}

pub fn call_pointer(f: fn(Option<f64>) -> bool, value: Option<f64>) -> bool {
    f(value)
}
`;

/** Each function's answers of JS's two `None`s, and Rust's own. */
const answers = (m: Record<string, (...args: unknown[]) => any>) => {
  const each = (f: () => unknown) => {
    try {
      return f();
    } catch (e) {
      return `throws ${(e as Error).constructor.name}`;
    }
  };
  const shown = { digits: 1, label: "x" };
  return [
    [each(() => m.digits(null)), each(() => m.digits(undefined)), each(() => m.digitsNone())],
    [each(() => m.cut("abc", null)), each(() => m.cut("abc", undefined)), each(() => m.cut("abc", 1))],
    [each(() => m.through(null)), each(() => m.made())],
    [each(() => m.shown(null)), each(() => m.shown(undefined))],
    [each(() => m.ofField({ value: null })), each(() => m.ofField({}))],
    [each(() => m.passed(null) === undefined), each(() => m.fresh(0) === undefined), each(() => m.empty(null))],
    [each(() => m.make(null).digits === undefined), each(() => (m.set(shown, null), shown.digits === undefined))],
    [each(() => m.call((v: unknown) => v === undefined, null))],
    [each(() => m.forwarded(null)), each(() => m.forwardedNone())],
    [
      each(() => m.throughDyn(null)),
      each(() => m.throughJsDyn({ impl: { digits: (_: unknown, v: unknown) => String(v === undefined) } }, null)),
      each(() => m.bound(null)),
    ],
    [(((globalThis as Record<string, unknown>).givenDigits = () => null), each(() => m.fromJs())), each(() => m.returned(null)), each(() => m.first([null]))],
    [each(() => m.fromCallback(() => null)), each(() => m.filled(null)), each(() => m.copied(null))],
    [each(() => m.rustCallback(null)), each(() => m.explicit())],
    [each(() => m.remade({ digits: 1, label: "x" }, null).digits === undefined), each(() => m.passedOn(null) === undefined)],
    [
      each(() => m.thisOf(class { static isNull() { return this === null; } }.isNull, null)),
      each(() => m.callPointer((v: unknown) => v === undefined, null)),
    ],
  ];
};

const rust = [
  ["1.25", "1.25", "1.25"],
  ["abc", "abc", "a"],
  ["2.5", "2.502.5"],
  ["null", "undefined"],
  ["1.25", "1.25"],
  [true, true, true],
  [true, true],
  [true],
  ["1.25", "1.25"],
  ["1.25", "true", "1.25"],
  ["1.25", "1.25", "1.25"],
  ["1.25", "1.25", "1.25"],
  [true, ""],
  [true, true],
  [true, true],
];

test("a None leaving for JS as undefined alone is Rust's, in Bun and in Chromium", async () => {
  const dir = fixture("none-given");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  // Each exit of what JS may have given: a binding's optional argument,
  // what a function JS calls returns, a field written, a callback's.
  expect(out).toContain("export function digits(value) {\n  return (1.25).toPrecision(value ?? undefined);\n}");
  expect(out).toContain("export function passed(value) {\n  return value ?? undefined;\n}");
  expect(out).toContain("    digits: digits ?? undefined,\n");
  expect(out).toContain("  shown.digits = digits ?? undefined;\n");
  expect(out).toContain("  return f(value ?? undefined);\n");
  // A trait object JS may have made, its impl JS's; one the crate made,
  // whose impl is Rust's, given what it holds as it is.
  expect(out).toContain("return digits.impl.digits(digits.value, value ?? undefined);");
  expect(out).toContain("const digits = { impl: plainDigits };\n  return digits.impl.digits(digits.value, value);");
  // Each item `function::bind` binds, the `.bind(..)` kept.
  expect(out).toMatch(/\.bind\(\n    undefined,\n    1\.25,\n    value \?\? undefined,\n  \)/);
  expect(out).toContain("((arg0, precision) => arg0.toPrecision(precision)).bind(");
  // A binding's argument of a type parameter, `js::set`'s `T`, JS takes as
  // it's given; a closure only Rust calls holds what it's given as it is;
  // and `js::null()` is JS's `null` on purpose (ADR 0935).
  expect(out).toContain('object.k = value;');
  expect(out).toMatch(/return shown\(value\);/);
  expect(out).toContain('return "abc".slice(0, null);');
  // A nullable parameter's `null`, given on (#1843).
  expect(out).toContain("export function forwarded(value) {\n  return (1.25).toPrecision(value ?? undefined);\n}");
  // What holds Rust's own `None`, as it is.
  expect(out).toContain("export function digitsNone() {\n  return (1.25).toPrecision();\n}");
  expect(out).toContain("return helper(3) + helper();");
  expect(out).toMatch(/export function fresh\(n\) \{\n  if \(n > 0\) \{\n    return n;\n  \}\n\}/);
  // Only tested, `== null`; and a value of any shape, JS's own.
  expect(out).toContain("export function empty(value) {\n  return value == null;\n}");
  expect(out).toContain("export function shown(value) {\n  return String(value);\n}");
  expect(answers(await import(join(dir, "lib.js")))).toEqual(rust);
  // Chromium's natives, the module's own functions, and of std's only
  // `$index`, a bounds-checked read, given as one.
  expect(out).toContain('import { $index } from "@rust-js/std";');
  const names = [...out.matchAll(/^export (?:function|const) (\w+)/gm)].map((m) => m[1]);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const inPage = await page.evaluate(
      ([code, answer, exported]) => {
        const m = new Function(`${code.replaceAll(/^export /gm, "")}; return { ${exported} };`)();
        return new Function("m", `return (${answer})(m);`)(m);
      },
      [
        out
          .replace(/\/\/# sourceMappingURL.*$/m, "")
          .replace(/^import \{ \$index \} from "@rust-js\/std";$/m, "const $index = (v, i) => v[i];"),
        answers.toString(),
        names.join(", "),
      ],
    );
    expect(inPage).toEqual(rust);
  } finally {
    await browser.close();
  }
}, underLoad(300_000));

// A consumer gives the library its `Option` parameter, which JS may give
// `null`; the library's function, which a consumer calls, holds what it's
// given as JS's, and gives it on to a binding's optional argument as Rust's
// (#1840).
test("a library's function a consumer gives JS's None gives a binding Rust's", async () => {
  const dir = fixture("none-given-library");
  mkdirSync(join(dir, "dep"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), "pub fn digits(value: Option<f64>) -> String {\n    js::number::to_precision(1.25, value)\n}\n");
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", "--edition=2024", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  mkdirSync(join(dir, "app"), { recursive: true });
  writeFileSync(join(dir, "app", "lib.rs"), "pub fn shown(value: Option<f64>) -> String {\n    dep::digits(value)\n}\n");
  run([compiler, join(dir, "app", "lib.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--edition=2024", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("(1.25).toPrecision(value ?? undefined)");
  const app = await import(join(dir, "app", "lib.js"));
  expect([app.shown(null), app.shown(undefined)]).toEqual(["1.25", "1.25"]);
}, underLoad(120_000));
