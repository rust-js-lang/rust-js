// Mutations of src/js/optional_call.rs.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "optional-call-of-a-write",
    breaks: "an optional call is made of an argument that does something, which JS evaluates nowhere its callee is `undefined`: `drop?.($entryInsert(..))` writes nothing where no drop is given (#1369)",
    file: "src/js/optional_call.rs",
    find: "        if !args.iter().any(Expr::has_effects) {\n",
    replace: "        if true {\n",
    unitTests: "js::optional_call::tests::optional_call_evaluates_what_does_something",
  },
];
