// The copy matrix (test/copy-matrix.ts, ADR 1090, amended): a matrix that
// doesn't copy what it says it copies finds nothing a copy loses.
import type { Mutation } from "./../mutations";
export const mutations: Mutation[] = [
  {
    name: "copy-matrix-clones-nothing",
    breaks: "the copy matrix's `clone` and `dup` rows move the iterator through generic code instead of copying it, so what a copy loses (#1983's `$given` wrapper made without its back) is never stepped",
    file: "test/copy-matrix.ts",
    find: '  const copied = way === "clone" ? "a.clone()" : way === "dup" ? "dup(&a)" : "pass(a)";',
    replace: '  const copied = "pass(a)";',
    env: { ITERATOR_TABLE: "copies", ITERATOR_MATRIX: "#1983" },
    tests: ["test/iterator-matrix.test.ts"],
  },
];
