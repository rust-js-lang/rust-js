// Mutations of src/lower/std_coverage.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-coverage-nothing-known",
    breaks: "every std method counts as refused, though rust-js knows it",
    file: "src/lower/std_coverage.rs",
    find: "                let known = recognition.classify(def_id, args).is_some();\n",
    replace: "                let known = recognition.classify(def_id, args).is_some() && false;\n",
    tests: ["test/std-coverage.test.ts"],
  },
];
