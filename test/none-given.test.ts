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
  // `String(undefined)`, as `js::string(None)` is (#1868).
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
  expect(out).toContain("export function digits(value) {\n  return value != null ? (1.25).toPrecision(value) : (1.25).toPrecision();\n}");
  expect(out).toContain("export function passed(value) {\n  return value ?? undefined;\n}");
  expect(out).toContain("    digits: digits ?? undefined,\n");
  expect(out).toContain("  shown.digits = digits ?? undefined;\n");
  expect(out).toContain("  return value != null ? f(value) : f();\n");
  // A closure of the crate's own, only Rust's, called as it is.
  expect(out).toContain("  const shown = (value) => value == null;\n  return shown(value);\n");
  // A trait object JS may have made, its impl JS's; one the crate made,
  // whose impl is Rust's, given what it holds as it is.
  expect(out).toContain("return digits.impl.digits(digits.value, value ?? undefined);");
  expect(out).toContain("const digits = { impl: plainDigits };\n  return digits.impl.digits(digits.value, value);");
  // Each item `function::bind` binds, the `.bind(..)` kept.
  expect(out).toMatch(/\.bind\(\n    undefined,\n    1\.25,\n    value \?\? undefined,\n  \)/);
  expect(out).toContain("((arg0, precision) =>\n    precision != null ? arg0.toPrecision(precision) : arg0.toPrecision()).bind(");
  // A binding's argument of a type parameter, `js::set`'s `T`, JS takes as
  // it's given; a closure only Rust calls holds what it's given as it is;
  // and `js::null()` is JS's `null` on purpose (ADR 0935).
  expect(out).toContain('object.k = value;');
  expect(out).toMatch(/return shown\(value\);/);
  expect(out).toContain('return "abc".slice(0, null);');
  // A nullable parameter's `null`, given on (#1843).
  expect(out).toContain("export function forwarded(value) {\n  return value != null ? (1.25).toPrecision(value) : (1.25).toPrecision();\n}");
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
  expect(readFileSync(join(dir, "dep", "lib.js"), "utf8")).toContain("value != null ? (1.25).toPrecision(value) : (1.25).toPrecision()");
  const app = await import(join(dir, "app", "lib.js"));
  expect([app.shown(null), app.shown(undefined)]).toEqual(["1.25", "1.25"]);
}, underLoad(120_000));

// A binding's trailing `None` is no argument, a dynamic one as a literal
// one is (#1842, ADR 0443, amended): `setFullYear(2026, month)` of a `None`
// is `setFullYear(2026)`, `new Date(value)` of one `new Date()`, now, where
// `setFullYear(2026, undefined)` is NaN and `new Date(undefined)` Invalid
// Date. Each count's call, tested from the last, `!= null`, as JS's `null`
// is Rust's `None` too (#1840); in Bun and in Chromium.
const trailing = `fn month_of(month: Option<i32>) -> f64 {
    let date = js::date::new_with_year_and_month(2020, 5, Some(15), None, None, None, None);
    date.set_full_year(2026, month, None)
}

pub fn dynamic(month: Option<i32>) -> f64 {
    month_of(month)
}

pub fn literal() -> f64 {
    let date = js::date::new_with_year_and_month(2020, 5, Some(15), None, None, None, None);
    date.set_full_year(2026, None, None)
}

pub fn now_given(value: Option<&dyn js::date::IntoNumberOrStrOrDate>) -> bool {
    js::date::new(value).get_time().is_nan()
}

pub fn utc(day: Option<i32>, hours: Option<i32>, minutes: Option<i32>, seconds: Option<i32>) -> f64 {
    js::date::utc(2020, Some(5), day, hours, minutes, seconds, None)
}

pub fn counted(month: &dyn Fn() -> Option<i32>) -> f64 {
    let date = js::date::new_with_year_and_month(2020, 5, Some(15), None, None, None, None);
    date.set_full_year(2026, month(), None)
}

pub fn either(given: bool) -> f64 {
    let month = if given { Some(1) } else { None };
    let date = js::date::new_with_year_and_month(2020, 5, Some(15), None, None, None, None);
    date.set_full_year(2026, month, None)
}

type SetYear = fn(&js::Date, i32, Option<i32>, Option<i32>) -> f64;

pub fn through_pointer(month: Option<i32>) -> f64 {
    let set: SetYear = js::Date::set_full_year;
    let date = js::date::new_with_year_and_month(2020, 5, Some(15), None, None, None, None);
    set(date, 2026, month, None)
}

pub fn last_slash(text: &str, at: f64) -> f64 {
    js::string::last_index_of(text, "/", Some(at - 1.0))
}
`;

