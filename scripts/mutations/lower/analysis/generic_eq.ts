// Mutations of src/lower/analysis/generic_eq.rs (ADR 1014).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-trusted-eq-allowed",
    breaks: "generic code given two `TrustedLen` maps it compares by `eq` compares them item by item, running their closures where Rust compares their lengths first and runs nothing (#386)",
    file: "src/lower/analysis/generic_eq.rs",
    find: "                    if trusted && (runs_code(a) || runs_code(b)) && refused.insert(expr.span) {\n",
    replace: "                    if false && trusted && (runs_code(a) || runs_code(b)) && refused.insert(expr.span) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "eq in generic code of two maps"],
  },
];
