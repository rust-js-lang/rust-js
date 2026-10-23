// The js crate's builtins: JS's errors, regular expressions, symbols,
// promises, Reflect and Proxy, atomics, Intl, typed arrays, dates and
// JSON, each JS's own. Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0035: what a JS function threw is shown as JS shows it, `String(error)`,
// `SyntaxError: ..`, by `{:?}` and by an `expect`'s panic.
test("a JS error is shown as JS shows it", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-error");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json};
pub fn parsed(text: &str) -> Option<&'static Unknown> {
    json::parse(text).unwrap()
}
pub fn kept(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("invalid");
            None
        }
    };
    value
}
pub fn valid(text: &str) -> bool {
    match json::parse(text) {
        Ok(_) => true,
        Err(_) => false,
    }
}
pub fn told(text: &str) -> String {
    let told = match json::parse(text) {
        Ok(_) => String::new(),
        Err(error) => format!("{error:?}"),
    };
    told
}
pub fn kept_or_told(text: &str) -> Option<&'static Unknown> {
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error:?}");
            None
        }
    };
    value
}
pub fn shown(text: &str) -> String {
    match json::parse(text) {
        Ok(_) => "parsed".to_string(),
        Err(error) => format!("{error:?}"),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withJs]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toMatch(/return String\((match\._0|error)\);/);
  // A `match` of what it threw, whose `Ok` only keeps the value and
  // whose `Err` reads no error, is JS's `try` (ADR 0035).
  expect(js).toContain('  let value;\n  try {\n    value = JSON.parse(text);\n  } catch {\n    console.error("invalid");\n    value = undefined;\n  }');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect([lib.kept("[1]"), lib.kept("{")]).toEqual([[1], undefined]);
  // One whose `Err` reads the error keeps it, `$try`'s.
  expect([lib.keptOrTold("[1]"), lib.keptOrTold("{")]).toEqual([[1], undefined]);
  expect([lib.told("1"), lib.told("{").startsWith("SyntaxError")]).toEqual(["", true]);
  expect([lib.valid("[1]"), lib.valid("{")]).toEqual([true, false]);
  // Its `unwrap()` is the call, which throws what JS threw (ADR 0643).
  expect(() => lib.parsed("{")).toThrow(SyntaxError);
  expect([lib.shown("{").startsWith("SyntaxError: "), lib.shown("1")]).toEqual([true, "parsed"]);
});

