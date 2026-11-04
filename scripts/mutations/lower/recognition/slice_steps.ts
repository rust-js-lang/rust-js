// Mutations of src/lower/recognition/slice_steps.rs (#234, #668, #670, #697).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nths-a-slice-not-through-adapters",
    breaks:
      "a `step_by` of a `Skip`, a `Fuse`, a `Peekable` or an `Enumerate` of a reversed `Vec` is a `filter`, which drops each item it passes, back to front, not as `nth_back`'s one slice: the `OwnStep` catalog's `nth` is not asked (#261, #277, #697)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "        let mut sources = asked.chain(self.nth_advanced_source(ty));\n",
    replace: "        let mut sources = asked.take(0).chain(self.nth_advanced_source(ty));\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "nth-back-of-a-take-unasked",
    breaks: "a `take`'s `rfold` isn't said to ask its source's `nth_back`, so a `take(n).rev()` collected into a `String` steps by `next` (#670)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "                OwnStep::NthBack => found.push(source),\n",
    replace: "                OwnStep::NthBack => {}\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "nth-back-not-through-try-rfold",
    breaks:
      "a `skip`'s `rfold`, its `try_rfold`, isn't followed to a `take`'s, so a `take(3).fuse().skip(1).rev()` collected into a `String` steps by `next`, dropping 4 before 5 (#697)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "                OwnStep::Fold | OwnStep::Rfold | OwnStep::TryRfold => {\n",
    replace: "                OwnStep::TryRfold => {}\n                OwnStep::Fold | OwnStep::Rfold => {\n",
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
