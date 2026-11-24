// A function JS gives Rust is a function, and where what it gives back is
// checked, it's wrapped where the function starts, each call's result
// checked as it returns (#2403 step 2c(a), ADR 1195): a JS callback that
// gives a `u32` -100 throws there, naming it. One whose result Rust
// ignores is given as it is, as is a Rust caller's closure. And what the
// crate's own binding gives back is checked where its call returns, but
// text, which waits on the user's decision (ADR 1195).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

const source = `pub struct Item {
    pub qty: u32,
}

pub fn discounted(price: u32, rule: impl Fn(u32) -> u32) -> u32 {
    price - rule(price)
}

pub fn each(items: &[u32], visit: &dyn Fn(u32)) -> u32 {
    let mut seen = 0;
    for &item in items {
        visit(item);
        seen += 1;
    }
    seen
}

pub fn pick(make: fn() -> Item) -> u32 {
    make().qty
}

pub fn twice(n: u32) -> u32 {
    discounted(n, |p| p / 2)
}

#[cfg_attr(rust_js, rust_js::link_name = "./box.js#width")]
fn width(good: bool) -> u32 {
    unreachable!("{good}")
}

#[cfg_attr(rust_js, rust_js::link_name = "./box.js#label")]
fn label(good: bool) -> String {
    unreachable!("{good}")
}

#[cfg_attr(rust_js, rust_js::link_name = "./box.js#maybe")]
#[cfg_attr(rust_js, rust_js::nullable)]
fn maybe(good: bool) -> Option<u32> {
    unreachable!("{good}")
}

pub fn measured(good: bool) -> u32 {
    width(good) + maybe(good).unwrap_or(0)
}

pub fn labelled(good: bool) -> String {
    label(good)
}

#[cfg_attr(rust_js, rust_js::link_name = "get rustJsDepth")]
fn depth() -> f64 {
    unreachable!()
}

// What a body asks the type of first, its test its check (#2403).
pub fn deep() -> f64 {
    if js::is_number(depth) { depth() } else { 1.0 }
}

// And one it doesn't.
pub fn deeper() -> f64 {
    depth() + 1.0
}
`;

let lib: Record<string, (...args: unknown[]) => unknown>;
let js: string;

beforeAll(async () => {
  buildReact();
  const dir = fixture("boundary-callbacks");
  writeFileSync(join(dir, "lib.rs"), source);
  writeFileSync(
    join(dir, "box.js"),
    'export const width = (good) => (good ? 7 : -1);\nexport const label = (good) => (good ? "seven" : 7);\nexport const maybe = (good) => (good ? null : "x");\n',
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
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

test("a callback whose result is checked is wrapped at entry, each result checked as it returns", () => {
  expect(js).toContain('rule = $checkCallback(rule, "discounted(rule)", $checkU32);');
  expect(lib.discounted!(10, (p: number) => p / 2)).toBe(5);
  expect(thrown(() => lib.discounted!(10, () => -100))).toBe("RangeError: rust-js: discounted(rule) returned -100, expected a u32");
  expect(thrown(() => lib.discounted!(10, () => "2"))).toBe('TypeError: rust-js: discounted(rule) returned "2", expected a u32');
});

test("a value that isn't a function is refused before the body runs", () => {
  expect(thrown(() => lib.discounted!(10, 3))).toBe("TypeError: rust-js: discounted(rule): expected a function, got 3");
  expect(thrown(() => lib.each!([1], "f"))).toBe('TypeError: rust-js: each(visit): expected a function, got "f"');
});

test("one whose result Rust ignores is given as it is", () => {
  expect(js).toContain('$checkCallback(visit, "each(visit)");');
  expect(js).not.toContain("visit = $checkCallback");
  const seen: number[] = [];
  expect(lib.each!([1, 2], (n: number) => seen.push(n))).toBe(2);
  expect(seen).toEqual([1, 2]);
});

test("a fn pointer's struct result is checked whole, its path in the error", () => {
  expect(lib.pick!(() => ({ qty: 3 }))).toBe(3);
  expect(thrown(() => lib.pick!(() => ({ qty: "3" })))).toBe('TypeError: rust-js: pick(make) returned "3" (qty), expected a u32');
});

test("a Rust caller's closure is given as it is, to the body", () => {
  expect(js).toMatch(/export function twice\(n\) \{\n {2}\$checkU32\(n, "twice\(n\)"\);\n {2}return discounted\$\(n, /);
  expect(lib.twice!(10)).toBe(5);
});

test("the crate's own binding's result is checked where its call returns", () => {
  expect(js).toContain('$checkResult(width(good), "./box.js#width", $checkU32)');
  expect(lib.measured!(true)).toBe(7);
  expect(thrown(() => lib.measured!(false))).toBe("RangeError: rust-js: ./box.js#width returned -1, expected a u32");
});

test("a binding's result its body type-tests first isn't checked: the test is its check", () => {
  const global = globalThis as Record<string, unknown>;
  global.rustJsDepth = "deep";
  expect(lib.deep!()).toBe(1);
  expect(js).toContain('export function deep() {\n  return typeof rustJsDepth === "number" ? rustJsDepth : 1;\n}');
  expect(thrown(() => lib.deeper!())).toBe('TypeError: rust-js: get rustJsDepth returned "deep", expected an f64');
  global.rustJsDepth = 2;
  expect(lib.deep!()).toBe(2);
  delete global.rustJsDepth;
});

test("but text it gives back waits on the user's decision", () => {
  expect(js).not.toContain('"./box.js#label"');
  expect(lib.labelled!(false)).toBe(7);
});
