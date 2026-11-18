// Mutations of src/lower/declarations/bounds.rs (ADR 0093): a type
// parameter bound by a binding trait that says what a type it bounds is,
// erased to it or its constraint (#1866, ADR 0196 amended).
import type { Mutation } from "../../../mutations";

const bounds = ["test/declarations-bounds.test.ts", "-t", "an IsA bound is TypeScript's constraint"];

export const mutations: Mutation[] = [
  {
    name: "bound-identity-in-result-unseen",
    breaks: "a parameter in a function's result is erased to its bound's type, `selected(value: Element): Element`, so a button given is an `Element` back, where Rust gives the button",
    file: "src/lower/declarations/bounds.rs",
    find: "            count(sig.output()) > 0 || sig.inputs().iter().map(|&ty| count(ty)).sum::<usize>() + in_bounds > 1\n",
    replace: "            sig.inputs().iter().map(|&ty| count(ty)).sum::<usize>() + in_bounds > 1\n",
    tests: bounds,
  },
  {
    name: "bound-identity-written-twice-unseen",
    breaks: "a parameter written twice is erased, `same(a: Element, b: Element)`, so two of different types are accepted where Rust's are one type",
    file: "src/lower/declarations/bounds.rs",
    find: "            count(sig.output()) > 0 || sig.inputs().iter().map(|&ty| count(ty)).sum::<usize>() + in_bounds > 1\n",
    replace: "            count(sig.output()) > 0 || sig.inputs().iter().map(|&ty| count(ty)).sum::<usize>() + in_bounds > 2\n",
    tests: bounds,
  },
  {
    name: "bound-identity-in-other-bounds-unseen",
    breaks: "a parameter in another's bound, `F: Fn(&T)`, is erased, so `each`'s callback is given an `Element` where Rust gives it the caller's type",
    file: "src/lower/declarations/bounds.rs",
    find: "                let given: usize = projection.projection_term.args.types().skip(1).map(count).sum();\n",
    replace: "                let given: usize = 0;\n",
    tests: bounds,
  },
  {
    name: "type-map-record-key-erased",
    breaks: "a record's reason field is widened to string and the record loses its generic parameter",
    file: "src/lower/declarations/bounds.rs",
    find: " || fields > 0 && is_type_map(tcx, bound)",
    replace: "",
    tests: ["test/declarations-type-maps.test.ts", "-t", "written event aliases"],
  },
];
