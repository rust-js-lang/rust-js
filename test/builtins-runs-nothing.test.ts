// A builtin that runs none of the program's code, `#[rust_js::runs_nothing]`
// (ADR 0867), or only what it's given, `#[rust_js::runs_only_given]` (ADR
// 0898): a `RefCell` borrowed across its call isn't counted, so a memo whose
// maker calls only such builtins reads its cache plainly. The marks of
// generated builtins are builtins/policy.ts' `RUNS`, each with its spec step;
// `RegExp`'s are written by hand (#2082). One whose maker calls the program
// back, a JS callback, keeps the check, and its re-entry panics as Rust's.

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
