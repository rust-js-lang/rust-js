// Mutations of src/prepare/truths.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "truth-tested-kept",
    breaks: "a module variable's text kept by its truth and matched stays `const h = x || undefined; h != null ? a : b`, where upstream writes `x ? a : b` (ADR 0635, amended)",
    file: "src/prepare/truths.rs",
    find: "    while i + 1 < body.len() {\n",
    replace: "    while false && i + 1 < body.len() {\n",
    tests: ["test/lowering-option-truth.test.ts", "-t", "filtered by its truth is tested by its truth"],
  },
  {
    name: "truth-tested-past-code",
    breaks: "an arm that runs code, which may change the variable, reads it again in the binding's place (ADR 0635, amended)",
    file: "src/prepare/truths.rs",
    find: "                    && !yes.has_effects()\n",
    replace: "",
    tests: ["test/lowering-option-truth.test.ts", "-t", "filtered by its truth is tested by its truth"],
  },
];
