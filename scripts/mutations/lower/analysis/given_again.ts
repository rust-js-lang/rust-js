// Mutations of src/lower/analysis/given_again.rs, what React keeps and gives again (#1723).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "component-not-given-again",
    breaks: "a component isn't one React renders again with the props it kept, so what it changes of them it changes in React's, 3 under StrictMode where Rust's is 2 (#1723)",
    file: "src/lower/analysis/given_again.rs",
    find: "                .any(|&tr| crate::lower::recognition::trait_holds_in(tcx, env, output, tr) == Some(true))\n",
    replace: "                .any(|&tr| crate::lower::recognition::trait_holds_in(tcx, env, output, tr) == Some(true) && false)\n",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "binding-callback-not-given-again",
    breaks: "a reducer `use_reducer` names isn't one React calls again with the action it kept, so what it changes of it StrictMode's second call sees, 12 where Rust's is 11 (#1723)",
    file: "src/lower/analysis/given_again.rs",
    find: "                        again.insert(f);\n",
    replace: "",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "number-callback-refused",
    breaks: "a `dyn Fn` reducer of numbers, which nothing changes in place, is refused as one of objects (#1723)",
    file: "src/lower/analysis/given_again.rs",
    find: "        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never | ty::Ref(..) => false,\n",
    replace: "        ty::Never => false,\n",
    tests: ["test/jsx-types.test.ts", "-t", "Reducer"],
  },
  {
    name: "unseen-callback-unrefused",
    breaks: "a reducer rust-js can't see, given an object, is refused with no word of why (#1723)",
    file: "src/lower/analysis/given_again.rs",
    find: "                        tcx.dcx().span_err(\n",
    replace: "                        let _ = (\n",
    tests: ["test/react-kept.test.ts", "-t", "is refused"],
  },
];
