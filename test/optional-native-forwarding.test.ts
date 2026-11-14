import {beforeAll, expect, test} from "bun:test";
import {writeFileSync} from "node:fs";
import {join} from "node:path";
import {buildReact, compiler, fixture, run, target} from "./support";

beforeAll(buildReact, 600_000);

test("both nullish Rust None inputs keep None when forwarded to an optional native argument", async () => {
  const dir = fixture("optional-native-forwarding");
  const input = join(dir, "lib.rs"), output = join(dir, "lib.js");
  writeFileSync(input, `pub fn empty(value: Option<f64>) -> bool { value.is_none() }
pub fn forwarded(value: Option<f64>) -> String { js::number::to_precision(1.25, value) }
pub fn literal_none() -> String { js::number::to_precision(1.25, None) }
`);
  run([compiler, input, "-o", output, "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const made = await import(output);
  expect(made.empty(undefined)).toBe(true);
  expect(made.empty(null)).toBe(true);
  expect(made.forwarded(undefined)).toBe(made.literalNone());
  expect(made.forwarded(null)).toBe(made.literalNone());
});
