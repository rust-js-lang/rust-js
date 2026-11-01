// The js crate's bindings of JS's globals each typed as TypeScript's lib
// types them, never narrower: React's Flight client's `Array.prototype`,
// `BigInt(text)`, `new Date(Date.parse(text))` and `Symbol()` (rd399).
// Each test is a program of its own, compiled, and its JS run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildReact, compiler, fixture, root, target } from "./support";

beforeAll(buildReact, 600_000);

const withJs = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `source` compiled, its JS, and the module it is, loaded. */
async function compiled(name: string, source: string): Promise<{ js: string; lib: any }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), source);
  const built = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withJs], root, 600_000);
  expect(built.code, built.stderr).toBe(0);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

// ReactFlightClient's parseModelString: `$D` is a date, `$n` a bigint, each
// of the text after its tag; `isArray` asks of a value's prototype. A
// symbol of no description is `Symbol()`, TypeScript's optional
// `string | number`, as its RESPONSE_SYMBOL is.
test("Array.prototype, BigInt, Date.parse and Symbol are typed as TypeScript's lib", async () => {
  const { js, lib } = await compiled(
    "js-globals-flight",
    `use js::{BigInt, Date, Symbol, Unknown, array, date, symbol};

pub fn is_plain_array(value: &Unknown) -> bool {
    js::object::get_prototype_of(Some(value)) == Some(array::prototype)
}

pub fn big(text: &str) -> &'static BigInt {
    js::big_int(&js::string::slice(text, 2.0, None))
}

pub fn of_number(value: f64) -> &'static BigInt {
    js::big_int(value)
}

pub fn dated(text: &str) -> &'static Date {
    date::new(Some(&date::parse(&js::string::slice(text, 2.0, None))))
}

pub fn parsed(text: &str) -> f64 {
    date::parse(text)
}

pub fn symbols() -> (&'static Symbol, &'static Symbol, &'static Symbol) {
    (symbol::new(None), symbol::new(Some(&"react.response")), symbol::new(Some(&1.0)))
}
`,
  );
  expect(js).toContain("Object.getPrototypeOf(value) === Array.prototype");
  expect(js).toContain("BigInt(text.slice(2))");
  expect(js).toContain("BigInt(value)");
  expect(js).toContain("new Date(Date.parse(text.slice(2)))");
  expect(js).toContain('[Symbol(), Symbol("react.response"), Symbol(1)]');
  expect([lib.isPlainArray([1]), lib.isPlainArray({}), lib.isPlainArray(new Uint8Array(1))]).toEqual([true, false, false]);
  expect(lib.big("$n123456789012345678901234567890")).toBe(123456789012345678901234567890n);
  expect(lib.ofNumber(3)).toBe(3n);
  expect(() => lib.ofNumber(1.5)).toThrow(RangeError);
  expect(() => lib.big("$nabc")).toThrow(SyntaxError);
  expect(lib.dated("$D2024-01-02T03:04:05.000Z").toISOString()).toBe("2024-01-02T03:04:05.000Z");
  expect(Number.isNaN(lib.parsed("not a date"))).toBe(true);
  const [none, named, numbered] = lib.symbols();
  expect([typeof none, none.description, named.description, numbered.description]).toEqual([
    "symbol",
    undefined,
    "react.response",
    "1",
  ]);
});
