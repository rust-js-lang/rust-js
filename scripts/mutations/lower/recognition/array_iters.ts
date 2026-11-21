// Mutations of src/lower/recognition/array_iters.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "text-escape-not-chars",
    breaks:
      "a held text escape isn't std's iterator of its `char`s' array: `e.next()` of `'\u00e9'.escape_default()` is `$next` of an array, `it.next is not a function` (#379, ADR 0971)",
    file: "src/lower/recognition/array_iters.rs",
    find: "            || is_text_escape(self.tcx, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^escapes_stepped.rs$"],
  },
  {
    name: "list-iter-not-items",
    breaks:
      "a `LinkedList`'s `iter()` is a JS helper of it, which can't be cloned where it is",
    file: "src/lower/recognition/array_iters.rs",
    find: "            Some(StdPath::OptionIter | StdPath::OptionIntoIter | StdPath::ResultIter | StdPath::LinkedListIter\n",
    replace: "            Some(StdPath::OptionIter | StdPath::OptionIntoIter | StdPath::ResultIter\n",
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "option-iter-not-items",
    breaks: "an `Option`'s `iter()` can't be cloned where it is",
    file: "src/lower/recognition/array_iters.rs",
    find: "            Some(StdPath::OptionIter | StdPath::OptionIntoIter | StdPath::ResultIter | StdPath::LinkedListIter\n",
    replace: "            Some(StdPath::ResultIter | StdPath::LinkedListIter\n",
    tests: ["test/corpus.test.ts", "-t", "^std_iterator_clones.rs$"],
  },
  {
    name: "byte-escape-not-stepped",
    breaks:
      "a byte's escape isn't a `$iter` of its bytes, and stepping it from its back or cloning it is refused",
    file: "src/lower/recognition/array_iters.rs",
    find: "            || is_byte_escape(self.tcx, ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^byte_escape.rs$"],
  },
  {
    name: "option-into-iter-unstepped",
    breaks:
      "an `Option`'s `into_iter()` can't be stepped from its back, so `chain(Some(x)).rev()` is refused",
    file: "src/lower/recognition/array_iters.rs",
    find: "StdPath::OptionIter | StdPath::OptionIntoIter | ",
    replace: "StdPath::OptionIter | ",
    tests: ["test/corpus.test.ts", "-t", "^rev_chain.rs$"],
  },
  {
    name: "chunks-not-made-arrays",
    breaks:
      "a slice's chunks and windows aren't known for the arrays they're made, so stepping one from its back is refused",
    file: "src/lower/recognition/array_iters.rs",
    find: "            | StdPath::SliceChunks | StdPath::SliceChunksMut | StdPath::SliceWindows\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chunks_from_their_back.rs$"],
  },
  {
    name: "once-no-array-of-items",
    breaks: "`iter::once(x)`, `[x]`, isn't taken for an array of items, so its `size_hint()` is refused, \"from its MIR\", where std's is `(1, Some(1))` (#1375)",
    file: "src/lower/recognition/array_iters.rs",
    find: "            | StdPath::IterOnce\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^once_and_once_with_hint.rs$"],
  },
  {
    name: "array-iter-every-std-iterator",
    breaks:
      "every type of `std::iter` is taken for an array's iterator, so in a crate that reads a capacity an adapter generic code makes of its `&mut I`, `src.inspect(f)`, is made an array's `$iter` and throws (#2226)",
    file: "src/lower/recognition/array_iters.rs",
    find: "        matches!(krate, Some(StdCrate::Core | StdCrate::Alloc))\n            && (matches!(\n",
    replace:
      "        matches!(krate, Some(StdCrate::Core | StdCrate::Alloc))\n            && (std_path_under(self.tcx, adt.did(), StdPath::IterMod) || matches!(\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_kept_of_a_lent_adapter.rs$"],
  },
];
