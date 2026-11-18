import {beforeAll, expect, test} from "bun:test";
import {readFileSync, writeFileSync} from "node:fs";
import {join} from "node:path";
import {chromium} from "playwright";
import {underLoad} from "../scripts/timing";
import {intlArgumentCases} from "../builtins/coverage";
import {runSync} from "./child";
import {buildReact, compiler, fixture, root, run, target} from "./support";

beforeAll(buildReact, 600_000);
const source = `use js::intl::{self, DateTimeFormatOptions, IntoLocalesArgument, IntoStrOrStrList, IntoDateOrNumber, IntoBigIntOrDateOrNumber, LocaleElement, LocaleOptions};
use js::{Date, JsError};
pub fn locales(value: Option<&dyn IntoStrOrStrList>) -> Result<Vec<String>, &'static JsError> {
    intl::get_canonical_locales(value)
}
pub fn formatted(locales: Option<&dyn IntoLocalesArgument>, value: Option<&dyn IntoDateOrNumber>) -> String {
    let formatter = intl::date_time_format::new(locales, Some(&DateTimeFormatOptions { time_zone: Some("UTC"), ..Default::default() }));
    formatter.format(value)
}
pub fn parts(locales: Option<&dyn IntoLocalesArgument>, value: Option<&dyn IntoDateOrNumber>) -> Vec<intl::DateTimeFormatPart> {
    intl::date_time_format::new(locales, Some(&DateTimeFormatOptions { time_zone: Some("UTC"), ..Default::default() })).format_to_parts(value)
}
pub fn range(start: impl IntoBigIntOrDateOrNumber, end: impl IntoBigIntOrDateOrNumber) -> String {
    intl::date_time_format::new(Some(&"en-US"), Some(&DateTimeFormatOptions { time_zone: Some("UTC"), ..Default::default() })).format_range(start, end)
}
pub fn range_parts(start: impl IntoBigIntOrDateOrNumber, end: impl IntoBigIntOrDateOrNumber) -> Vec<intl::DateTimeRangeFormatPart> {
    intl::date_time_format::new(Some(&"en-US"), Some(&DateTimeFormatOptions { time_zone: Some("UTC"), ..Default::default() })).format_range_to_parts(start, end)
}
pub fn rust_forms(date: &Date) -> Vec<String> {
    let locale = intl::locale::new("de-DE", &LocaleOptions::default()).unwrap();
    let mixed = [LocaleElement::Text("en-US"), LocaleElement::Locale(locale)];
    let owned = vec![String::from("fr-FR")];
    let formatter = intl::date_time_format::new(Some(&mixed), None);
    let _ = formatter.format(Some(&0.0));
    let _ = formatter.format(Some(&date));
    let _ = intl::date_time_format::new(Some(&owned), None);
    let _ = intl::date_time_format::new(Some(&owned.as_slice()), None);
    let _ = intl::date_time_format::new(Some(&String::from("ja-JP")), None);
    let _ = intl::date_time_format::new(Some(&locale), None);
    intl::get_canonical_locales(Some(&["en-US", "de-DE"])).unwrap()
}
`;

