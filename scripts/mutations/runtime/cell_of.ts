// Mutations of crates/rust-js-program/src/runtime/cell_of.js (ADRs 0153, 1115).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "through-place-made-bare",
    breaks:
      "a placed dictionary's entry that gives a `Self` back, `clone()`, gives it bare, out of its place (#2255)",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "  if (shape === 1) return { value };\n",
    replace: "  if (shape === 1) return value;\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "from-place-left-placed",
    breaks:
      "what generic code gives back of a tuple of iterators in their places, `firsts`'s `(A, B)`, is given its caller still in its places, so `{:?}` of a part prints `[object Object]` (#2255)",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "  if (shape === 1) return value.value;\n",
    replace: "  if (shape === 1) return value;\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "place-parts-unwalked",
    breaks:
      "a tuple's parts given generic code by `$inPlace(t, [1, 1])` are given as their caller holds them, so a tuple's own impl's `step` stores into the bare string (#2255)",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "  return Array.isArray(shape) ? shape.map((part, i) => $inPlace(value[i], part)) : value;\n",
    replace: "  return value;\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "into-place-entries-take-bare",
    breaks:
      "an `$intoPlace` dictionary's entry is given what it takes as its caller holds it, so a tuple's own impl's `step` stores into the bare string (#2255)",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "  return $placedEntries(dictionary, given, back, $inPlace, $fromPlace);\n",
    replace: "  return dictionary;\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "cell-of-write-rebinds",
    breaks:
      "an object's in-place cell, given where generic code has its box, `*t = v` of `fn put<T>(t: &mut T, v: T)`, rebinds the cell's own name for the object, so the caller's object is left as it was",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
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
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "      return $cellOf(cell.value);\n",
    replace: "      return cell.value;\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_of_a_held_mut_given_to_generic_code.rs$"],
  },
  {
    name: "through-box-not-through",
    breaks: "`$throughBox` gives each entry the box as it is: `show(&mut parser)` shows `undefined` (#1414)",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "given[i] === 1 ? arg.value :",
    replace: "given[i] === 1 ? arg :",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_given_for_a_bound.rs$"],
  },
  {
    name: "in-place-part-function-unapplied",
    breaks: "`$inPlace` keeps a part whose shape is the function that adapts it as it is, so a tuple remade for generic code gives the crate's iterator in it bare (#2268)",
    file: "crates/rust-js-program/src/runtime/cell_of.js",
    find: "  if (typeof shape === \"function\") return shape(value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^crates_iterator_in_a_tuple_given_to_generic_code.rs$"],
  },
];
