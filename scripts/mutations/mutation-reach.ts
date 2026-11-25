// A survivor's reach over the corpus, swept by a pool that lets the
// machine's turn go to a waiting push (scripts/mutation-reach.ts, #2585).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "reach-sweep-unyielding",
    breaks: "mutation-reach.ts' sweep is a pool of its own, not leases' `runYielding`, so it holds the machine's turn to its end however long a push waits: 36 survivors' sweep held it some 50 min (#2585)",
    file: "scripts/mutation-reach.ts",
    find: "  await runYielding([...cases], Math.max(1, cores), one);",
    replace: "  await Promise.all([...cases].map(one)); void runYielding;",
    tests: ["test/leases.test.ts", "-t", "let the turn go through leases' own"],
  },
];
