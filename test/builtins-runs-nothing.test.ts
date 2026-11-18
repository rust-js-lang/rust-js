// A builtin that runs none of the program's code, `#[rust_js::runs_nothing]`
// (ADR 0867), or only what it's given, `#[rust_js::runs_only_given]` (ADR
// 0898): a `RefCell` borrowed across its call isn't counted, so a memo whose
// maker calls only such builtins reads its cache plainly. The marks of
// generated builtins are builtins/policy.ts' `RUNS`, each with its spec step;
// `RegExp`'s are written by hand (#2082). A constructor that runs only what
// it's given runs nothing given `None` where its options' type is an object a
// JS caller could hand over, but not given that object. One whose maker calls
// the program back, a JS callback, keeps the check, and its re-entry panics
// as Rust's.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const memo = (maker: string, signature = "key: &str", key = "key") => `use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

pub fn memo(${signature}) -> f64 {
    CACHE.with_borrow_mut(|cache| *cache.entry(${key}.to_string()).or_insert_with(|| ${maker}))
}
`;

async function compiled(name: string, source: string) {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

test("a memo whose maker runs only a regex reads its cache plainly", async () => {
  const { js, lib } = await compiled("runs-nothing-regex", memo(`js::reg_exp::new("^de", "i").test(key) as u8 as f64`));
  expect(js).not.toContain("$withBorrowMut");
  expect([lib.memo("de-DE"), lib.memo("en"), lib.memo("de-DE")]).toEqual([1, 0, 1]);
});

test("a memo whose maker reads a locale's own reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-locale",
    memo("tag.maximize().to_string().len() as f64", "tag: &js::intl::Locale", "tag"),
  );
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo(new Intl.Locale("en"))).toBe(new Intl.Locale("en").maximize().toString().length);
});

test("a memo whose maker calls the program back keeps its borrow checked, and its re-entry panics", async () => {
  const { js, lib } = await compiled("runs-nothing-callback", memo("make()", "key: &str, make: &dyn Fn() -> f64"));
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("a", () => 2)).toBe(2);
  expect(() => lib.memo("b", () => lib.memo("c", () => 3))).toThrow("already borrowed");
});

test("a memo whose maker makes a locale and a date format of no options reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-intl-none",
    memo(
      `js::intl::locale::new(key, None).map_or(0.0, |tag| tag.to_string().len() as f64) + js::intl::date_time_format::new(None, None).resolved_options().locale.len() as f64`,
    ),
  );
  expect(js).not.toContain("$withBorrowMut");
  const runtime = new Intl.DateTimeFormat().resolvedOptions().locale.length;
  expect(lib.memo("de-DE")).toBe(5 + runtime);
  expect(lib.memo("not a tag!")).toBe(runtime);
});

test("a memo whose maker gives a locale a JS caller's options keeps its borrow checked", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-intl-given",
    memo(
      `js::intl::locale::new(key, Some(options)).map_or(0.0, |tag| tag.to_string().len() as f64)`,
      "key: &str, options: &js::intl::LocaleOptions",
    ),
  );
  expect(js).toContain("$withBorrowMut");
  expect(lib.memo("en", { region: "GB" })).toBe("en-GB".length);
});

// `js::optional_call((tag, Locale::get_week_info), ())`, `tag.getWeekInfo?.()`,
// runs what its method runs: nothing, of a locale the program made or a JS
// caller handed over, as a property read off any object runs nothing
// (`JsForm::Get`); not a method that isn't marked, a formatter's `format`.
test("a memo whose maker calls a locale's method optionally reads its cache plainly", async () => {
  const { js, lib } = await compiled(
    "runs-nothing-optional-call",
    memo(
      "js::optional_call((tag, js::intl::Locale::get_week_info), ()).map_or(0.0, |info| info.first_day)",
      "tag: &js::intl::Locale",
      "tag",
    ),
  );
  expect(js).not.toContain("$withBorrowMut");
  const tag = new Intl.Locale("en-US") as any;
  const expected = tag.getWeekInfo?.()?.firstDay ?? 0;
  expect(lib.memo(tag)).toBe(expected);
  // A JS caller's subclass, read as any object's property is.
  class Hand extends Intl.Locale {}
  expect(lib.memo(new Hand("de-DE"))).toBe((new Intl.Locale("de-DE") as any).getWeekInfo?.()?.firstDay ?? 0);
});

test("a memo whose maker calls a method not marked optionally keeps its borrow checked", async () => {
  const { js } = await compiled(
    "runs-nothing-optional-call-given",
    memo(
      "js::optional_call((format, js::intl::DateTimeFormat::format), (Some(&now as &dyn js::intl::IntoDateOrNumber),)).map_or(0.0, |text| text.len() as f64)",
      "key: &str, format: &js::intl::DateTimeFormat, now: f64",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

// `optional_call_unchecked`'s value is asserted of another contract, so the
// method its path names may not be the binding's: its call is counted.
test("a memo whose maker calls a method optionally, unchecked, keeps its borrow checked", async () => {
  const { js } = await compiled(
    "runs-nothing-optional-call-unchecked",
    memo(
      "unsafe { js::optional_call_unchecked((tag, js::intl::Locale::get_week_info), ()) }.map_or(0.0, |info| info.first_day)",
      "tag: &js::intl::Locale",
      "tag",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