// ADR 0643: `.unwrap()` of a throwing JS call is the call itself, its throw
// what JS threw: `JSON.parse(text)`, `"" + value`, `await p`. An
// `.expect(msg)` keeps its message, `$unwrapOk`'s; and a `Result` held,
// unwrapped later, is `$try`'s.
test("unwrap of a throwing JS call is the call", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("unwrap-throwing");
  writeFileSync(join(dir, "lib.rs"), `use js::{Promise, Unknown, json};
pub fn parsed(text: &str) -> Option<&'static Unknown> {
    json::parse(text).unwrap()
}
pub fn coerced(value: Option<&Unknown>) -> String {
    js::concat_text(value).unwrap()
}
pub async fn awaited(p: Promise<f64>) -> f64 {
    js::settle(p).await.unwrap()
}
pub fn expected(text: &str) -> Option<&'static Unknown> {
    json::parse(text).expect("valid JSON")
}
pub fn held(text: &str, first: bool) -> Option<&'static Unknown> {
    let parsed = json::parse(text);
    if first {
        println!("parsing");
    }
    parsed.unwrap()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function parsed(text) {\n  return JSON.parse(text);\n}");
  expect(js).toContain('function coerced(value) {\n  return "" + value;\n}');
  expect(js).toContain("async function awaited(p) {\n  return await p;\n}");
  expect(js).toContain('$unwrapOk(\n    $try(() => JSON.parse(text)),\n    "valid JSON",');
  expect(js).toContain("const parsed = $try(() => JSON.parse(text));");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.parsed("[1]")).toEqual([1]);
  expect(() => lib.parsed("{")).toThrow(SyntaxError);
  expect(() => lib.coerced(Symbol("s"))).toThrow(TypeError);
  expect(lib.coerced(1)).toBe("1");
  expect(await lib.awaited(Promise.resolve(2))).toBe(2);
  expect(lib.awaited(Promise.reject(new RangeError("no")))).rejects.toThrow(RangeError);
  expect(() => lib.expected("{")).toThrow("valid JSON: SyntaxError");
  expect(() => lib.held("{", false)).toThrow("called `Result::unwrap()` on an `Err` value: SyntaxError");
});

// ADR 0283: a regular expression's match, an `Error`'s parts, symbols, the
// global functions and the weak collections are the js crate's, each a
// method or a function of the JS it names; a weak key is an object.
test("RegExp, Error, Symbol, weak collections and globals are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-objects");
  writeFileSync(join(dir, "lib.rs"), `use js::{WeakKey, dict, js_error, parse_int, reg_exp, symbol, weak_map, weak_ref};
pub fn matched(text: &str) -> Option<(String, u32, String, bool)> {
    let pattern = reg_exp::new(r"(?<word>[a-z]+)(\\d)", "g");
    let m = pattern.exec(text)?;
    let word = m.groups().and_then(|groups| dict::get(groups, "word").cloned()).unwrap_or_default();
    Some((m.get(2).unwrap_or_default(), m.index(), word + &m.full_match(), pattern.global()))
}
pub fn errors() -> (String, String, f64, f64) {
    let e = js_error::new("boom");
    (e.name(), e.message(), parse_int("42px", Some(10)), parse_int("0x1f", None))
}
pub fn symbols() -> (bool, Option<String>, Option<String>) {
    let local = symbol::new("local");
    (js::object::is(local, local), symbol::key_for(local), local.description())
}
pub struct State {
    pub n: u32,
}
unsafe impl WeakKey for State {}
pub fn weak(state: &'static State) -> (Option<u32>, bool) {
    let seen = weak_map::new::<State, u32>();
    seen.set(state, state.n);
    let r = weak_ref::new(state);
    (seen.get(state), r.deref().is_some())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const m = pattern.exec(text);");
  expect(js).toContain('const e = new Error("boom");');
  // A radix given or not, as TypeScript's `radix?`: `parseInt(text)` reads `0x` as hex.
  expect(js).toContain('parseInt("42px", 10), parseInt("0x1f")]');
  expect(js).toContain('const local = Symbol("local");');
  expect(js).toContain("const seen = new WeakMap();\n  seen.set(state, state.n);\n  const r = new WeakRef(state);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.matched("ab cd7")).toEqual(["7", 3, "cdcd7", true]);
  expect(lib.matched("none")).toBe(undefined);
  expect(lib.errors()).toEqual(["Error", "boom", 42, 31]);
  expect(lib.symbols()).toEqual([true, undefined, "local"]);
  expect(lib.weak({ n: 5 })).toEqual([5, true]);
});

// ADR 0283: JS's Promise's statics and methods are the js crate's, as
// ReScript's Stdlib has them; `allSettled`'s results a union tagged by
// `status` (ADR 0284).
test("a Promise's statics and methods are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-promises");
  writeFileSync(join(dir, "lib.rs"), `use js::{PromiseSettledResult, PromiseWithResolvers, js_error, promise};
pub async fn made() -> u32 {
    promise::new(|resolve, _| resolve(1)).then(|n: u32| promise::resolve(n + 1)).await
}
pub async fn caught() -> String {
    promise::reject::<String>(&js_error::new("boom")).catch(|e| promise::resolve(e.message())).finally(|| {}).await
}
pub async fn paired() -> (u32, String) {
    promise::all2((promise::resolve(1), promise::resolve("a".to_string()))).await
}
pub async fn settled() -> Vec<String> {
    let all = promise::all_settled(vec![promise::resolve(1), promise::reject(&js_error::new("no"))]).await;
    all.into_iter()
        .map(|r| match r {
            PromiseSettledResult::Fulfilled { value } => value.to_string(),
            PromiseSettledResult::Rejected { reason } => reason.message(),
        })
        .collect()
}
pub async fn resolvers() -> u32 {
    let PromiseWithResolvers { promise, resolve, .. } = promise::with_resolvers::<u32>();
    resolve(7);
    promise::race(vec![promise, promise::any(vec![promise::resolve(8)])]).await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return await new Promise((resolve) => {\n    resolve(1);\n  }).then(");
  expect(js).toContain('Promise.reject(new Error("boom"))\n    .catch((e) => Promise.resolve(e.message))');
  expect(js).toContain('Promise.all([Promise.resolve(1), Promise.resolve("a")])');
  expect(js).toContain('r.status === "fulfilled"');
  const lib = await import(join(dir, "lib.js"));
  expect(await lib.made()).toBe(2);
  expect(await lib.caught()).toBe("boom");
  expect(await lib.paired()).toEqual([1, "a"]);
  expect(await lib.settled()).toEqual(["1", "no"]);
  expect(await lib.resolvers()).toBe(7);
});

// ADR 0283: Reflect's functions and Proxy's traps are the js crate's, of a JS
// value of any shape.
test("Reflect and Proxy are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-reflect");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, property_descriptor, proxy, proxy_handler, reflect, string, unknown};
pub fn reflected(o: &Unknown) -> (bool, String, usize, bool, bool) {
    let d = property_descriptor::new();
    d.set_value(Some(unknown("fixed")));
    d.set_writable(false);
    let defined = reflect::define_property(o, "id", d);
    let written = reflect::set(o, "id", "other", None);
    let own = reflect::get_own_property_descriptor(o, "id").and_then(|d| d.writable()) == Some(false);
    (defined, string(reflect::get(o, "id", None)), reflect::own_keys(o).len(), written, own || !reflect::set_prototype_of(o, None))
}
pub fn proxied(o: &'static Unknown) -> (String, String, bool) {
    let handler = proxy_handler::new();
    handler.set_get(Box::new(|target, key, _| match reflect::get(target, key, None) {
        Some(value) => Some(value),
        None => Some(unknown("missing")),
    }));
    let p = proxy::revocable(o, handler);
    let seen = (string(reflect::get(p.proxy(), "a", None)), string(reflect::get(p.proxy(), "b", None)));
    p.revoke();
    (seen.0, seen.1, reflect::get_prototype_of(o).is_none())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const d = {};");
  expect(js).toContain("d.writable = false;");
  expect(js).toContain("Reflect.setPrototypeOf(o, null)");
  expect(js).toContain("const p = Proxy.revocable(o, handler);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.reflected({ a: 1 })).toEqual([true, "fixed", 2, false, true]);
  expect(lib.proxied(Object.assign(Object.create(null), { a: 1 }))).toEqual(["1", "missing", true]);
});

// ADR 0283: Atomics' functions take an integer typed array, its element a
// Rust number of its kind, and one that a thread may wait on a
// SharedArrayBuffer's.
test("Atomics are JS's, of a view of a shared buffer", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-atomics");
  writeFileSync(join(dir, "lib.rs"), `use js::{WaitAsyncValue, WaitResult, atomics, big_int64_array, int32_array, shared_array_buffer};
pub fn counted() -> (i32, i32, i32, i64, bool, bool) {
    let shared = shared_array_buffer::new(16);
    let counts = int32_array::new_with_buffer(shared, None, None);
    atomics::add(counts, 0, 5);
    let before = atomics::sub(counts, 0, 2);
    atomics::compare_exchange(counts, 1, 0, 9);
    let wide = big_int64_array::new(1);
    atomics::store(wide, 0, 7);
    let waited = matches!(atomics::wait(counts, 0, 0, None), WaitResult::NotEqual);
    (before, atomics::load(counts, 0), atomics::load(counts, 1), atomics::or(wide, 0, 8), waited, counts.buffer().growable())
}
pub async fn waited() -> bool {
    let counts = int32_array::new_with_buffer(shared_array_buffer::new(4), None, None);
    match atomics::wait_async(counts, 0, 0, Some(1.0)).value {
        WaitAsyncValue::Waiting(promise) => matches!(promise.await, WaitResult::TimedOut),
        WaitAsyncValue::Ended(_) => false,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const counts = new Int32Array(shared);");
  expect(js).toContain("Atomics.compareExchange(counts, 1, 0, 9);");
  expect(js).toContain('Atomics.wait(counts, 0, 0) === "not-equal"');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.counted()).toEqual([5, 3, 9, 7n, true, false]);
  expect(await lib.waited()).toBe(true);
});

// ADR 0283: Intl's formatters are the js crate's, their options structs whose
// `None` isn't given, each string union an enum.
test("Intl's formatters are JS's, their options typed", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-intl");
  writeFileSync(join(dir, "lib.rs"), `use js::date;
use js::intl::{self, *};
pub fn numbers() -> (String, String, bool, Vec<String>, String) {
    let price = number_format::new(&["en-US"], &NumberFormatOptions { style: Some(NumberStyle::Currency), currency: Some("USD"), ..Default::default() });
    let plain = number_format::new(&["en-US"], &NumberFormatOptions { use_grouping: Some(UseGrouping::Bool(false)), ..Default::default() });
    let options = plain.resolved_options();
    let parts = price.format_to_parts(-1.5).into_iter().filter(|p| p.r#type != NumberFormatPartType::Literal).map(|p| p.value).collect();
    (price.format(1234.5), plain.format(1234.0), options.use_grouping == UseGrouping::Bool(false), parts, price.format_range(3.0, 5.0))
}
pub fn dates() -> (String, String, Vec<String>) {
    let d = date::new(Some(&0.0));
    let utc = DateTimeFormatOptions { time_zone: Some("UTC"), year: Some(NumericStyle::Numeric), month: Some(MonthStyle::Long), day: Some(NumericStyle::Numeric), ..Default::default() };
    let format = date_time_format::new(&["en-US"], &utc);
    let kinds = format.format_to_parts(d).into_iter().filter(|p| p.r#type != DateTimeFormatPartType::Literal).map(|p| p.value).collect();
    (format.format(d), d.to_locale_date_string(Some(&["en-GB"]), Some(&utc)), kinds)
}
pub fn words() -> (Vec<String>, PluralCategory, String, String, Option<String>, usize) {
    let collator = collator::new(&["en"], &CollatorOptions { numeric: Some(true), ..Default::default() });
    let mut files = vec!["file10", "file2", "File1"];
    files.sort_by(|a, b| collator.compare(a, b));
    let ordinal = plural_rules::new(&["en-US"], &PluralRulesOptions { r#type: Some(PluralRuleType::Ordinal), ..Default::default() });
    let ago = relative_time_format::new(&["en"], &RelativeTimeFormatOptions { numeric: Some(RelativeTimeNumeric::Auto), ..Default::default() });
    let list = list_format::new(&["en"], &ListFormatOptions { r#type: Some(ListFormatType::Disjunction), ..Default::default() });
    let names = display_names::new(&["en"], &DisplayNamesOptions { r#type: DisplayNamesType::Region, locale_matcher: None, style: None, language_display: None, fallback: None });
    let words = segmenter::new(&["en"], &SegmenterOptions { granularity: Some(Granularity::Word), ..Default::default() });
    let count = segments::iter(words.segment("Hello, big world")).filter(|s| s.is_word_like == Some(true)).count();
    (files.iter().map(|f| f.to_string()).collect(), ordinal.select(2.0), ago.format(-1.0, RelativeTimeUnit::Day), list.format(&["a", "b"]), names.of("US"), count)
}
pub fn locales() -> (String, bool, Vec<String>, bool) {
    let maximal = locale::new("en", &LocaleOptions::default()).map(|l| l.maximize().to_string()).unwrap_or_default();
    let canonical = intl::get_canonical_locales(&["EN-us"]).unwrap_or_default();
    (maximal, locale::new("not a tag!", &LocaleOptions::default()).is_err(), canonical, intl::supported_values_of(SupportedValuesKey::Currency).len() > 10)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('new Intl.NumberFormat(["en-US"], { style: "currency", currency: "USD"');
  expect(js).toContain('const format = new Intl.DateTimeFormat(["en-US"], utc);');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.numbers()).toEqual(["$1,234.50", "1234", true, ["-", "$", "1", ".", "50"], "$3.00 – $5.00"]);
  expect(lib.dates()).toEqual(["January 1, 1970", "1 January 1970", ["January", "1", "1970"]]);
  expect(lib.words()).toEqual([["File1", "file2", "file10"], "two", "yesterday", "a or b", "United States", 3]);
  expect(lib.locales()).toEqual(["en-Latn-US", true, ["en-US"], true]);
});

// ADR 0283: JS's typed arrays are the js crate's, each element a Rust number
// of its kind, a `BigInt64Array`'s an `i64`, the `BigInt` rust-js makes one.
test("typed arrays and their buffers are JS's", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-typed-arrays");
  writeFileSync(join(dir, "lib.rs"), `use js::{array_buffer, big_int64_array, data_view, float64_array, int8_array, uint8_array};
pub fn floats() -> (f64, Option<f64>, u32, String) {
    let a = float64_array::from(&[1.5, -2.0, 3.25]);
    a.set_index(1, 4.0);
    let doubled = a.map(|x, _| x * 2.0);
    let sorted = a.to_sorted(Some(Box::new(|x, y| y.partial_cmp(&x).unwrap())));
    let shifted = sorted.map(|x, i| x + i as f64);
    (doubled.reduce(|sum, x, _| sum + x, 0.0), a.at(-1), float64_array::BYTES_PER_ELEMENT, shifted.slice(Some(1), None).join(Some("|")))
}
pub fn bytes() -> (u32, u32, i8, Vec<u8>) {
    let buffer = array_buffer::new(8);
    let view = data_view::new(buffer, None, None);
    view.set_int16(0, -2, true);
    let bytes = uint8_array::new_with_buffer(buffer, None, None);
    let signed = int8_array::of(&[1, -1, 127]);
    (buffer.byte_length(), bytes.length(), signed.get(1).unwrap_or(0), bytes.subarray(Some(0), Some(2)).values().collect())
}
pub fn bigs() -> (i64, bool) {
    let a = big_int64_array::from(&[1, -2, 3]);
    (a.reduce(|sum, x, _| sum + x, 0), a.includes(-2, None))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const a = Float64Array.from([1.5, -2, 3.25]);\n  a[1] = 4;");
  // One callback form, given the index (ADR 0443): a `_` it doesn't read is
  // left out, as JS's callback leaves it.
  expect(js).toContain("const doubled = a.map((x) => x * 2);");
  expect(js).toContain("const shifted = sorted.map((x, i) => x + i);");
  expect(js).toContain("doubled.reduce((sum, x) => sum + x, 0)");
  expect(js).toContain("const signed = Int8Array.of(1, -1, 127);");
  expect(js).toContain("view.setInt16(0, -2, true);");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.floats()).toEqual([17.5, 3.25, 8, "4.25|3.5"]);
  expect(lib.bytes()).toEqual([8, 8, -1, [254, 255]]);
  expect(lib.bigs()).toEqual([2n, true]);
});

// ADR 0283: JS's `Date` is the js crate's, a type with its members as
// methods and its constructors and statics a module's, each the JS it names.
test("a Date is JS's, its members methods", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("js-date");
  writeFileSync(join(dir, "lib.rs"), `use js::{Date, date};
pub fn moment(time: f64) -> (f64, f64, f64, String) {
    let d = date::new(Some(&time));
    d.set_utc_hours(12, None, None, None);
    d.set_utc_minutes(30, Some(15), None);
    (d.get_utc_full_year(), d.get_utc_month(), d.get_time(), d.to_iso_string().unwrap())
}
pub fn invalid() -> (bool, bool, Option<String>) {
    let d = date::new(Some(&"not a date"));
    (d.get_time().is_nan(), d.to_iso_string().is_err(), d.to_json())
}
pub fn utc() -> f64 {
    date::utc(2026, Some(9), Some(8), None, None, None, None)
}
pub fn now_is_recent(d: &Date) -> bool {
    date::now() - d.get_time() < 60_000.0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const d = new Date(time);\n  d.setUTCHours(12);\n  d.setUTCMinutes(30, 15);");
  expect(js).toContain("return Date.UTC(2026, 9, 8);");
  expect(js).toContain("return Date.now() - d.getTime() < 60000;");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.moment(0)).toEqual([1970, 0, 45_015_000, "1970-01-01T12:30:15.000Z"]);
  expect(lib.invalid()).toEqual([true, true, null]);
  expect(lib.utc()).toBe(Date.UTC(2026, 9, 8));
  expect(lib.nowIsRecent(new Date())).toBe(true);
});

// A struct whose JS is its JSON, `unsafe impl JsonText`, is written by
// `JSON.stringify`, indented, as react.dev's Sandpack template writes its
// package.json: `JSON.stringify({ .. }, null, 2)`.
test("a struct whose JS is its JSON is stringified as it is", async () => {
  const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json-text");
  writeFileSync(join(dir, "lib.rs"), `use js::{JsonText, json};
pub struct Dependencies {
    pub react: &'static str,
    #[cfg_attr(rust_js, rust_js::name = "react-dom")]
    pub react_dom: &'static str,
}
pub struct Package {
    pub name: &'static str,
    pub version: Option<&'static str>,
    pub files: Vec<String>,
    pub dependencies: Dependencies,
}
unsafe impl JsonText for Dependencies {}
unsafe impl JsonText for Package {}
pub fn manifest(version: Option<&'static str>) -> String {
    json::stringify_with(
        &Package {
            name: "react.dev",
            version,
            files: vec!["a.js".to_string()],
            dependencies: Dependencies { react: "19", react_dom: "19" },
        },
        None,
        2,
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('dependencies: { react: "19", "react-dom": "19" },\n    },\n    null,\n    2,\n  );');
  const lib = await import(join(dir, "lib.js"));
  const want = { name: "react.dev", files: ["a.js"], dependencies: { react: "19", "react-dom": "19" } };
  expect(lib.manifest(undefined)).toBe(JSON.stringify(want, null, 2));
  expect(JSON.parse(lib.manifest("1")).version).toBe("1");
});

// ADR 0225: JSON is a typed value, `js::Json`, as ReScript's `JSON.t` is,
// and an object of it a `js::Dict`, ReScript's `dict`, TypeScript's
// `Record<string, T>`: a JSON `null` is `None`, a key that isn't there too,
// told apart by `dict::get`.
test("JSON is a typed value, and its objects dictionaries", async () => {
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  const dir = fixture("json");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, Json, dict};
fn show(value: Option<Json>) -> String {
    match value {
        None => "null".to_string(),
        Some(Json::String(text)) => format!("'{text}'"),
        Some(Json::Number(n)) => n.to_string(),
        Some(Json::Bool(b)) => b.to_string(),
        Some(Json::Array(items)) => format!("[{}]", items.iter().map(|item| show(*item)).collect::<Vec<_>>().join(",")),
        Some(Json::Object(fields)) => format!(
            "{{{}}}",
            dict::entries(fields).into_iter().map(|(key, value)| format!("{key}:{}", show(value))).collect::<Vec<_>>().join(",")
        ),
    }
}
pub fn parsed(text: &str) -> String {
    match Json::parse(text) {
        Ok(value) => show(value),
        Err(_) => "invalid".to_string(),
    }
}
pub fn lookup(text: &str, key: &str) -> String {
    match Json::parse(text) {
        Ok(Some(Json::Object(fields))) => match dict::get(fields, key) {
            None => "missing".to_string(),
            Some(value) => show(*value),
        },
        _ => "not an object".to_string(),
    }
}
pub fn round_trip(text: &str) -> String {
    match Json::parse(text) {
        Ok(Some(value)) => Json::stringify(&value),
        _ => "null".to_string(),
    }
}
pub fn built() -> Vec<String> {
    let numbers: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::set(numbers, "b", 2.0);
    dict::keys(numbers)
}
pub fn counted(key: &str) -> f64 {
    let counts: &Dict<f64> = dict::from_entries(vec![("a".to_string(), 1.0)]);
    dict::get(counts, key).copied().unwrap_or(0.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), ...withWeb]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // A lookup is the runtime's `$dictGet`, imported with its other helpers.
  expect(js).toContain('import { $dictGet, $displayF64, $someValue, $try } from "@rust-js/runtime";');
  expect(js).toContain("Object.entries(value)");
  expect(js).toMatch(/ = \$dictGet\(\w+(\._0)?, key\);/);
  expect(js).toMatch(/return JSON\.stringify\(\w+(\._0)?\);/);
  expect(js).toContain("const numbers = { a: 1 };\n  numbers.b = 2;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.parsed('{"a":[1,"x",null,true],"b":{}}'), lib.parsed("null"), lib.parsed("nope")])
    .toEqual(["{a:[1,'x',null,true],b:{}}", "null", "invalid"]);
  expect([lib.lookup('{"a":null,"b":1}', "a"), lib.lookup('{"a":null,"b":1}', "b"), lib.lookup('{"a":null}', "c"), lib.lookup('{"a":null}', "toString"), lib.lookup("[1]", "a")])
    .toEqual(["null", "1", "missing", "missing", "not an object"]);
  expect(lib.roundTrip('{"a": [1, null]}')).toBe('{"a":[1,null]}');
  expect(lib.built()).toEqual(["a", "b"]);
  // A dictionary of what isn't an Option is read as JS reads one (ADR 0225).
  expect(js).toContain("return counts[key] ?? 0;");
  expect([lib.counted("a"), lib.counted("b")]).toEqual([1, 0]);
});
