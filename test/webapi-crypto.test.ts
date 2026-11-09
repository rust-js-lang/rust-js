// webapi's crypto, against the real API: Node's and Chromium's.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chromium } from "playwright";

import { compiler, fixture, run, target } from "./support";
import { underLoad } from "../scripts/timing";

// `crypto.getRandomValues(array)` fills the array it's given and gives it
// back, TypeScript's `getRandomValues<T extends ..>(array: T): T`: the
// type that parameter takes, `A: IntoArrayBufferView`, the argument itself,
// against the real API, Node's and Chromium's (#1103).
test("getRandomValues fills the array it's given and gives it back", async () => {
  const dir = fixture("webapi-random-values");
  writeFileSync(join(dir, "lib.rs"), `use js::{DataView, Uint8Array, uint32_array};
use webapi::global::crypto;

pub fn fill(bytes: &Uint8Array) -> &Uint8Array {
    crypto().get_random_values(bytes)
}

pub fn fill_new(length: f64) -> f64 {
    let words = crypto().get_random_values(uint32_array::new(length));
    words.length()
}

pub fn fill_view(view: &DataView) -> &DataView {
    crypto().get_random_values(view)
}
`);
  const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function fill(bytes) {\n  return crypto.getRandomValues(bytes);\n}");
  expect(js).toContain("const words = crypto.getRandomValues(new Uint32Array(length));\n  return words.length;");
  // Each gives back the array itself, filled: 64 bytes all zero is a
  // chance in 2^512. WebIDL's type takes any view, as TypeScript's does;
  // one not of integers throws, as JS's call does.
  const check = `const lib = await import(${JSON.stringify(join(dir, "lib.js"))});
const bytes = new Uint8Array(64);
const view = new DataView(new ArrayBuffer(8));
let thrown;
try { lib.fillView(view); } catch (e) { thrown = e.name; }
console.log(JSON.stringify([lib.fill(bytes) === bytes, bytes.some((b) => b !== 0), lib.fillNew(3), thrown]));`;
  const expected = [true, true, 3, "TypeMismatchError"];
  writeFileSync(join(dir, "check.mjs"), check);
  const node = Bun.spawnSync(["node", join(dir, "check.mjs")], { stderr: "pipe" });
  expect(node.stderr.toString()).toBe("");
  expect(JSON.parse(node.stdout.toString())).toEqual(expected);
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const result = await page.evaluate(async (js) => {
      const url = URL.createObjectURL(new Blob([js], { type: "text/javascript" }));
      const lib = await import(url);
      const bytes = new Uint8Array(64);
      const view = new DataView(new ArrayBuffer(8));
      let thrown;
      try {
        lib.fillView(view);
      } catch (e) {
        thrown = (e as Error).name;
      }
      return [lib.fill(bytes) === bytes, bytes.some((b) => b !== 0), lib.fillNew(3), thrown];
    }, js);
    expect(result).toEqual(expected);
  } finally {
    await browser.close();
  }
}, underLoad(60_000));
