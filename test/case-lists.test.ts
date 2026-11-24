// A list of what to run that names what its runner hasn't fails, naming
// each entry (test/case-lists.ts): the metamorphic net given its cases as
// `test/corpus/...` paths ran none of them and said "25 pass" (2026-11-25).

import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

import { refuseUnmatched, unmatchedOf } from "./case-lists";
import { caseShard, fixture } from "./support";

const known = new Set(["closure_mut_ref.rs", "held_enumerate.rs", "zip_size_hints.rs"]);
const what = { list: "the list", what: "corpus cases" };

test("a list naming a case by its path, or misspelt, fails, naming each", () => {
  // Every entry a path: it would run none.
  expect(() => refuseUnmatched(["test/corpus/closure_mut_ref.rs", "test/corpus/held_enumerate.rs"], known, what)).toThrow(
    "the list names 2 of its 2 that aren't corpus cases, so it would run none of them: test/corpus/closure_mut_ref.rs, test/corpus/held_enumerate.rs; corpus cases are named by their names, `closure_mut_ref.rs`, not by a path, `test/corpus/closure_mut_ref.rs`",
  );
  // One misspelt among the rest: it would run fewer than it names.
  expect(() => refuseUnmatched(["held_enumerate.rs", "zip_size_hint.rs"], known, what)).toThrow("the list names 1 of its 2 that aren't corpus cases: zip_size_hint.rs");
  expect(unmatchedOf(["held_enumerate.rs", "zip_size_hint.rs", "zip_size_hint.rs"], known)).toEqual(["zip_size_hint.rs"]);
  // A selector's, a folder of rustc's tests, by whether it matches any.
  expect(() => refuseUnmatched(["drop/", "dorp/"], (entry) => entry === "drop/", { list: "the tests named", what: "folders of rustc's tests" })).toThrow("the tests named names 1 of its 2 that aren't folders of rustc's tests: dorp/");
  // Each matched, or none named: nothing to say.
  expect(() => refuseUnmatched(["held_enumerate.rs", "zip_size_hints.rs"], known, what)).not.toThrow();
  expect(() => refuseUnmatched([], known, what)).not.toThrow();
});

test("the corpus's case list, `RUST_JS_CASES=i/n:<file>`, of a path or a misspelt name, fails", () => {
  const dir = fixture("case-lists");
  const list = join(dir, "cases.txt");
  const given = process.env.RUST_JS_CASES;
  try {
    process.env.RUST_JS_CASES = `1/1:${list}`;
    writeFileSync(list, "test/corpus/held_enumerate.rs\n");
    expect(() => caseShard([...known])).toThrow(`RUST_JS_CASES=1/1:${list} names 1 of its 1 that aren't corpus cases, so it would run none of them: test/corpus/held_enumerate.rs; corpus cases are named by their names, \`held_enumerate.rs\``);
    writeFileSync(list, "held_enumerate.rs\nheld_enumerat.rs\n");
    expect(() => caseShard([...known])).toThrow("names 1 of its 2 that aren't corpus cases: held_enumerat.rs");
    // A runner of some of the corpus's cases, the metamorphic net's, given
    // one it doesn't run: a case all the same, so said by the runner, not refused.
    writeFileSync(list, "held_enumerate.rs\nzip_size_hints.rs\n");
    expect(caseShard(["held_enumerate.rs"], [...known])).toEqual(["held_enumerate.rs"]);
    // An empty list names nothing: the runner runs none of its cases.
    writeFileSync(list, "");
    expect(caseShard([...known])).toEqual([]);
  } finally {
    if (given === undefined) delete process.env.RUST_JS_CASES;
    else process.env.RUST_JS_CASES = given;
  }
});
