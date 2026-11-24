// Mutations of crates/rust-js-lower/src/generic_facts/refusals.rs (ADR 0093): what generic code is refused of a call's values.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "placed-item-unrefused",
    breaks:
      "a type parameter generic code holds in a place of its own, also given as another iterator's items, `S: Iterator<Item = I>`, compiles: the items come bare and its `next` reads `.value` of a string, `Some(-1)` (#2534)",
    file: "crates/rust-js-lower/src/generic_facts/refusals.rs",
    find: "        self.refuse_placed_item((id, args), span)\n",
    replace: "        Ok(())\n",
    tests: ["test/corpus.test.ts", "-t", "^placed_iterator_also_another_iterators_item\\.rs$"],
  },
  {
    name: "placed-item-refused-unplaced",
    breaks:
      "a call whose type parameter is another iterator's items, `fn gather<T, I: Iterator<Item = T>>`, is refused whatever it's given, held in a place or not (#2534)",
    file: "crates/rust-js-lower/src/generic_facts/refusals.rs",
    find: "                if self.holds_in_place((id, param, instantiated)) {\n",
    replace: "                if true || self.holds_in_place((id, param, instantiated)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_code_right_for_every_instance\\.rs$"],
  },
  {
    name: "placed-within-unrefused",
    breaks:
      "a fieldless enum's iterator inside a `Vec<I>` generic code makes, held in a place there, is given its caller in that place where it reads the bare value (#2255)",
    file: "crates/rust-js-lower/src/generic_facts/refusals.rs",
    find: "            if let Some(placed) = self.placed_within(id, declared, self.instantiated(declared, args)) {\n",
    replace: "            if let Some(placed) = self.placed_within(id, declared, self.instantiated(declared, args)).filter(|_| false) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "naming #2255"],
  },
  {
    name: "guard-given-to-a-library-whole-write-unrefused",
    breaks: "a `RefCell` guard's `&mut` given to a library's generic `&mut T` isn't refused where it's a guard: the guard is taken for any value (#2475)",
    file: "crates/rust-js-lower/src/generic_facts/refusals.rs",
    find: "            && crate::recognition::is_cell_guard(self.tcx, guard)\n",
    replace: "            && false\n",
    tests: ["test/libraries.test.ts", "-t", "RefCell guard's `&mut` given to a library"],
  },
  {
    name: "placed-items-through-their-bound-unrefused",
    breaks:
      "a fieldless enum's iterator given as items generic code steps through `S::Item: Iterator` compiles, its bare string stepped by a `next` that writes a place's `value`: `Cannot create property 'value'` (#2541)",
    file: "crates/rust-js-lower/src/generic_facts/refusals.rs",
    find: "            if self.placed_iterator(instantiated) {\n",
    replace: "            if false && self.placed_iterator(instantiated) {\n",
    tests: ["test/corpus.test.ts", "-t", "^placed_items_stepped_through_their_bound\\.rs$"],
  },
];
