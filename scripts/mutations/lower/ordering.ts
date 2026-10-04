// Mutations of src/lower/ordering.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cmp-without-cells",
    breaks: "`refs.sort()` of a `Vec<&mut i32>` orders the cells, not what they point at",
    file: "src/lower/ordering.rs",
    find: "        let (a, _) = self.through_refs(a, ty);\n        let (b, ty) = self.through_refs(b, ty);\n",
    replace: "        let ty = ty.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "wrapping-ordered-reversed",
    breaks: "`Wrapping`s order the other way round, as `Reverse`'s",
    file: "src/lower/ordering.rs",
    find: "                self.cmp_value(inside(a), inside(b), args.type_at(0), partial, span, out)",
    replace: "                self.cmp_value(inside(b), inside(a), args.type_at(0), partial, span, out)",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
  },
];
