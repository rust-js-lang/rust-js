// The corpus's verified runs (test/verified-runs.ts, ADR 0567): the nightly's
// share run again.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "verified-run-recheck-skipped",
    breaks: "the nightly's share of the corpus's verified runs isn't run again, so a verified run a key missed an input of is never seen",
    file: "test/verified-runs.ts",
    find: "  return !rechecked();\n}",
    replace: "  return true;\n}",
    tests: ["test/native-outcomes.test.ts", "-t", "verified run is run again"],
  },
];
