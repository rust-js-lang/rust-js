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
  {
    name: "untimed-mutations-as-the-median",
    breaks: "a change's untimed mutations are estimated as the timed ones' median, not their file's or the 90th percentile, and a shard of several of them takes twice another's (run 43976170163)",
    file: "scripts/shard.ts",
    find: "    return known.length === 0 ? p90 : known[known.length >> 1]!;\n",
    replace: "    return all[all.length >> 1] ?? 1;\n",
    tests: ["test/ci.test.ts", "-t", "a change's untimed mutations are dealt first"],
  },
  {
    name: "untimed-mutations-to-the-least",
    breaks: "a change's untimed mutations are dealt to the least loaded shard, all to one when they're estimated alike, not in turn",
    file: "scripts/shard.ts",
    find: "    load[k % count] = load[k % count]! + estimate(item);\n    if (k % count === index - 1) mine.push(item);\n",
    replace: "    const at = leastLoaded(load.map((l, i) => (i === 0 ? -1 : l)));\n    load[at] = load[at]! + estimate(item);\n    if (at === index - 1) mine.push(item);\n",
    tests: ["test/ci.test.ts", "-t", "a change's untimed mutations are dealt first"],
  },
];
