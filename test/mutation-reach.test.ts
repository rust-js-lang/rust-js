// scripts/mutation-reach.ts' answers of what it found (#1979's sweep): the
// target it suggests, what its line says of a mutation, and what of the JS
// it compares. Its sweep itself builds the compiler and runs the corpus:
// run by hand, `bun scripts/mutation-reach.ts <name>`.
import { expect, test } from "bun:test";
import { corpusPattern, jsCompared, retarget, runtimeOnly, said, type Reached } from "../scripts/mutation-reach";
import type { Mutation } from "../scripts/mutations";

const mutation = (over: Partial<Mutation>): Mutation =>
  ({ name: "m", breaks: "b", file: "crates/rust-js-lower/src/x.rs", find: "a", replace: "b", tests: ["test/corpus.test.ts", "-t", "^own\\.rs$"], ...over }) as Mutation;
const sizes: Record<string, number> = { "big.rs": 900, "small.rs": 100, "own.rs": 300, "form.rs": 50 };
const size = (name: string) => sizes[name] ?? 500;

// What runs otherwise first, the smallest case's, of the main build; then
// what compiles otherwise; then a JS that alone differs, held by its
// snapshot.
test("a retarget names the case that runs otherwise, the smallest first", () => {
  const cases: Reached["cases"] = [
    { name: "form.rs", build: "main", reach: "form" },
    { name: "big.rs", build: "main", reach: "behaves" },
    { name: "small.rs", build: "main", reach: "behaves" },
    { name: "own.rs", build: "std-iterators", reach: "behaves" },
  ];
  expect(retarget(cases, size)).toEqual({ tests: ["test/corpus.test.ts", "-t", "^small\\.rs$"], snapshots: false });
  expect(retarget([{ name: "form.rs", build: "main", reach: "form" }], size)).toEqual({ tests: ["test/corpus.test.ts", "-t", "^form\\.rs$"], snapshots: true });
  expect(retarget([], size)).toBeUndefined();
});

// Its line: held where its own case reaches it as its tests judge it, a
// retarget where none does, a retirement where it reaches none.
test("a mutation's line says whether it's held, to retarget or to retire", () => {
  expect(said(mutation({}), [{ name: "own.rs", build: "main", reach: "behaves" }], size)).toEndWith("held as it is");
  expect(said(mutation({}), [{ name: "own.rs", build: "main", reach: "form" }], size)).toContain('retarget: tests: ["test/corpus.test.ts","-t","^own\\\\.rs$"], snapshots: true');
  expect(said(mutation({ snapshots: true }), [{ name: "own.rs", build: "main", reach: "form" }], size)).toEndWith("held as it is");
  expect(said(mutation({}), [{ name: "small.rs", build: "main", reach: "behaves" }], size)).toContain("its tests' cases don't reach it; retarget");
  expect(said(mutation({ tests: ["test/serde.test.ts"] }), [], size)).toContain("its tests run no corpus case; reaches no case");
});

test("what's compared of a mutation and its tests", () => {
  expect(corpusPattern(["test/serde.test.ts", "-t", "x"])).toBeUndefined();
  expect(corpusPattern(["test/corpus.test.ts"])).toBeNull();
  expect(corpusPattern(["test/corpus.test.ts", "-t", "^a\\.rs$"])?.test("a.rs")).toBe(true);
  expect(jsCompared("x;\n//# sourceMappingURL=a.js.map\n")).toBe(jsCompared("x;\n//# sourceMappingURL=b.js.map\n"));
  expect(runtimeOnly(mutation({ file: "crates/rust-js-program/src/runtime/into_iter.js" }))).toBe(true);
  expect(runtimeOnly(mutation({}))).toBe(false);
});
