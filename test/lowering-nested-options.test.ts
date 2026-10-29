// What the compiler writes of an `Option` of an `Option` a variable holds
// as its payload (ADR 1017): its `None` `undefined`, its `Some(None)`
// `null`. Each test is a program of its own, compiled.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

// `matches!(last, Some(None))` is `last === null`, its two tests one where
// they're of one place; a test of `undefined` beside another place's of
// `null` stays two (ADR 1017, amended).
test("an Option of an Option's Some(None) is === null", () => {
  const dir = fixture("nested-option-null");
  writeFileSync(
    join(dir, "lib.rs"),
    `fn counted(values: &[Option<u32>], plain: Option<u32>, wanted: usize) -> u32 {
    let mut last = None;
    let mut seen = 0;
    for (i, value) in values.iter().enumerate() {
        if i == wanted {
            last = Some(*value);
        }
        if matches!(last, Some(None)) {
            seen += 1;
        }
        if matches!(last, Some(_)) && plain.is_none() {
            seen += 10;
        }
    }
    seen
}

pub fn shown(values: &[Option<u32>], plain: Option<u32>) -> String {
    format!("{}", counted(values, plain, 1))
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("if (last === null) {");
  expect(js).toContain("last !== undefined && plain == null");
});
