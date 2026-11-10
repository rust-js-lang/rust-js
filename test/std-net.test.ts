// What the std net (test/std-net.ts, #1171) covers is what std has: each
// stable method of std's collections, a slice's, `String`'s and the cells'
// that changes what it's called on in place or calls what it's given is a
// template's, or excluded with why. A toolchain that adds one fails here
// until a template covers it, and one that takes one away until its
// template or exclusion goes. The net itself runs in
// test/iterator-matrix.test.ts, `ITERATOR_TABLE=std`.

import { expect, test } from "bun:test";

import { excluded, stdNetCases, stdNetMethods, templates, whyNot } from "./std-net";

const found = stdNetMethods();
const ids = new Set(found.map((m) => m.id));
const covered = new Set(templates.flatMap((t) => t.covers));

test("std's source is read: its collections', slices', strings' and cells' methods are found", () => {
  for (const id of ["Vec::retain", "VecDeque::rotate_left", "String::replace_range", "BTreeMap::entry", "HashMap::extract_if", "[T]::sort_by", "[T]::select_nth_unstable", "RefCell::get_mut"]) {
    expect(ids.has(id), id).toBe(true);
  }
  // What takes `&self` and calls nothing isn't one.
  expect(ids.has("Vec::len")).toBe(false);
  expect(found.length).toBeGreaterThan(200);
});

test("each method is a template's, or excluded with why", () => {
  const missed = found.filter((m) => !covered.has(m.id) && whyNot(m) === undefined).map((m) => `${m.id} (${m.file}): cover it in test/std-net.ts, or say in \`excluded\` why not`);
  expect(missed).toEqual([]);
  expect(found.filter((m) => covered.has(m.id) && excluded[m.id] !== undefined).map((m) => m.id)).toEqual([]);
  expect([...covered, ...Object.keys(excluded)].filter((id) => !ids.has(id))).toEqual([]);
});

test("each template has cases, each of its own id", () => {
  const cases = stdNetCases();
  expect(new Set(templates.map((t) => t.name)).size).toBe(templates.length);
  expect(new Set(cases.map((c) => c.id)).size).toBe(cases.length);
  expect(new Set(cases.map((c) => c.variant))).toEqual(new Set(["ok", "panics"]));
});
