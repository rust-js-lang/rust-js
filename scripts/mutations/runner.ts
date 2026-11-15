// The mutation runner's own rules (scripts/mutations.ts): what a run's
// builds read.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "mutation-run-reads-the-live-checkout",
    breaks: "a run's builds and sites read the checkout as it's edited, not its snapshot",
    file: "scripts/mutations.ts",
    find: "  const touched = syncTree(from, to, crateFiles);\n  crateSource = to;\n",
    replace: "  const touched = syncTree(from, to, crateFiles);\n",
    tests: ["test/mutation-snapshot.test.ts"],
  },
];
