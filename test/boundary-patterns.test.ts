// A pattern parameter JS gives keeps its shape where it's given, and each
// part it binds is checked by its path (#2403 step 2b(d)(2), ADR 1192,
// amended): `Card({ title, children })` checks `title` as
// `Card(props.title)`. Rust reads only what the pattern binds, so that's
// the whole check. Its function isn't split, so a Rust caller pays it.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, run } from "./support";

const source = `pub struct Size {
    pub w: u32,
    pub h: u32,
}

pub struct Line {
    pub from: Size,
    pub label: Option<String>,
}

pub fn area((w, h): (u32, u32)) -> u32 {
    w * h
}

pub fn sized(Size { w, h }: Size) -> u32 {
    w + h
}

pub fn nested(Line { from: Size { w, .. }, label }: Line) -> usize {
    w as usize + label.map_or(0, |l| l.len())
}

pub fn line(Line { from, label: _ }: Line) -> u32 {
    from.w
}

pub struct Rule<'a> {
    pub price: u32,
    pub rule: &'a dyn Fn(u32) -> u32,
    pub notify: &'a dyn Fn(u32),
}

pub fn ruled(Rule { price, rule, notify }: Rule) -> u32 {
    notify(price);
    price - rule(price)
}

pub fn parted(size: Size) -> u32 {
    let Size { w, h } = size;
    w * h
}

pub fn halved(size: Size) -> u32 {
    let Size { w, h } = size;
    (w + h) / 2
}

pub fn calls_halved() -> u32 {
    halved(Size { w: 2, h: 4 })
}

pub fn deref(&n: &u32) -> u32 {
    n
}

pub fn ignored(_: u32) -> u32 {
    1
}
`;

let lib: Record<string, (...args: unknown[]) => unknown>;
let js: string;

beforeAll(async () => {
  const dir = fixture("boundary-patterns");
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

test("a tuple's pattern keeps its shape, each item checked by its path", () => {
  expect(js).toContain("export function area([w, h]) {");
  expect(lib.area!([2, 3])).toBe(6);
  expect(thrown(() => lib.area!([2, -3]))).toBe("RangeError: rust-js: area(props[1]): expected a u32, got -3");
});

test("a struct's pattern keeps its shape, each field checked by its key", () => {
  expect(js).toContain("export function sized({ w, h }) {");
  expect(lib.sized!({ w: 2, h: 3 })).toBe(5);
  expect(thrown(() => lib.sized!({ w: 2 }))).toBe("TypeError: rust-js: sized(props.h): expected a u32, got undefined");
});

test("a nested pattern checks only what it binds, an Option's None allowed", () => {
  expect(lib.nested!({ from: { w: 2 }, label: null })).toBe(2);
  expect(lib.nested!({ from: { w: 2, h: "unread" }, label: "ab" })).toBe(4);
  expect(thrown(() => lib.nested!({ from: { w: "2" }, label: null }))).toBe('TypeError: rust-js: nested(props.from.w): expected a u32, got "2"');
  expect(thrown(() => lib.nested!({ from: { w: 2 }, label: 3 }))).toBe("TypeError: rust-js: nested(props.label): expected a String, got 3");
});

test("a part bound whole is checked whole, its path in the error", () => {
  expect(thrown(() => lib.line!({ from: { w: 1, h: "1" } }))).toBe('TypeError: rust-js: line(props.from.h): expected a u32, got "1"');
});

test("a reference's pattern and a `_` are checked", () => {
  expect(lib.deref!(4)).toBe(4);
  expect(thrown(() => lib.deref!(1.5))).toBe("RangeError: rust-js: deref(props): expected a u32, got 1.5");
  expect(thrown(() => lib.ignored!("x"))).toBe('TypeError: rust-js: ignored(_): expected a u32, got "x"');
});

test("a null given takes JS's own TypeError before Rust runs", () => {
  expect(thrown(() => lib.sized!(null))).toStartWith("TypeError: ");
});

test("a callback a pattern binds is checked a function, wrapped where its result is read", () => {
  expect(js).toContain('rule = $checkCallback(rule, "ruled(props.rule)", $checkU32);');
  expect(js).toContain('$checkCallback(notify, "ruled(props.notify)");');
  expect(lib.ruled!({ price: 10, rule: () => 3, notify: () => {} })).toBe(7);
  expect(thrown(() => lib.ruled!({ price: 10, rule: () => -1, notify: () => {} }))).toBe("RangeError: rust-js: ruled(props.rule) returned -1, expected a u32");
  expect(thrown(() => lib.ruled!({ price: 10, rule: () => 3, notify: 1 }))).toBe("TypeError: rust-js: ruled(props.notify): expected a function, got 1");
});

test("a named parameter its body takes apart is checked whole, its shape as it was", () => {
  expect(js).toContain('export function parted(size) {\n  $checks("parted(size)", size, checkSize);\n  const { w, h } = size;');
  expect(thrown(() => lib.parted!({ w: 2 }))).toBe("TypeError: rust-js: parted(size.h): expected a u32, got undefined");
  expect(lib.callsHalved!()).toBe(3);
});
