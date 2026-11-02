// Mutations of src/lower/analysis/foreign_bodies.rs (ADRs 0561, 0914, 0176
// amended): the bodies of another crate's a crate lowers, from its metadata's
// MIR.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inlined-closures-unfound",
    breaks:
      "a closure only a local of a std body holds, `sum`'s in `Filter::count` of a `core` built to unwind, is never collected, so the body is refused (#424)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "                if let rustc_middle::ty::Closure(closure, _) = *part.kind() {\n                    closures.0.push(closure);\n",
    replace:
      "                if let rustc_middle::ty::Closure(closure, _) = *part.kind() {\n                    let _ = closure;\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "foreign-const-without-mir",
    breaks:
      "a library's impl's constant has no MIR, as its metadata keeps one only for evaluation, so a library that compiles arrayvec's `ArrayVecImpl` impl is refused at its `CAPACITY = CAP` (#468, ADR 0176, amended)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "            (!is_type_const && tcx.defaultness(def_id).has_value()).then(|| tcx.mir_for_ctfe(def_id))\n",
    replace: "            (!is_type_const && tcx.defaultness(def_id).has_value() && false).then(|| tcx.mir_for_ctfe(def_id))\n",
    tests: ["test/crates.test.ts", "-t", "three crates: generic_impl_const_of_instance"],
  },
];
