// What a JS caller gives a Rust function, checked against its Rust type
// before the body runs (#2403 step 2a, ADR 1192): a number, a `bool`, a
// `char`, a string, a fieldless enum, and an `Option` of one, each as its
// parameter's form reads `None`. A value outside the type throws a
// `TypeError` of the wrong kind or a `RangeError` of a number out of range,
// naming the function and the parameter, what was expected and what was
// given; one inside it runs as before.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

const source = `pub enum Status {
    Open,
    #[cfg_attr(rust_js, rust_js::name = "shipped")]
    Shipped,
}

pub fn double(n: u32) -> u32 {
    n * 2
}

pub fn wide(n: u64, s: i128, x: f32, y: f64) -> f64 {
    n as f64 + s as f64 + x as f64 + y
}

pub fn label(name: &str, owned: String, c: char, on: bool, status: Status) -> String {
    let s = match status {
        Status::Open => "open",
        Status::Shipped => "shipped",
    };
    format!("{name}{owned}{c}{on}{s}")
}

pub fn pad(text: &str, width: Option<u32>) -> String {
    format!("{text:>w$}", w = width.unwrap_or(12) as usize)
}

#[cfg_attr(rust_js, rust_js::nullable(note))]
pub fn noted(note: Option<&str>) -> usize {
    note.map_or(0, str::len)
}

// A \`Vec\` and a tuple, checked whole (2b(b), test/boundary-deep.test.ts).
pub fn untouched(items: Vec<u32>, pair: (u32, u32)) -> u32 {
    items.len() as u32 + pair.0
}

fn private_double(n: u32) -> u32 {
    n * 2
}

pub fn through(n: u32) -> u32 {
    private_double(n) + double(n)
}
`;

let lib: Record<string, (...args: unknown[]) => unknown>;
let js: string;

beforeAll(async () => {
  const dir = fixture("boundary-checks");
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  lib = await import(join(dir, "lib.js"));
  js = readFileSync(join(dir, "lib.js"), "utf8");
}, 600_000);

const thrown = (call: () => unknown) => {
  try {
    call();
  } catch (error) {
    return `${(error as Error).name}: ${(error as Error).message}`;
  }
  return "nothing thrown";
};

test("each parameter checked is one statement, first in the body", () => {
  expect(js).toContain('export function double(n) {\n  $checkU32(n, "double(n)");\n');
  expect(js).toContain('$checkU32(width, "pad(width)", "nullish");');
  expect(js).toContain('$checkStr(note, "noted(note)", "null");');
  expect(js).toContain('$checkOneOf(status, ["Open", "shipped"], "label(status)");');
});

test("an integer of 32 bits or fewer: a whole number in its range", () => {
  expect(lib.double!(21)).toBe(42);
  expect(thrown(() => lib.double!(-1))).toBe("RangeError: rust-js: double(n): expected a u32, got -1");
  expect(thrown(() => lib.double!(3.5))).toBe("RangeError: rust-js: double(n): expected a u32, got 3.5");
  expect(thrown(() => lib.double!(2 ** 32))).toBe("RangeError: rust-js: double(n): expected a u32, got 4294967296");
  expect(thrown(() => lib.double!("2"))).toBe('TypeError: rust-js: double(n): expected a u32, got "2"');
  expect(thrown(() => lib.double!())).toBe("TypeError: rust-js: double(n): expected a u32, got undefined");
});

test("a 64- or 128-bit integer is a bigint in its range; an f64 any number; an f32 one it holds", () => {
  expect(lib.wide!(1n, -2n, 0.5, Number.NaN)).toBeNaN();
  expect(thrown(() => lib.wide!(2n ** 64n, 0n, 0, 0))).toBe("RangeError: rust-js: wide(n): expected a u64, got 18446744073709551616n");
  expect(thrown(() => lib.wide!(1, 0n, 0, 0))).toBe("TypeError: rust-js: wide(n): expected a u64, got 1");
  expect(thrown(() => lib.wide!(0n, -(2n ** 127n) - 1n, 0, 0))).toContain("RangeError: rust-js: wide(s): expected an i128");
  expect(thrown(() => lib.wide!(0n, 0n, 0.1, 0))).toBe("RangeError: rust-js: wide(x): expected an f32, got 0.1");
  expect(thrown(() => lib.wide!(0n, 0n, 0, "1"))).toBe('TypeError: rust-js: wide(y): expected an f64, got "1"');
});

test("a string is well-formed, a char one code point, a bool a boolean, an enum one of its names", () => {
  expect(lib.label!("a", "b", "😀", true, "shipped")).toBe("ab😀trueshipped");
  expect(thrown(() => lib.label!("\uD800", "b", "c", true, "Open"))).toBe('TypeError: rust-js: label(name): expected a String, got "\\ud800"');
  expect(thrown(() => lib.label!("a", 1, "c", true, "Open"))).toBe("TypeError: rust-js: label(owned): expected a String, got 1");
  expect(thrown(() => lib.label!("a", "b", "ab", true, "Open"))).toBe('TypeError: rust-js: label(c): expected a char, got "ab"');
  expect(thrown(() => lib.label!("a", "b", "", true, "Open"))).toBe('TypeError: rust-js: label(c): expected a char, got ""');
  expect(thrown(() => lib.label!("a", "b", "c", 1, "Open"))).toBe("TypeError: rust-js: label(on): expected a bool, got 1");
  expect(thrown(() => lib.label!("a", "b", "c", true, "lost"))).toBe('TypeError: rust-js: label(status): expected "Open" | "shipped", got "lost"');
  // A fieldless enum's is one statement of its names, no checker of its
  // own, as a tagged enum's is (ADR 1194).
  expect(js).toContain('  $checkOneOf(status, ["Open", "shipped"], "label(status)");\n');
  expect(js).not.toContain("function checkStatus(");
});

test("an Option's None is what its form reads as one", () => {
  expect([lib.pad!("tea"), lib.pad!("tea", null), lib.pad!("tea", 6)]).toEqual(["         tea", "         tea", "   tea"]);
  expect(thrown(() => lib.pad!("tea", -6))).toBe("RangeError: rust-js: pad(width): expected a u32, got -6");
  expect([lib.noted!(null), lib.noted!("abc")]).toEqual([0, 3]);
  // A nullable parameter's None is `null`, never left out.
  expect(thrown(() => lib.noted!(undefined))).toBe("TypeError: rust-js: noted(note): expected a String, got undefined");
});

test("what holds other values is checked whole, a private function is Rust's, and a Rust caller is unaffected", () => {
  expect(lib.untouched!([1, 2], [3, 4])).toBe(5);
  // Its place written once, each parameter named in it (#2403, the lead's (A)).
  expect(js).toContain('  $checks(\n    "untouched(items, pair)",\n    items,\n');
  expect(thrown(() => lib.untouched!([1, -2], [3, 4]))).toBe("RangeError: rust-js: untouched(items[1]): expected a u32, got -2");
  expect(js).not.toMatch(/function privateDouble\(n\) \{\n {2}\$check/);
  expect(lib.through!(5)).toBe(20);
});
