// Mutations of crates/rust-js-lower/src/traits/dictionaries/ref_into_iter.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ref-into-iter-steps-missing",
    breaks:
      "a reference's `IntoIterator` dictionary carries no `Iterator` of what it makes, so a held flatten of `&[T; N]` items can't step them",
    file: "crates/rust-js-lower/src/traits/dictionaries/ref_into_iter.rs",
    find: "        let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];\n        props.extend(self.made_steps(tr, (itself, itself), span)?);\n        Ok(Expr::object(props))\n",
    replace: "        let props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];\n        Ok(Expr::object(props))\n",
    tests: ["test/corpus.test.ts", "-t", "^std_flatten.rs$"],
  },
  {
    name: "ref-into-iter-bound-taken-for-a-collections",
    breaks:
      "in the build of std's iterator structs, std's `zip` given a `&I` whose `IntoIterator` is a bound of its caller's asks a std collection's reference's dictionary of it, refused, where its caller's bound gives it (#1264)",
    file: "crates/rust-js-lower/src/traits/dictionaries/ref_into_iter.rs",
    find: "            && self.evidence_for(tr).is_none()\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^into_iterator_of_a_reference_given_to_generic_code with"],
  },
  {
    name: "ref-iterator-into-iter-by-collection",
    breaks: "a `&mut` of the crate's iterator given where std asks `IntoIterator` is taken for a collection's, and has no evidence, so std's compiled `flatten()` of `&mut`s is refused (#742)",
    file: "crates/rust-js-lower/src/traits/dictionaries/ref_into_iter.rs",
    find: "            && !self.recognition().iterates_as_itself(ty)\n            && !self.has_user_impl(tr.def_id, ty)\n",
    replace: "            && !self.has_user_impl(tr.def_id, ty)\n",
    tests: ["test/iterator-forms.test.ts", "test/corpus.test.ts", "-t", "^mut_ref_iterators_into_iter with std|^mut_ref_iterator_through_a_reference_bound.rs$"],
  },
];
