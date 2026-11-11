// Mutations of src/lower/bodies.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "props-with-default-taken-whole",
    breaks:
      "a component taking its props whole has no defaults, its JS callers' missing ones undefined",
    file: "src/lower/bodies.rs",
    find: "            && !pat.is_some_and(|p| matches!(p.kind, PatKind::Leaf { .. }))",
    replace: "            && false",
    tests: ["test/jsx-props.test.ts", "-t", "a props field"],
  },
  {
    name: "props-whole-refused",
    breaks:
      "props passed on whole are refused, as if a field with a default were read",
    file: "src/lower/bodies.rs",
    find: "            && !pat.is_some_and(|p| self.passed_on_whole(p, *adt))\n",
    replace: "",
    tests: ["test/jsx-props.test.ts", "-t", "literal default it says"],
  },
  {
    name: "props-whole-default-read",
    breaks: "a field with a default read of props taken whole is let through",
    file: "src/lower/bodies.rs",
    find: "                && super::bindings::field_default(self.tcx, &adt.non_enum_variant().fields[name]).is_none()",
    replace: "",
    tests: ["test/jsx-props.test.ts", "-t", "literal default it says"],
  },
  {
    name: "default-in-a-dictionary-asks-its-impl",
    breaks: "a trait's default lowered into a blanket impl's dictionary asks the impl, not itself, whether it takes `J` as it is, so its `other.into_iter()` is `$intoIter(other)` of what its callers give as it is, and reads a tuple struct `[0]` as an array: `[2, 3]` where Rust gives `[2, 3, 1, 2, 3]` (itertools' laziness tests)",
    file: "src/lower/bodies.rs",
    find: "            rustc_hir::def::DefKind::Impl { .. } => self.tcx.typeck_root_def_id(self.frame.owner),\n",
    replace: "            rustc_hir::def::DefKind::Impl { .. } => item,\n",
    tests: ["test/corpus.test.ts", "-t", "^trait_default_takes_an_iterator_as_its_callers_give_it.rs$"],
  },
];
