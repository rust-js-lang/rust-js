// Mutations of src/lower/recognition/slice_steps.rs (#234, #668, #670, #697).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nths-a-slice-not-through-adapters",
    breaks:
      "a `step_by` of a `Skip`, a `Fuse`, a `Peekable` or an `Enumerate` of a reversed `Vec` is a `filter`, which drops each item it passes, back to front, not as `nth_back`'s one slice: the `OwnStep` catalog's `nth` is not asked (#261, #277, #697)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "            .any(|(source, asked)| self.passes_a_slice(source, asked, in_order))\n",
    replace: "            .any(|(source, asked)| false && self.passes_a_slice(source, asked, in_order))\n",
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
    find: "                OwnStep::Fold | OwnStep::TryFold | OwnStep::Rfold | OwnStep::TryRfold => {\n",
    replace: "                OwnStep::TryRfold => {}\n                OwnStep::Fold | OwnStep::TryFold | OwnStep::Rfold => {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^a take's rev, consumed by fold or by next"],
  },
  {
    name: "slice-in-order-unseen",
    breaks: "a `Vec`'s `IntoIter` isn't said to drop what its `nth` passes as one slice from its front, so a `step_by(n)` of one is a filter by index whose drops stop at a panic (#668)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "        if (step == OwnStep::NthBack || in_order) && self.drops_passed_as_slice(ty) {\n",
    replace: "        if step == OwnStep::NthBack && self.drops_passed_as_slice(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nth_passed_drops_unwind.rs$"],
  },
  {
    name: "nth-back-passes-no-slice-through-adapters",
    breaks:
      "a `Rev`'s `nth` of a `take(n)`, a `skip(k)`, an `enumerate()` or a `fuse()` of a `Vec`'s items isn't followed to the `Vec`'s `nth_back`, so a `step_by` of it is a filter by index, which drops what it passes back to front where std's one slice drops it front to back (#2239)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "            .filter(|&(_, asked)| matches!(asked, OwnStep::Nth | OwnStep::NthBack))\n",
    replace: "            .filter(|&(_, asked)| step == OwnStep::Nth && asked == OwnStep::Nth)\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_stepped_by_from_its_back_through_adapters_drops_a_slice.rs$"],
  },
  {
    name: "nth-back-of-a-vec-no-slice",
    breaks: "a `Vec`'s `nth_back` isn't said to drop what it passes as one slice, so a `rev().step_by(n)` of its items is a filter by index, which drops them back to front (#234)",
    file: "src/lower/recognition/slice_steps.rs",
    find: "        if (step == OwnStep::NthBack || in_order) && self.drops_passed_as_slice(ty) {\n",
    replace: "        if in_order && self.drops_passed_as_slice(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_stepped_by_from_its_back_through_adapters_drops_a_slice.rs$"],
  },
  {
    name: "steps-past-a-slice-next-unfollowed",
    breaks: "stepping a `take_while()` or a `map()` of a `Skip` of a reversed `Vec` isn't said to first ask that `Skip`'s `nth`, so a `step_by` of it is a filter by index, folded, whose `nth(n - 1)` drops the first it skips after the rest (#294, #2239)",
    file: "src/lower/recognition/slice_steps.rs",
    find: '            "next" => self.steps_past_a_slice(source),\n',
    replace: '            "next" => false,\n',
    tests: ["test/corpus.test.ts", "-t", "^vec_stepped_by_from_its_back_through_adapters_drops_a_slice.rs$"],
  },
];
