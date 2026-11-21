// Mutations of src/lower/classes/states.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-state-missing-key-skipped",
    breaks: "a variant given to such a constructor drops the keys it hasn't, so the later ones shift into their places (rd380)",
    file: "src/lower/classes/states.rs",
    find: ".map(|key| field(key).unwrap_or_else(Expr::js_undefined))",
    replace: ".filter_map(|key| field(key))",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
  {
    name: "state-maker-params-unnamed",
    breaks: "the function a flattened state made by a call is given to names its parameters `arg0`, not as the constructor does, `(id, state)` (#1328)",
    file: "src/lower/classes/states.rs",
    find: "                    let own = own.get(i).cloned().flatten();\n",
    replace: "                    let own = own.get(usize::MAX).cloned().flatten();\n",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made by a call"],
  },
];
