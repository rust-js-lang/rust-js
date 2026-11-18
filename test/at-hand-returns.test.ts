// What a function returns, and what passes a value on, at hand where what
// it's made of is (#2116, ADR 1173): a call of a function only calls reach
// (`only_called`) whose every value is a builtin's; `catch_unwind`'s value,
// an `Option`'s or a `Result`'s fallback, `unwrap_or(v)`, `or_else(|| ..)`;
// and `value.method?.()` of a method that creates its result. A memo whose
// maker reads such a value under its cache's borrow reads it plainly; one
// that may be a JS caller's keeps the check. Each program is compiled and
// run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

async function compiled(name: string, source: string) {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// A memo whose maker is `maker`, `more` the crate's other items.
const memo = (maker: string, more = "", signature = "key: &str") => `use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}
${more}
pub fn memo(${signature}) -> f64 {
    CACHE.with_borrow_mut(|cache| *cache.entry(key.to_string()).or_insert_with(|| ${maker}))
}
`;

const made = `
fn made(key: &str) -> &'static js::intl::Locale {
    js::intl::locale::new(key, None).unwrap()
}
`;

test("a locale a private function makes and returns reads plainly", async () => {
  const { js, lib } = await compiled("returns-made", memo("made(key).maximize().to_string().len() as f64", made));
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo("en")).toBe(new Intl.Locale("en").maximize().toString().length);
});

test("a locale a function also used as a value returns keeps the borrow checked", async () => {
  const { js } = await compiled(
    "returns-made-escapes",
    memo(
      "made(key).maximize().to_string().len() as f64",
      `${made}
pub fn maker() -> fn(&str) -> &'static js::intl::Locale {
    made
}
`,
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

test("a locale a function may return of a JS caller's keeps the borrow checked", async () => {
  const { js } = await compiled(
    "returns-given",
    memo(
      "chosen(key, given).maximize().to_string().len() as f64",
      `
fn chosen(key: &str, given: &'static js::intl::Locale) -> &'static js::intl::Locale {
    if key.is_empty() {
        return given;
    }
    js::intl::locale::new(key, None).unwrap()
}
`,
      "key: &str, given: &'static js::intl::Locale",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

test("a locale catch_unwind gives, or its fallback, reads plainly", async () => {
  const { js, lib } = await compiled(
    "returns-catch-unwind",
    memo(
      `std::panic::catch_unwind(|| js::intl::locale::new(key, None).unwrap())
            .unwrap_or_else(|_| js::intl::locale::new("en", None).unwrap())
            .maximize()
            .to_string()
            .len() as f64`,
    ),
  );
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo("de")).toBe(new Intl.Locale("de").maximize().toString().length);
});

test("a locale whose fallback is a JS caller's keeps the borrow checked", async () => {
  const { js } = await compiled(
    "returns-catch-unwind-given",
    memo(
      `std::panic::catch_unwind(|| js::intl::locale::new(key, None).unwrap())
            .unwrap_or_else(|_| given)
            .maximize()
            .to_string()
            .len() as f64`,
      "",
      "key: &str, given: &'static js::intl::Locale",
    ),
  );
  expect(js).toContain("$withBorrowMut");
});

test("the week info a made locale's getWeekInfo?.() makes reads plainly", async () => {
  const { js, lib } = await compiled(
    "returns-week-info",
    memo(
      `js::intl::locale::new(key, None).map_or(0.0, |tag| {
            js::optional_call((tag, js::intl::Locale::get_week_info), ()).map_or(0.0, |info| info.first_day)
        })`,
    ),
  );
  expect(js).not.toContain("$withBorrowMut");
  expect(lib.memo("en-US")).toBe((new Intl.Locale("en-US") as any).getWeekInfo?.()?.firstDay ?? 0);
});
