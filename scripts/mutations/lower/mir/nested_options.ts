// Mutations of src/lower/mir/nested_options.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nested-option-mentions-unchecked",
    breaks:
      "an `Option` of an `Option` given to a function is held as its payload too, so the function, which boxes, reads its `Some(None)`, `null`, as `None` (ADR 1017)",
    file: "src/lower/mir/nested_options.rs",
    find: "                (nested.contains(&local) || refs.contains_key(&local)) && !allowed.contains(&(at, local))\n",
    replace:
      "                (nested.contains(&local) || refs.contains_key(&local)) && false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-copies-unlinked",
    breaks:
      "an `Option` of an `Option` copied to one given to a function is held as its payload while the copy is boxed (ADR 1017)",
    file: "src/lower/mir/nested_options.rs",
    find: "                if out.contains(&x) || out.contains(&y) {\n",
    replace: "                if out.contains(&x) && out.contains(&y) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-inner-given-on-unboxed",
    breaks:
      "the inner `Option` of one held as its payload, given on, is its `null`: `first(undefined, f)` calls `f(null)` (rd489, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                            && (!of.projection[2..].iter().all(|elem| *elem == PlaceElem::Deref)\n",
    replace: "                            && (true\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-through-reference-unchecked",
    breaks:
      "the inner `Option` an `Option<&Option<T>>` held as its payload refers to, given on, is its `null` (rd489, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                            && (!of.projection[2..].iter().all(|elem| *elem == PlaceElem::Deref)\n",
    replace: "                            && (!of.projection[2..].is_empty()\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-copied-on-unchecked",
    breaks:
      "the inner `Option` of one held as its payload, copied out and returned, is its `null` (rd489, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                                | Rvalue::CopyForDeref(of) => of.local == held && its_payload(of),\n",
    replace: "                                | Rvalue::CopyForDeref(of) => of.local == held,\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-test-boxed",
    breaks:
      "an `Option` of an `Option` whose inner `Option` is only tested, `inner.is_some()`, keeps its box, `$some(value)`, not `last != null` (ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "        locals.writes[held] == 1\n            && (mentions.iter())\n",
    replace: "        false\n            && (mentions.iter())\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-test-reference-unseen",
    breaks:
      "an `Option` of an `Option` whose inner `Option` is given by a reference to `is_some()` keeps its box (ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                tested.insert(reference);\n",
    replace: "",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-option-is-some-boxed",
    breaks:
      "`last.is_some()` of an `Option` of an `Option` held where it's made keeps its box, `$some(item)`",
    file: "src/lower/mir/nested_options.rs",
    find: "                        through(&a.node)\n",
    replace: "                        None\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-some-none-eq-boxed",
    breaks:
      "`last == Some(None)` of an `Option` of an `Option` held where it's made keeps its box, `$eq(last, { $someNone: 0 })`",
    file: "src/lower/mir/nested_options.rs",
    find: "                            (Some(a), None) if self.nested_some_none(body, &b.node) => Some(a),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-keyed-read-out",
    breaks:
      "a keyed read of a value a collection holds `None` of as `null` doesn't write a local held as its payload, so it's `$someGet`'s box, `$eq(.., { $someNone: 0 })` (ADR 0884, amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && bindings::reads_a_nullable_value(self.tcx, def_id, args)\n            {\n                allowed.insert((body.terminator_loc(block), to));",
    replace:
      "                && bindings::reads_a_nullable_value(self.tcx, def_id, args)\n                && false\n            {\n                allowed.insert((body.terminator_loc(block), to));",
    tests: ["test/js-collections.test.ts", "-t", "a Map's value is any value"],
  },
];
