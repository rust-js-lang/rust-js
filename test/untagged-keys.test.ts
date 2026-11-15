// An untagged enum's variant told apart by its key alone,
// `#[rust_js::test(key)]` on its struct's field: an object that has it,
// JS's `"bound" in move`, inherited too, its value unread, as TypeScript's
// `in` narrows a union (ADR 1162, #1846). A bare `#[rust_js::test]` tells
// it by its value's kind, which reads it; the two may tell one object
// apart otherwise, so each says which it is.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { underLoad } from "../scripts/timing";

beforeAll(buildReact, 600_000);

const js = ["--", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

const program = `pub struct Bound {
    #[cfg_attr(rust_js, rust_js::test(key))]
    pub bound: String,
}

pub struct Direction {
    pub direction: f64,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Move {
    Bound(Bound),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Direction(Direction),
}

pub fn step(given: &Move) -> String {
    match given {
        Move::Bound(bound) => format!("bound {}", bound.bound),
        Move::Direction(direction) => format!("direction {}", direction.direction),
    }
}

pub struct Kinded {
    #[cfg_attr(rust_js, rust_js::test)]
    pub bound: String,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ByKind {
    Kinded(Kinded),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Direction(Direction),
}

pub fn by_kind(given: &ByKind) -> &'static str {
    match given {
        ByKind::Kinded(_) => "kinded",
        ByKind::Direction(_) => "direction",
    }
}

pub fn by_key(given: &Move) -> &'static str {
    match given {
        Move::Bound(_) => "bound",
        Move::Direction(_) => "direction",
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Valued {
    Number(f64),
    Bound(Bound),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Direction(Direction),
}

pub fn valued(given: &Valued) -> &'static str {
    match given {
        Valued::Number(_) => "number",
        Valued::Bound(_) => "bound",
        Valued::Direction(_) => "direction",
    }
}
`;

test("a variant told by its key is JS's in, its value unread, in the order upstream reads", async () => {
  const dir = fixture("untagged-keys");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js]);
  const out = readFileSync(join(dir, "lib.js"), "utf8");
  expect(out).toContain('"bound" in given');
  const lib = await import(join(dir, "lib.js"));
  // A Proxy's traps, each its key: `has` alone tells it, `get` reads it in its arm.
  const traced = (target: Record<string, unknown>) => {
    const trace: string[] = [];
    const proxy = new Proxy(target, {
      has: (t, key) => (trace.push(`has ${String(key)}`), key in t),
      get: (t, key) => (typeof key === "string" && trace.push(`get ${key}`), t[key as string]),
    });
    return { proxy, trace };
  };
  const bound = traced({ bound: "edge" });
  expect([lib.step(bound.proxy), bound.trace]).toEqual(["bound edge", ["has bound", "get bound"]]);
  const direction = traced({ direction: 2 });
  expect([lib.step(direction.proxy), direction.trace]).toEqual(["direction 2", ["has bound", "get direction"]]);
  // A getter that throws, of a key the test asks no value of.
  const throwing = { get bound(): string { throw new Error("read"); } };
  expect(lib.byKey(throwing)).toBe("bound");
  // An inherited key, as `in` counts it.
  expect(lib.byKey(Object.create({ bound: "x" }))).toBe("bound");
  // One object, told apart otherwise by its key and by its value's kind: a
  // `bound` that isn't a string is a `Bound` to `in` and not to a kind test.
  expect([lib.byKey({ bound: 5 }), lib.byKind({ bound: 5 })]).toEqual(["bound", "direction"]);
  // Beside a primitive variant, its object test first: no `in` of a number.
  expect([lib.valued(5), lib.valued({ bound: "x" }), lib.valued({ direction: 1 })]).toEqual(["number", "bound", "direction"]);
}, underLoad(120_000));

test("two variants told by one key are refused", () => {
  const dir = fixture("untagged-keys-refused");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub struct A {\n    #[cfg_attr(rust_js, rust_js::test(key))]\n    pub k: f64,\n}\npub struct B {\n    #[cfg_attr(rust_js, rust_js::test(key))]\n    pub k: String,\n}\n#[cfg_attr(rust_js, rust_js::untagged)]\npub enum E {\n    A(A),\n    B(B),\n}\npub fn e(given: &E) -> bool {\n    matches!(given, E::A(_))\n}\n`,
  );
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js], root, 60_000);
  expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining("an object that has a `k`")]);
}, underLoad(60_000));

test("a field's test of another word is refused", () => {
  const dir = fixture("untagged-keys-word");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub struct A {\n    #[cfg_attr(rust_js, rust_js::test(value))]\n    pub k: f64,\n}\npub fn a() -> A {\n    A { k: 1.0 }\n}\n`,
  );
  const p = runSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js], root, 60_000);
  expect([p.code === 0, p.stderr]).toEqual([false, expect.stringContaining("or `#[rust_js::test(key)]`, by its key alone")]);
}, underLoad(60_000));