test("Intl locale unions and date inputs preserve native calls in Bun and Chromium", async () => {
  const dir = fixture("intl-arguments");
  const rust = join(dir, "lib.rs"), output = join(dir, "lib.js");
  writeFileSync(rust, source);
  run([compiler, "--declarations", rust, "-o", output, "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(output, "utf8");
  expect(js).toContain("Intl.getCanonicalLocales(value)");
  expect(js).toContain("formatter.format(value)");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("Intl.LocalesArgument");
  expect(declarations).toContain("readonly string[]");
  expect(declarations).toContain("value?: (Date | number) | null | undefined): string");
  expect(declarations).toContain("): Intl.DateTimeFormatPart[]");
  expect(declarations).toContain("end: bigint | Date | number): string");
  expect(declarations).toContain("end: bigint | Date | number): Intl.DateTimeRangeFormatPart[]");
  // The Result return of locales is the separately tracked declaration gap;
  // this regression checks all changed arguments and native formatter records.
  const consumer = join(dir, "consumer.ts");
  writeFileSync(consumer, `import {formatted, parts, range, rangeParts, locales} from './lib.js';
const locale = new Intl.Locale('en-US');
formatted(locale, new Date(0));
formatted(['de-DE', locale], 0);
parts('en-US', 0)[0].type;
range(0n, new Date(0));
rangeParts(0, 0)[0].source;
locales(['en-US']);
// @ts-expect-error a locale is not a boolean
formatted(true, 0);
// @ts-expect-error a timestamp is not a string
formatted('en-US', 'today');
// @ts-expect-error required range inputs cannot be missing
range(undefined, 0);
// @ts-expect-error canonical names exclude Intl.Locale
locales(locale);
// @ts-expect-error parts retain native field types
parts('en-US', 0)[0].missing;
`);
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({compilerOptions: {strict: true, noEmit: true, skipLibCheck: false, target: "esnext", module: "esnext", moduleResolution: "bundler", typeRoots: [join(root, "node_modules/@types")]}, files: [consumer]}));
  const checked = runSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], dir, 20000);
  expect(checked.code, checked.stdout + checked.stderr).toBe(0);
  const made = await import(output);
  function compare(made: any) {
    const results: unknown[] = [];
    const outcome = (run: () => unknown) => {try {return {value: run()};} catch (error) {return {error: error instanceof Error ? [error.name, error.message] : error};}};
    for (const locales of [undefined, "en-US", "de-DE", ["fr-FR", "en-US"], new Intl.Locale("ja-JP"), ["en-US", new Intl.Locale("de-DE")], "not a tag!"]) {
      results.push([outcome(() => {const result = made.locales(locales); if (result.TAG === "Err") throw result._0; return result._0;}), outcome(() => Intl.getCanonicalLocales(locales as any))]);
      for (const value of [new Date(0), 0, -86400000, 1.5, NaN, Infinity]) {
        const expected = () => new Intl.DateTimeFormat(locales as any, {timeZone: "UTC"});
        results.push([outcome(() => made.formatted(locales, value)), outcome(() => expected().format(value))]);
        results.push([outcome(() => made.parts(locales, value)), outcome(() => expected().formatToParts(value))]);
      }
    }
    const formatter = new Intl.DateTimeFormat("en-US", {timeZone: "UTC"});
    for (const start of [new Date(0), 0, -86400000, 0n, NaN, Infinity]) for (const end of [new Date(86400000), 86400000, 86400000n, NaN]) {
      results.push([outcome(() => made.range(start, end)), outcome(() => formatter.formatRange(start as any, end as any))]);
      results.push([outcome(() => made.rangeParts(start, end)), outcome(() => formatter.formatRangeToParts(start as any, end as any))]);
    }
    results.push([made.rustForms(new Date(0)), ["en-US", "de-DE"]]);
    return results;
  }
  for (const [madeValue, native] of compare(made) as any[]) expect(madeValue).toEqual(native);
  const browser = await chromium.launch({headless: true});
  try {
    const page = await browser.newPage();
    const bundled = await Bun.build({entrypoints: [output], target: "browser", format: "esm"});
    expect(bundled.success).toBe(true);
    const module = "data:text/javascript;base64," + Buffer.from(await bundled.outputs[0].text()).toString("base64");
    const results = await page.evaluate(async ({module, compare}) => {
      const made = await import(module);
      return (0, eval)("(" + compare + ")")(made);
    }, {module, compare: compare.toString()});
    for (const [madeValue, native] of results) expect(madeValue).toEqual(native);
  } finally {await browser.close();}
}, underLoad(60_000));

// Member-count coverage alone would miss the original narrowing bugs.
test("pinned TypeScript argument unions are admitted by the Rust bindings", async () => {
  const generated = await intlArgumentCases();
  expect(generated.sites).toHaveLength(26);
  // Each alternative of a root the crate doesn't bind, named with its issue
  // (builtins/gaps.txt): `Intl.FormattableTemporalObject`'s six (#1918).
  expect(generated.gapped).toEqual(
    ["Instant", "PlainDate", "PlainDateTime", "PlainMonthDay", "PlainTime", "PlainYearMonth"].map((t) => `Temporal.${t}\t#1918`),
  );
  expect(generated.cases).toBeGreaterThan(200);
  const dir = fixture("intl-argument-coverage");
  const input = join(dir, "lib.rs");
  writeFileSync(input, generated.source);
  run([compiler, input, "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
});

test("the Rust input unions reject nonmembers", () => {
  const dir = fixture("intl-argument-nonmembers");
  for (const [name, statement] of [
    ["bool-locale", 'let _ = js::intl::date_time_format::new(Some(&true), None);'],
    ["number-locale", 'let _ = js::intl::date_time_format::new(Some(&1.0), None);'],
    ["bool-list", 'let _ = js::intl::date_time_format::new(Some(&[true]), None);'],
    ["string-date", 'let _ = formatter.format(Some(&"today"));'],
    ["bool-range", 'let _ = formatter.format_range(true, 0.0);'],
  ]) {
    const input = join(dir, name + ".rs");
    writeFileSync(input, 'pub fn invalid(formatter: &js::intl::DateTimeFormat) {' + statement + '}');
    const result = runSync([compiler, input, "-o", join(dir, name + ".js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target], root, 20000);
    expect(result.code, name).not.toBe(0);
    expect(result.stderr, name).toContain("error[E0277]");
  }
});
