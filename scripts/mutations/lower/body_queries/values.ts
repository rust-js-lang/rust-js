// Mutations of src/lower/body_queries/values.rs (ADR 0093): what a place is
// the value of, a box's content its local's (#980, #997, #1000).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "through-box-not-the-value",
    breaks: "a step through a box isn't the value it holds, so `*n += 1` doesn't write `n` again and a read of it before is moved past the write, `pair(n, n)` (#980)",
    file: "src/lower/body_queries/values.rs",
    find: "        PlaceElem::Deref => ty.boxed_ty(),\n",
    replace: "        PlaceElem::Deref => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^box_read_before_its_write.rs$"],
  },
];
