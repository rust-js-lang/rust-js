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
];
