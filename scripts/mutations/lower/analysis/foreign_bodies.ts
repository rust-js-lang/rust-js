// Mutations of src/lower/analysis/foreign_bodies.rs (ADRs 0561, 0914, 0176
// amended): the bodies of another crate's a crate lowers, from its metadata's
// MIR.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closure-of-an-inlined-function-made-by-none",
    breaks: "`joined`'s closure, made in `fold_with` rustc inlined `joined` into, has no maker, so no module of the consumer's: a panic, `has no module for metamorph_lib::joined::{closure#0}` (#1029)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "                makers.insert(closure, def_id);\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "inlined function is placed"],
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
  {
    name: "closure-in-constant-type-unfound",
    breaks: "a closure a constant's type holds, not the constant itself, `Iterator::count`'s in the `fold` std's `FlattenCompat::count` calls, is never collected, so the body is refused (#736)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "                for part in constant.ty().walk().filter_map(|part| part.as_type()) {\n",
    replace: "                for part in std::iter::once(constant.ty()) {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "closure-in-aggregate-operand-unfound",
    breaks:
      "a closure capturing nothing made into a struct's field, a `const ZeroSized` operand of the struct's aggregate, is never visited, so a consumer compiling the library's function that makes it is refused: arrayvec's `ScopeExitGuard` in `extend_from_iter` (#795)",
    file: "src/lower/analysis/foreign_bodies.rs",
    find: "                self.super_rvalue(rvalue, at);\n",
    replace: "                let _ = at;\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "a library's closure capturing nothing, made into a struct's field"],
  },
];
