// How the suite and the checks split their work across the cores and the
// shards (scripts/shard.ts).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "case-pool-never-widens",
    breaks: "a test shard's files of cases stay at half its cores once the rest of the suite has ended, two of four idle (check 43700659375)",
    file: "scripts/shard.ts",
    find: "return restRunning ? caseParts(cores) : pooledParts(cores);",
    replace: "return caseParts(cores);",
    tests: ["test/ci.test.ts", "-t", "small parts of one pool"],
  },
  {
    name: "case-pool-metamorphic-one-file",
    breaks: "the metamorphic sample runs as one file of the rest of the suite, a shard's last file, again (check 43700659375)",
    file: "scripts/shard.ts",
    find: "return ([\"metamorphic\", \"corpus\"] as const).flatMap",
    replace: "return ([\"corpus\"] as const).flatMap",
    tests: ["test/ci.test.ts", "-t", "small parts of one pool"],
  },
];
