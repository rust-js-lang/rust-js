// Mutations of src/lower/analysis/validation.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-base-refused",
    breaks: "a flattened field read whole as an update's base, `..props.html`, is refused, where it's the object its parent is",
    file: "src/lower/analysis/validation.rs",
    find: "                        && !bases.contains(&e) =>",
    replace: "                        && true =>",
    tests: ["test/jsx.test.ts", "-t", "flattened props with a field of them set"],
  },
  {
    name: "init-of-nested-function",
    breaks: "a function a thread-local's `init` makes is taken for the `init`, its value the thread-local's",
    file: "src/lower/analysis/validation.rs",
    find: "        if matches!(tcx.def_kind(p), DefKind::Fn | DefKind::AssocFn | DefKind::Closure) {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "a function a block makes and gives"],
  },
];
