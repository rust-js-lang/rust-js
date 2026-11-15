// What a binding's `#[rust_js::nullable(..)]` parameter is given (ADR
// 0275): its `None` `null`, and what's `null` or a value already, a
// nullable function's result or a nullable field's read, as it is, no `??
// null` (ADR 0949, #1932). Each test is a program of its own, compiled and
// run.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

test("a binding's nullable parameter is given what's null or a value already as it is", async () => {
  const dir = fixture("nullable-param-nulled");
  writeFileSync(
    join(dir, "lib.rs"),
    `#[cfg_attr(rust_js, rust_js::link_name = "JSON.stringify")]
#[cfg_attr(rust_js, rust_js::nullable(replacer))]
fn stringify(value: &[u32], replacer: Option<&[&str]>, space: u32) -> String {
    unreachable!()
}

pub struct Settings {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub keys: Option<&'static [&'static str]>,
}

#[cfg_attr(rust_js, rust_js::nullable)]
pub fn chosen(all: bool) -> Option<&'static [&'static str]> {
    if all { None } else { Some(&["a"]) }
}

pub fn of_a_result(items: &[u32], all: bool) -> String {
    stringify(items, chosen(all), 0)
}

pub fn of_a_field(items: &[u32], settings: &Settings) -> String {
    stringify(items, settings.keys, 0)
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return JSON.stringify(items, chosen(all), 0);");
  expect(js).toContain("return JSON.stringify(items, settings.keys, 0);");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.ofAResult([1], true), lib.ofAResult([1], false)]).toEqual(["[1]", "[1]"]);
  expect([lib.ofAField([2], { keys: null }), lib.ofAField([2], { keys: ["a"] })]).toEqual(["[2]", "[2]"]);
});