const trailingAnswers = (m: Record<string, (...args: unknown[]) => unknown>) => [
  m.dynamic(undefined) === m.literal(),
  m.dynamic(null) === m.literal(),
  m.dynamic(1) !== m.literal(),
  m.nowGiven(undefined),
  m.nowGiven(null),
  m.nowGiven(0),
  m.utc(undefined, undefined, undefined, undefined) === Date.UTC(2020, 5),
  m.utc(2, undefined, undefined, undefined) === Date.UTC(2020, 5, 2),
  Number.isNaN(m.utc(2, 3, undefined, 4)),
  m.utc(2, 3, 4, 5) === Date.UTC(2020, 5, 2, 3, 4, 5),
  m.lastSlash("a/b/c", 3) === 1,
  // An `Option` made `Some` or `None` is tested.
  m.either(false) === m.literal(),
  m.either(true) === m.dynamic(1),
  // A method's pointer, an arrow anyway, its call of each count.
  m.throughPointer(undefined) === m.literal(),
  // Each argument made once, where it's given.
  (() => {
    let made = 0;
    return [m.counted(() => (made++, 1)), made].join() === [m.dynamic(1), 1].join();
  })(),
];

test("a binding's trailing None is no argument, dynamic or literal, in Bun and in Chromium", async () => {
  const dir = fixture("none-trailing");
  writeFileSync(join(dir, "lib.rs"), trailing);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("  return month != null ? date.setFullYear(2026, month) : date.setFullYear(2026);\n");
  expect(out).toContain("(value != null ? new Date(value) : new Date()).getTime()");
  // A `Some`'s value never `undefined`, `Some(at - 1.0)`'s, is there.
  expect(out).toContain('  return text.lastIndexOf("/", at - 1);\n');
  expect(out).toContain("  return seconds != null\n    ? Date.UTC(2020, 5, day ?? undefined, hours ?? undefined, minutes ?? undefined, seconds)\n");
  // Rust's: a `None` before one that's there stays, `undefined`, NaN.
  const rust = [true, true, true, false, false, false, true, true, true, true, true, true, true, true, true];
  expect(trailingAnswers(await import(join(dir, "lib.js")))).toEqual(rust);
  const names = [...out.matchAll(/^export function (\w+)/gm)].map((m) => m[1]);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const inPage = await page.evaluate(
      ([code, answer, exported]) => {
        const m = new Function(`${code.replaceAll(/^export /gm, "")}; return { ${exported} };`)();
        return new Function("m", `return (${answer})(m);`)(m);
      },
      [out.replace(/\/\/# sourceMappingURL.*$/m, ""), trailingAnswers.toString(), names.join(", ")],
    );
    expect(inPage).toEqual(rust);
  } finally {
    await browser.close();
  }
}, underLoad(300_000));

// A WebIDL operation's trailing `undefined` of an optional argument is no
// argument, as its overload resolution says, so a call of a binding marked
// `#[rust_js::undefined_is_absent]` gives a trailing `None` as
// `undefined`, no call of each count (#1842, ADR 0443, amended): `new
// URL(url, undefined)` is `new URL(url)`, not of a base "undefined", which
// throws; `console.timeStamp`'s too, shown so by
// test/undefined-is-absent.test.ts. But webapi's `scheduler.yield(options)`,
// Chrome's first form, which isn't shown to, is a call of each count.
const webidl = `pub fn resolved(url: &str, base: Option<&str>) -> String {
    webapi::url::new(url, base).href()
}

pub fn plain(url: &str) -> String {
    webapi::url::new(url, None).href()
}

pub fn stamp(label: &str, track: Option<&str>, group: Option<&str>) {
    webapi::console::time_stamp(label, None, None, track, group, None)
}

// A binding that takes undefined as none, as a value, a dyn Fn or std's
// map's function, is given a nullable place's null as Rust's None: new
// Comment(null) is of "null", new Comment(undefined) of "".
thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static DATA: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

pub fn commented() -> String {
    let make: &dyn Fn(Option<&str>) -> &'static webapi::Comment = &webapi::comment::new;
    make(DATA.get()).data()
}

pub fn commented_by_pointer() -> String {
    let make: fn(Option<&str>) -> &'static webapi::Comment = webapi::comment::new;
    make(DATA.get()).data()
}

pub fn each_commented() -> Vec<String> {
    vec![DATA.get()].into_iter().map(webapi::comment::new).map(|c| c.data()).collect()
}

pub fn yielded(scheduler: &webapi::Scheduler, delay: Option<f64>) -> js::Promise<()> {
    scheduler.r#yield(delay.map(|delay| webapi::SchedulerPostTaskOptions { delay: Some(delay), ..Default::default() }))
}
`;

test("a WebIDL operation's trailing None is undefined, no ladder, in Bun and in Chromium", async () => {
  const dir = fixture("none-webidl");
  writeFileSync(join(dir, "lib.rs"), webidl);
  const web = ["--extern", `webapi=${join(target, "libwebapi.rmeta")}`];
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", ...web, ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  const [marked, unmarked] = out.split("export function yielded");
  // But a dyn Fn's call, which may be JS's, of each count (#1876).
  expect(marked.split("export function commented")[0]).not.toContain("!= null");
  expect(unmarked).toContain("!= null ? scheduler.yield(");
  expect(out).toContain("new URL(url, base");
  // A literal one left out, as its ladder isn't there to.
  expect(out).toContain("return new URL(url).href;");
  expect(out).toContain("console.timeStamp(label, undefined, undefined, track");
  const m = await import(join(dir, "lib.js"));
  const answers = (m: Record<string, (...args: (string | undefined)[]) => string>) => [m.resolved("https://a.test/b", undefined), m.resolved("c", "https://a.test/b")];
  const rust = ["https://a.test/b", "https://a.test/c"];
  expect(answers(m)).toEqual(rust);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const inPage = await page.evaluate(
      ([code, answer]) => {
        const m = new Function(`${code.replaceAll(/^export /gm, "")}; return { resolved, commented, commentedByPointer, eachCommented };`)();
        return [new Function("m", `return (${answer})(m);`)(m), m.commented(), m.commentedByPointer(), m.eachCommented()];
      },
      [out.replace(/\/\/# sourceMappingURL.*$/m, ""), answers.toString()],
    );
    expect(inPage).toEqual([rust, "", "", [""]]);
  } finally {
    await browser.close();
  }
}, underLoad(300_000));

// Each source of JS's `null` among the crate's `Option`s reaches an exit no
// call of each count tests, what a function JS calls returns, as Rust's
// `None`, `undefined` (#1840, ADR 0030, amended): a nullable parameter's,
// field's and thread-local's, a binding's result, what a `dyn Fn` JS gave
// gives, what std gives back of JS's, an `Option` of another's payload, a
// place a `&mut` the crate's function writes, a closure JS calls. Since a
// binding's optional argument is a call of each count, `!= null` (#1842),
// only such an exit tells each source's answer.
const sources = `use std::cell::Cell;

#[cfg_attr(rust_js, rust_js::nullable(value))]
fn nullable_param(value: Option<f64>) -> Option<f64> {
    value
}

pub fn of_a_nullable_param() -> Option<f64> {
    nullable_param(None)
}

pub struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Option<f64>,
}

pub fn of_a_nullable_field(slot: &Slot) -> Option<f64> {
    slot.value
}

pub fn of_a_none_field() -> Option<f64> {
    let slot = Slot { value: None };
    slot.value
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CURRENT: Cell<Option<f64>> = const { Cell::new(None) };
}

pub fn of_a_nullable_local() -> Option<f64> {
    CURRENT.get()
}

#[cfg_attr(rust_js, rust_js::link_name = "globalThis.givenNumber")]
fn given_number() -> Option<f64> {
    unreachable!()
}

pub fn of_a_binding() -> Option<f64> {
    given_number()
}

pub fn of_a_callback(f: &dyn Fn() -> Option<f64>) -> Option<f64> {
    f()
}

pub fn of_std(values: Vec<Option<f64>>) -> Option<f64> {
    values[0]
}

pub fn of_a_payload(value: &Option<f64>) -> Option<f64> {
    value.as_ref().copied()
}

fn fill(slot: &mut Option<f64>, value: Option<f64>) {
    *slot = value;
}

pub fn of_a_filled_place(value: Option<f64>) -> Option<f64> {
    let mut slot = Some(1.0);
    fill(&mut slot, value);
    slot
}

pub fn of_an_escaped_closure(f: &dyn Fn(&dyn Fn(Option<f64>))) -> Option<f64> {
    let got = Cell::new(Some(1.0));
    f(&|value| got.set(value));
    got.get()
}
`;

test("each source of JS's null leaves a function JS calls as undefined", async () => {
  const dir = fixture("none-sources");
  writeFileSync(join(dir, "lib.rs"), sources);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", ...js]);
  const m = await import(join(dir, "lib.js"));
  (globalThis as Record<string, unknown>).givenNumber = () => null;
  const each = {
    ofANullableParam: m.ofANullableParam(),
    ofANullableField: m.ofANullableField({ value: null }),
    ofANoneField: m.ofANoneField(),
    ofANullableLocal: m.ofANullableLocal(),
    ofABinding: m.ofABinding(),
    ofACallback: m.ofACallback(() => null),
    ofStd: m.ofStd([null]),
    ofAPayload: m.ofAPayload(null),
    ofAFilledPlace: m.ofAFilledPlace(null),
    ofAnEscapedClosure: m.ofAnEscapedClosure((f: (v: unknown) => void) => f(null)),
  };
  expect(Object.entries(each).filter(([, value]) => value !== undefined)).toEqual([]);
}, underLoad(120_000));

// A function value JS may have made, a `dyn Fn` or a pointer JS gave,
// called of a trailing `None` known only where it runs, is called of each
// count, as a binding is, for a JS function may count what it's given
// (#1876, ADR 0443, amended): `f(a, None)` is `f(a)`, `b != null ? f(a, b)
// : f(a)`. A binding taken as a value is the arrow of its call of each
// count, so a `bind` of some of its items, called of the rest, is right
// either way: `Math.max.bind`'s `(1, None)` called of `(Some(5))` is
// `Math.max(1, undefined, 5)`, NaN, as Rust's middle `None` is given; of
// `(None)`, `Math.max(1)`. And one that takes `undefined` as none is the
// function itself, `scheduleTimeout = setTimeout`.
const values = `#[cfg_attr(rust_js, rust_js::link_name = "Math.max")]
fn max3(_a: f64, _b: Option<f64>, _c: Option<f64>) -> f64 {
    unreachable!()
}

pub fn all(b: Option<f64>, c: Option<f64>) -> f64 {
    let bound = js::function::bind(max3, None, (1.0, b, c));
    bound()
}

pub fn some(b: Option<f64>, c: Option<f64>) -> f64 {
    let bound = js::function::bind(max3, None, (1.0, b));
    bound(c)
}

pub fn called(f: &dyn Fn(f64, Option<f64>) -> f64, a: f64, b: Option<f64>) -> f64 {
    f(a, b)
}

pub fn pointed(f: fn(f64, Option<f64>) -> f64, a: f64, b: Option<f64>) -> f64 {
    f(a, b)
}

pub fn some_called(f: &dyn Fn(f64, Option<f64>) -> f64, a: f64, b: f64) -> f64 {
    f(a, Some(b))
}

pub fn some_pointed(f: fn(f64, Option<f64>) -> f64, a: f64, b: f64) -> f64 {
    f(a, Some(b))
}

pub fn unit_pointed(f: fn(f64, ()) -> f64, a: f64) -> f64 {
    f(a, ())
}

type SetTimeout = fn(Box<dyn FnOnce()>, Option<f64>) -> &'static js::TimeoutId;

thread_local! {
    pub static SCHEDULE: Option<SetTimeout> = if js::is_function(js::set_timeout::<fn()>) { Some(js::set_timeout) } else { None };
}
`;

const valueAnswers = (m: Record<string, (...args: unknown[]) => unknown>) => {
  const count = (...args: unknown[]) => args.length;
  return [
    m.all(undefined, undefined),
    m.all(2, undefined),
    Number.isNaN(m.all(undefined, 3)),
    m.some(undefined, undefined),
    Number.isNaN(m.some(undefined, 5)),
    m.some(2, 5),
    m.called(count, 1, undefined),
    m.called(count, 1, null),
    m.called(count, 1, 2),
    m.pointed(count, 1, undefined),
    m.pointed(count, 1, 2),
  ];
};

test("a function value's trailing None is no argument, a bound one's too, in Bun and in Chromium", async () => {
  const dir = fixture("none-values");
  writeFileSync(join(dir, "lib.rs"), values);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain("  return b != null ? f(a, b) : f(a);\n");
  // A Some where it's given is there.
  expect(out).toContain("export function someCalled(f, a, b) {\n  return f(a, b);\n}");
  expect(out).toContain("export function somePointed(f, a, b) {\n  return f(a, b);\n}");
  // A trailing () is no argument either, as it was (ADR 0441).
  expect(out).toContain("export function unitPointed(f, a) {\n  return f(a);\n}");
  expect(out).toContain('typeof setTimeout === "function" ? setTimeout : undefined');
  const rust = [1, 2, true, 1, true, 5, 1, 1, 2, 1, 2];
  expect(valueAnswers(await import(join(dir, "lib.js")))).toEqual(rust);
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const inPage = await page.evaluate(
      ([code, answer]) => {
        const m = new Function(`${code.replaceAll(/^export /gm, "")}; return { all, some, called, pointed };`)();
        return new Function("m", `return (${answer})(m);`)(m);
      },
      [out.replace(/\/\/# sourceMappingURL.*$/m, ""), valueAnswers.toString()],
    );
    expect(inPage).toEqual(rust);
  } finally {
    await browser.close();
  }
}, underLoad(300_000));

test("a return preserves its locally written null and undefined", async () => {
  const dir = fixture("locally-written-null");
  writeFileSync(join(dir, "lib.rs"), `
pub fn removed(has_entry: bool, remove: fn() -> f64) -> Option<f64> {
    let mut value = js::null();
    if has_entry {
        let removed = remove();
        value = if removed > 0.0 { Some(removed) } else { None };
    }
    value
}

pub fn passed(value: Option<f64>) -> Option<f64> {
    value
}

fn computed(x: f64) -> Option<f64> {
    if x > 0.0 { Some(x) } else { None }
}

pub fn found(has_entry: bool, x: f64) -> Option<f64> {
    let mut value = js::null();
    if has_entry {
        value = computed(x);
    }
    value
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const output = readFileSync(join(dir, "lib.js"), "utf8");
  expect(output).toContain("  return value;\n}");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toMatch(/removed\([^;]+\): number \| null \| undefined;/);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.removed(false, () => { throw new Error("must not run"); })).toBeNull();
  expect(lib.removed(true, () => 0)).toBeUndefined();
  expect(lib.removed(true, () => 12)).toBe(12);
  expect(lib.passed(null)).toBeUndefined();
  // Its other write the crate's own function's result, whose \`None\` is
  // Rust's: only the written null's source says the null is the program's.
  expect([lib.found(false, 3), lib.found(true, 0), lib.found(true, 3)]).toEqual([null, undefined, 3]);
});

test("a return keeps written null beside an undefined-only object lookup", async () => {
  const dir = fixture("written-null-and-object-lookup");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Unknown};

type Data = Dict<Option<&'static Unknown>>;

#[cfg_attr(rust_js, rust_js::link_name = "testLookup")]
#[cfg_attr(rust_js, rust_js::none_undefined)]
fn lookup() -> Option<&'static Data> {
    unreachable!()
}

pub fn removed(has_entry: bool, post_lookup: fn()) -> Option<&'static Data> {
    let mut value = js::null();
    if has_entry {
        value = lookup();
        post_lookup();
    }
    value
}

pub fn is_missing(has_entry: bool, post_lookup: fn()) -> bool {
    removed(has_entry, post_lookup).is_none()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const output = readFileSync(join(dir, "lib.js"), "utf8");
  expect(output).toContain("  return value;\n}");
  const lib = await import(join(dir, "lib.js"));
  const lookup = globalThis as typeof globalThis & { testLookup?: () => unknown };
  const entry = { active: null };
  try {
    lookup.testLookup = () => { throw new Error("must not look up"); };
    expect(lib.removed(false, () => {})).toBeNull();
    expect(lib.isMissing(false, () => {})).toBe(true);
    lookup.testLookup = () => undefined;
    expect(lib.removed(true, () => {})).toBeUndefined();
    expect(lib.isMissing(true, () => {})).toBe(true);
    lookup.testLookup = () => entry;
    expect(lib.removed(true, () => {})).toBe(entry);
    expect(lib.isMissing(true, () => {})).toBe(false);
  } finally {
    delete lookup.testLookup;
  }
});

test("the crate's own callers read a returned local's written null as None", async () => {
  const dir = fixture("locally-written-null-called-here");
  writeFileSync(join(dir, "lib.rs"), `
pub fn removed(has_entry: bool, remove: fn() -> f64) -> Option<f64> {
    let mut value = js::null();
    if has_entry {
        let removed = remove();
        value = if removed > 0.0 { Some(removed) } else { None };
    }
    value
}

fn zero() -> f64 {
    0.0
}

fn twelve() -> f64 {
    12.0
}

pub fn missing(has_entry: bool) -> bool {
    removed(has_entry, zero).is_none()
}

pub fn removed_or(has_entry: bool) -> f64 {
    removed(has_entry, twelve).unwrap_or(-1.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.missing(false), lib.missing(true)]).toEqual([true, true]);
  expect([lib.removedOr(false), lib.removedOr(true)]).toEqual([-1, 12]);
});

test("an explicit null fallback stays a supplied callback argument", async () => {
  const dir = fixture("callback-explicit-null-fallback");
  writeFileSync(join(dir, "lib.rs"), `pub fn send(value: Option<f64>, callback: &dyn Fn(Option<f64>)) {
    callback(value.or_else(js::null));
}

pub fn send_optional(value: Option<f64>, callback: &dyn Fn(Option<f64>)) {
    callback(value);
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", ...js]);
  const { send, sendOptional } = await import(join(dir, "lib.js"));
  const received: unknown[][] = [];
  const callback = (...args: unknown[]) => { received.push(args); };
  for (const value of [undefined, null, 3]) send(value, callback);
  for (const value of [undefined, null, 3]) sendOptional(value, callback);
  expect(received).toEqual([[null], [null], [3], [], [], [3]]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("callback(value ?? null)");
});
