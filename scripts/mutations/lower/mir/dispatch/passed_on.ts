// Mutations of src/lower/mir/dispatch/passed_on.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "passed-on-receiver-not-read-through",
    breaks:
      "a method std's impl for a `&mut W` passes on to `W`'s is given its receiver, the `&mut &mut W`, not the `&mut W` it's of: a `&mut &mut String`'s `write_str` replaces the cell its local holds with text, and a `&mut &mut Vec<u8>`'s `write_all` throws (#1309)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "            None => self.tcx.mk_place_deref(place),\n",
    replace: "            None => place,\n",
    tests: ["test/corpus.test.ts", "-t", "^std_method_of_a_mut_to_a_held_mut.rs$"],
  },
  {
    name: "shared-receiver-passed-through-a-cell",
    breaks: "a `&self` method of a `&mut W` whose `&mut W` is a cell, not `W` itself, is called as `W`'s, given the cell",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "!shared || self.mut_ref_form(w).read_is_itself()",
    replace: "!shared || true",
    tests: ["test/corpus.test.ts"],
  },
  {
    name: "passed-on-one-level-only",
    breaks: "`finish()` of a `&mut &mut` to the crate's own hasher passes on one `&mut` only, to std's impl for the inner `&mut`, which is refused (#1724)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "            levels.push((inner, shared));\n            at = inner;\n",
    replace: "            levels.push((inner, shared));\n            at = inner;\n            break;\n",
    tests: ["test/corpus.test.ts", "-t", "^hasher_finished_through_a_mut_of_its_mut.rs$"],
  },
  {
    name: "shared-receiver-read-through-its-temporary",
    breaks: "a `&self`'s receiver, a temporary `&` of a `&mut &mut`, is read through rather than from what it borrows, one `.value` too many: a TypeError (#1724)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "            Some(lent) => lent,\n",
    replace: "            Some(_) => self.tcx.mk_place_deref(place),\n",
    tests: ["test/corpus.test.ts", "-t", "^hasher_finished_through_a_mut_of_its_mut.rs$"],
  },
  {
    name: "reference-w-not-passed-on",
    breaks: "`finish()` of a `&mut &mut Tally`, whose `&mut Tally` has the crate's own `Hasher`, isn't passed on to that impl, and is refused (#1724)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "                false => w.is_ref().then_some((k, inner, true)),\n",
    replace: "                false => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^hasher_finished_through_a_mut_of_its_mut.rs$"],
  },
  {
    name: "lent-place-read-through-once-only",
    breaks: "a `&self`'s receiver read through twice, of the crate's own `impl Hasher for &mut Tally` given a `&mut &mut Tally`, goes through its temporary `&`, one `.value` too many (#1724)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "        let lent = (k + usize::from(holds) > 0 && levels[0].1)\n",
    replace: "        let lent = (k > 0 && levels[0].1)\n",
    tests: ["test/corpus.test.ts", "-t", "^hasher_finished_through_a_mut_of_its_mut.rs$"],
  },
  {
    name: "lent-place-read-through-always",
    breaks: "a `&self`'s receiver read through once, `{:?}` of a `&mut` to a struct a closure captures, is read from the place its temporary borrows, which may be no JS name: a ReferenceError (#1724)",
    file: "src/lower/mir/dispatch/passed_on.rs",
    find: "        let lent = (k + usize::from(holds) > 0 && levels[0].1)\n",
    replace: "        let lent = (levels[0].1)\n",
    tests: ["test/mut-ref-matrix.test.ts"],
  },
];
