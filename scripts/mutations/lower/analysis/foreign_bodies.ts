// Mutations of src/lower/analysis/foreign_bodies.rs (ADRs 0561, 0914, 0176
// amended): the bodies of another crate's a crate lowers, from its metadata's
// MIR.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "foreign-const-without-mir",
    breaks:
      "a library's impl's constant has no MIR, as its metadata keeps one only for evaluation, so a library that compiles arrayvec's `ArrayVecImpl` impl is refused at its `CAPACITY = CAP` (#468, ADR 0176, amended)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "            (!is_type_const && tcx.defaultness(def_id).has_value()).then(|| tcx.mir_for_ctfe(def_id))\n",
    replace: "            (!is_type_const && tcx.defaultness(def_id).has_value() && false).then(|| tcx.mir_for_ctfe(def_id))\n",
    tests: ["test/crates.test.ts", "-t", "three crates: generic_impl_const_of_instance"],
  },
  {
    name: "closure-in-constant-type-unfound",
    breaks: "a closure a constant's type holds, not the constant itself, `Iterator::count`'s in the `fold` std's `FlattenCompat::count` calls, is never collected, so the body is refused (#736)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "                for part in constant.ty().walk().filter_map(|part| part.as_type()) {\n",
    replace: "                for part in std::iter::once(constant.ty()) {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
];
