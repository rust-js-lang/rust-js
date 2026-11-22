// Mutations of src/phase_times.rs (ADR 0093): where a compile's time goes (#2296).
import type { Mutation } from "../mutations";

const tested = ["test/phase-times.test.ts", "-t", "a timed compile appends its phases"];

export const mutations: Mutation[] = [
  {
    name: "phase-times-unmarked",
    breaks: "a timed compile appends a line of no phases, so the report can't say where its time went (#2296)",
    file: "src/phase_times.rs",
    find: "        times.push((phase, now.duration_since(*at).as_secs_f64() * 1000.0));\n",
    replace: "        _ = phase;\n",
    tests: tested,
  },
];
