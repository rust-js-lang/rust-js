// Oracle C's parts (ADR 1204): the values the export net makes of a
// crate's types from their records (test/generated-calls.ts `values`),
// each a JS literal of its guide row's form beside the Rust literal of the
// same value, and each type it makes none of counted by why; and the
// counting proxies the net and getter-reads give them through
// (test/counting-proxies.ts). The export net runs them, nightly; this file
// holds them on each push.
import { expect, test } from "bun:test";
import { countingProxies } from "./counting-proxies";
import { values, type Giving, type Tree } from "./generated-calls";

const u8: Tree = { kind: "int", rust: "u8" };
const of = (rust: string): Tree => ({ kind: "named", crate: "dep", rust: [rust] });
const giving = (): Giving => ({
  types: {
    "dep::Point": { path: "dep::Point", fields: [{ rust: "max_len", js: "maxLen", ty: u8 }, { rust: "parent", js: "parent", ty: { kind: "option", of: u8 }, none: "null" }] },
    "dep::Pair": { path: "dep::Pair", positional: [u8, { kind: "bool" }] },
    "dep::Escape": { path: "dep::Escape", unit: { kind: "str", value: "escape-key" } },
    "dep::Shape": { path: "dep::Shape", variants: { form: "tagged", key: "TAG", variants: [
      { rust: "Empty", tag: { kind: "str", value: "Empty" } },
      { rust: "Square", tag: { kind: "str", value: "Square" }, fields: [{ rust: "0", js: "_0", ty: u8 }] },
    ] } },
    "dep::Size": { path: "dep::Size", variants: { form: "untagged", key: "TAG", variants: [
      { rust: "Exact", tag: { kind: "str", value: "Exact" }, fields: [{ rust: "0", js: "_0", ty: { kind: "bool" } }] },
    ] } },
    "dep::Clicks": { path: "dep::Clicks", sealed: true, fields: [{ rust: "n", js: "n", ty: u8 }] },
    "dep::Counter": { path: "dep::Counter", class: true, fields: [{ rust: "n", js: "n", ty: u8 }] },
    "dep::Hidden": { fields: [{ rust: "n", js: "n", ty: u8 }] },
  },
  missing: {},
});

test("a crate's type's values are its guide row's JS beside the same Rust", () => {
  const g = giving();
  expect(values(of("Point"), false, g)?.slice(0, 2)).toEqual([
    { js: '{ "maxLen": 0, "parent": null }', rust: "dep::Point { max_len: 0u8, parent: None::<u8> }" },
    { js: '{ "maxLen": 1, "parent": 0 }', rust: "dep::Point { max_len: 1u8, parent: Some(0u8) }" },
  ]);
  expect(values(of("Pair"), false, g)?.[0]).toEqual({ js: "[0, true]", rust: "dep::Pair(0u8, true)" });
  expect(values(of("Escape"), false, g)).toEqual([{ js: '"escape-key"', rust: "dep::Escape" }]);
  expect(values(of("Shape"), false, g)?.slice(0, 2)).toEqual([
    { js: '"Empty"', rust: "dep::Shape::Empty" },
    { js: '{ "TAG": "Square", "_0": 0 }', rust: "dep::Shape::Square(0u8)" },
  ]);
  expect(values(of("Size"), false, g)?.[0]).toEqual({ js: "true", rust: "dep::Size::Exact(true)" });
  expect(values({ kind: "vec", of: of("Pair"), borrowed: true }, false, g)?.[0]).toEqual({ js: "[]", rust: "&Vec::<dep::Pair>::from([])[..]" });
});

test("a type it makes no value of is counted by why, never silently", () => {
  const g = giving();
  for (const name of ["Clicks", "Counter", "Hidden"]) expect(values(of(name), false, g)).toBeUndefined();
  expect(values({ kind: "named", crate: "dep", rust: ["Pair"], args: [u8] }, false, g)).toBeUndefined();
  expect(g.missing).toEqual({ sealed: 1, class: 1, unnamed: 1, generic: 1 });
});

test("a counting proxy logs each read, test and write by its path, and gives what it holds", () => {
  const { log, counted } = countingProxies();
  const point = counted({ x: 1, inner: { y: 2 } }, "arg0") as { x: number; inner: { y: number }; z?: number };
  expect(point.x).toBe(1);
  expect(point.inner.y).toBe(2);
  expect("x" in point).toBe(true);
  point.z = 3;
  expect(log).toEqual(["get arg0.x", "get arg0.inner", "get arg0.inner.y", "has arg0.x", "set arg0.z = 3"]);
});
