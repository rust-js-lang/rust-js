// Mutations of src/lower/recognition/slice_steps.rs (#234, #668, #670).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nths-a-slice-not-through-enumerate",
    breaks:
      "a `step_by` of an `Enumerate` of a reversed `Vec` is a `filter`, which drops each item it passes, back to front, not as `nth_back`'s one slice (#277)",
    file: "src/lower/recognition/slice_steps.rs",
    find: '                        | "std::iter::Enumerate"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "nths-a-slice-not-through-fuse",
    breaks:
      "a `step_by` of a `Fuse` of a reversed `Vec` is a `filter`, which drops each item it passes, back to front, not as `nth_back`'s one slice (#261)",
    file: "src/lower/recognition/slice_steps.rs",
    find: '                        | "std::iter::Fuse"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "step_by_compiled_source_nth"],
  },
  {
    name: "nths-a-slice-not-through-peekable",
    breaks:
      "a `step_by` of a `Peekable` of a reversed `Vec` is a `filter`, which drops each item it passes, back to front, not as `nth_back`'s one slice (#261)",
    file: "src/lower/recognition/slice_steps.rs",
    find: '                        | "std::iter::Peekable"\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "step_by_compiled_source_nth"],
  },
  {
    name: "chain-nths-no-slice",
    breaks:
      "`rev().chain(..).step_by(2)` of a `Vec` is a filter dropping what it steps over from the back one at a time, not `$stepBy` asking the chain's `nth`, one slice (#261)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "                self.nth_drops_a_slice(args.type_at(0), in_order) || self.nth_drops_a_slice(args.type_at(1), in_order)\n",
    replace: "                false\n",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "nth-back-of-a-take-unasked",
    breaks: "a `take`'s `rfold` isn't said to ask its source's `nth_back`, so a `take(n).rev()` collected into a `String` steps by `next` (#670)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "            Some(\"Take\") => (true, 1),\n",
    replace: "            Some(\"Take\") => (false, 1),\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "slice-in-order-unseen",
    breaks: "a `Vec`'s `IntoIter` isn't said to drop what its `nth` passes as one slice from its front, so a `step_by(n)` of one is a filter by index whose drops stop at a panic (#668)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "        if in_order && self.drops_passed_as_slice(ty) {\n",
    replace: "        if false && self.drops_passed_as_slice(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nth_passed_drops_unwind.rs$"],
  },
];
