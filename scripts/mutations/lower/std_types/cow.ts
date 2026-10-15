// Mutations of src/lower/std_types/cow.rs (ADR 0319).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cow-to-mut-shares",
    breaks: "`to_mut()` changes what the `Cow` borrowed, not a clone of it",
    file: "src/lower/std_types/cow.rs",
    find: "            StmtKind::Assign(value.clone(), clone).at(js_span),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cows"],
  },
  {
    name: "cow-into-owned-shares",
    breaks: "`into_owned()` of a borrowed slice is the slice, which a push then changes",
    file: "src/lower/std_types/cow.rs",
    find: "            return Ok(Expr::cond(borrowed, clone, value));\n",
    replace: "            return Ok(value);\n",
    tests: ["test/corpus.test.ts", "-t", "^cows"],
  },
  {
    name: "cow-side-through-refs-unread",
    breaks: "`&cow == &\"a\"`, a `Cow` behind a reference, compares the reference's `_0`",
    file: "src/lower/std_types/cow.rs",
    find: "        let (inner, inner_ty) = self.through_refs(value.clone(), ty);\n",
    replace: "        let (inner, inner_ty) = (value.clone(), ty);\n",
    tests: ["test/corpus.test.ts", "-t", "^cow_compare.rs$"],
  },
];
