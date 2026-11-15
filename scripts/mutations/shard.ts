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
  {
    name: "balanced-dealt-to-the-busiest",
    breaks: "a mutation shard's next mutation goes to the shard with the most time so far, not the least: one shard gets the long ones",
    file: "scripts/shard.ts",
    find: "if (load[i]! < load[least]!) least = i;",
    replace: "if (load[i]! > load[least]!) least = i;",
    tests: ["test/ci.test.ts", "-t", "mutations shard by their times"],
  },
  {
    name: "balanced-unknown-takes-nothing",
    breaks: "a mutation no run timed counts as no time, so the shards dealt new ones pile them up",
    file: "scripts/shard.ts",
    find: "const time = (item: T) => seconds.get(item) ?? usual;",
    replace: "const time = (item: T) => seconds.get(item) ?? 0;",
    tests: ["test/ci.test.ts", "-t", "mutations shard by their times"],
  },
  {
    name: "timed-shard-ignores-timings",
    breaks: "a mutation shard given the nightly's times deals in turn all the same",
    file: "scripts/shard.ts",
    find: "  if (dir === undefined) return shard(items, index, count);\n",
    replace: "  if (dir === undefined || dir !== undefined) return shard(items, index, count);\n",
    tests: ["test/ci.test.ts", "-t", "reads its times of the runs"],
  },
];
