// A library's iterator at the JS boundary, across two crates (ADR 1169):
// the pair `test/crates/pairs/library_impl_iterator_given_back`, which
// crates.test.ts runs against native as each pair, read here for what its
// JS gives a JS caller and what its consumer's Rust calls.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { chain, pairs } from "./crates";
import { root } from "./support";

// A library's `impl Iterator` gives JS the JS iterator, its `back()` and
// `len()` where its bounds say, and its consumer's Rust a copy of its own,
// which gives what it reveals, as the contract's `ReturnsJs` says
// (ADR 1169, #1999).
test("a library's impl Iterator is JS's iterator to JS and its own value to its consumer's Rust", async () => {
  const out = chain(pairs, "library_impl_iterator_given_back", ["dep", "app"]);
  const app = readFileSync(join(out, "app", "lib.js"), "utf8");
  expect(app).not.toMatch(/Dep\.(upto|counted)\(|Bag\.odds\(bag\)[^;]*from "/);
  const manifest = JSON.parse(readFileSync(join(out, "dep", "lib.manifest.json"), "utf8"));
  expect(JSON.stringify(manifest)).toContain("returns_js");
  const dep = await import(join(out, "dep", "lib.js"));
  expect([...dep.upto(4)]).toEqual([0, 1, 2, 3]);
  const made = dep.upto(5);
  made.next();
  expect([made.back().value, made.len(), [...made]]).toEqual([4, 3, [1, 2, 3]]);
  // A closure lowered inside it gives back what it gives, its filter's `bool`.
  expect(readFileSync(join(out, "dep", "lib.js"), "utf8")).toContain("iterator_filter($iter(bag[0]), (x) => x % 2 === 1)");
  expect([[...dep.Bag.odds([[1, 2, 3, 5, 6]])], dep.Bag.odds([[1, 2, 3, 5, 6]]).back().value]).toEqual([[1, 3, 5], 5]);
  expect([...dep.counted(["a", "b"])].join("")).toBe("ab");
}, 300_000);

// #2255 across crates: a library's struct whose fields only its own
// generic code reads holds an iterator JS holds as no object as that code
// holds it, in a place of its own (pairs/
// library_holder_of_a_placed_iterator_built_by_its_generic_code); one with
// a public field its consumer may read by its shape, which holds it as it
// is, is refused, never read as the place.
test("a consumer reading a library's public field of an iterator its generic code holds in a place is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_public_field_holding_a_placed_iterator", ["dep", "app"], [
    "rust-js does not support a `Light` inside a `dep::Open<I>` of generic code's, which holds it in a place of its own, `{ value }`, where its caller holds it as it is (#2255) yet",
    "app.rs:23:",
  ]);
  expect(native).toBe("Green\n");
}, 300_000);

// A library's trait default that holds its own `J` in a place of its own and
// is given values of it as its `Self`'s items, which come bare, is refused at
// the call, as one crate refuses it (#2534, #2508's form 5): never run with
// the items read as places, `[Some(0), Some(-1), Some(-1)]`.
test("a consumer giving a library's default a placed iterator as its items too is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_default_holds_its_selfs_items_in_place", ["dep", "app"], [
    "which it's also given as `Self`'s items, as their caller holds them (#2534, #2535)",
    "app.rs:27:",
  ]);
  expect(native).toBe("held_with [Some(0), Some(1), Some(0)] [Some(1), Some(0), Some(1)]\n");
}, 300_000);
