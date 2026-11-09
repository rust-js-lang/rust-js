// What the consumer net (test/consumer-net.ts) covers is what std has: each
// method std's own source has that takes an iterator by value, as its
// receiver or an argument, a consumer of `Iterator`'s, each `Extend`,
// `FromIterator`, `Sum` and `Product`, and each other function, is a
// template's, or excluded with why. A toolchain that adds one fails here
// until a template covers it, and one that takes one away until its
// template or exclusion goes. The net itself runs in
// test/iterator-matrix.test.ts, `ITERATOR_TABLE=consumers`.

import { expect, test } from "bun:test";

import { consumerCases, excluded, stdMethods, templates, whyNot } from "./consumer-net";

const found = stdMethods();
const ids = new Set(found.map((m) => m.id));
const covered = new Set(templates.flatMap((t) => t.covers));

test("std's source is read: its consumers, its impls and its functions are found", () => {
  for (const id of ["Iterator::count", "Iterator::eq", "DoubleEndedIterator::rfold", "FromIterator<T> for Vec<T>", "Extend<(K, V)> for HashMap<K, V, S, A>", "Vec<T, A>::splice", "Sum<Option<U>> for Option<T>"]) {
    expect(ids.has(id), id).toBe(true);
  }
  // What takes `&mut self`, and no iterator, isn't one.
  expect(ids.has("Iterator::nth")).toBe(false);
  expect(found.length).toBeGreaterThan(150);
});

test("each std method that takes an iterator by value is a template's, or excluded with why", () => {
  const missed = found.filter((m) => !covered.has(m.id) && whyNot(m) === undefined).map((m) => `${m.id} (${m.file}): cover it in test/consumer-net.ts, or say in \`excluded\` why not`);
  expect(missed).toEqual([]);
  // Nothing's both, and neither names what std hasn't.
  expect(found.filter((m) => covered.has(m.id) && whyNot(m) !== undefined).map((m) => m.id)).toEqual([]);
  expect([...covered, ...Object.keys(excluded)].filter((id) => !ids.has(id))).toEqual([]);
});

test("each template has cases, each of its own id", () => {
  const cases = consumerCases();
  expect(templates.filter((t) => !cases.some((c) => c.template === t)).map((t) => t.name)).toEqual([]);
  expect(new Set(templates.map((t) => t.name)).size).toBe(templates.length);
  expect(new Set(cases.map((c) => c.id)).size).toBe(cases.length);
  // Each variant is made: a closure's panic, a source's step's and an
  // argument's.
  expect(new Set(cases.map((c) => c.variant))).toEqual(new Set(["ok", "closure", "step", "arg step"]));
});
