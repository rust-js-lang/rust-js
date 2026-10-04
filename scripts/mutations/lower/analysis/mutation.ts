// Mutations of src/lower/analysis/mutation.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-step-untracked",
    breaks: "a range `next()` changes is taken to never change, so a clone of it is the same object",
    file: "src/lower/analysis/mutation.rs",
    find: "                } if enum_ty(body.thir[arg].ty) || range_ty(body.thir[arg].ty) => {",
    replace: "                } if enum_ty(body.thir[arg].ty) => {",
    tests: ["test/corpus.test.ts","-t","range_values"],
  },
  {
    name: "for-loop-steps-counted",
    breaks: "a `for` over a range counts as changing it, so its end is a `const` first: right, but not the JS a person writes",
    file: "src/lower/analysis/mutation.rs",
    find: "                !expr.span.is_desugaring(DesugaringKind::ForLoop)\n",
    replace: "                true\n",
    tests: ["test/corpus.test.ts","-t","range_values"],
    snapshots: true,
  },
  {
    name: "replaced-whole-not-mutation",
    breaks: "`*p = P::default()` through a `&mut` isn't a change in place, so `let b = a` shares `a`'s object, and resetting `b` resets `a`",
    file: "src/lower/analysis/mutation.rs",
    find: "            {\n                mutated.insert(body.thir[lhs].ty);\n            }",
    replace: "            {}",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
  {
    name: "mem-swap-not-mutation",
    breaks: "`mem::swap` of two `&mut` objects isn't a change in place, so a `Copy` value shared with the swapped one changes too",
    file: "src/lower/analysis/mutation.rs",
    find: "                && replaces_whole(tcx, def_id)",
    replace: "                && replaces_whole(tcx, def_id) && false",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
  {
    name: "replaced-string-mutated",
    breaks: "a `String` replaced through a `&mut` counts as changed in place, so its `clone()` is refused",
    file: "src/lower/analysis/mutation.rs",
    find: "                && object_like(tcx, body.thir[lhs].ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
];
