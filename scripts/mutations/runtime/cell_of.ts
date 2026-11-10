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
  {
    name: "held-cell-of-the-cell",
    breaks: "a cell of a `&mut` to an object, given as a generic `&mut W`, holds the object, not its in-place cell (#1300)",
    file: "src/runtime/cell_of.js",
    find: "      return $cellOf(cell.value);\n",
    replace: "      return cell.value;\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_to_generic_code.rs$"],
  },
];
