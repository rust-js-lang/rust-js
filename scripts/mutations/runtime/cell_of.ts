// Mutations of src/runtime/cell_of.js (ADRs 0153, 1115).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-of-write-rebinds",
    breaks:
      "an object's in-place cell, given where generic code has its box, `*t = v` of `fn put<T>(t: &mut T, v: T)`, rebinds the cell's own name for the object, so the caller's object is left as it was",
    file: "src/runtime/cell_of.js",
    find: "      $assign(object, value);\n",
    replace: "      object = value;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(generic_mut_written_whole|mut_method_pointer_replaced|generic_mut_moved_out|evidence_paths)\\.rs$",
    ],
  },
];
