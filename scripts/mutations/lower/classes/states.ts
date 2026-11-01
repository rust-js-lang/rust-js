// Mutations of src/lower/classes/states.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-state-missing-key-skipped",
    breaks: "a variant given to such a constructor drops the keys it hasn't, so the later ones shift into their places (rd380)",
    file: "src/lower/classes/states.rs",
    find: ".map(|key| field(key).unwrap_or_else(Expr::undefined))",
    replace: ".filter_map(|key| field(key))",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
];
