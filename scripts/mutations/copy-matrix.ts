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
    // Its rows, #1983's that 41d89f84f fixed and their plain twins: an
    // issue's rows go when it's fixed, which left `#1983` naming none.
    env: { ITERATOR_TABLE: "copies", ITERATOR_MATRIX: "/ \\| copied \\| (clone|dup) \\| len$/" },
    tests: ["test/iterator-matrix.test.ts"],
  },
];
