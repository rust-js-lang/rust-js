// Which commit of a batched check broke each test it failed
// (scripts/attribute.ts, ADR 1101, amended): a wrong commit named sends the
// fix to the wrong agent.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "attribute-first-failing-run",
    breaks: "a test that broke inside the batch is said to have broken at the commit before, which passed it",
    file: "scripts/attribute.ts",
    find: 'since: "batch" as const };',
    replace: 'since: "batch" as const, ...{ commit: runs[at - 1]!.commit } };',
    tests: ["test/attribute.test.ts", "-t", "each failing test broke at the first commit"],
  },
  {
    name: "attribute-passing-at-newest",
    breaks: "a test that passes at the batch's newest commit, a flaky or machine's failure, is said to be broken by it",
    file: "scripts/attribute.ts",
    find: '    if (at === runs.length) return { unit, commit: runs.at(-1)!.commit, since: "passes" as const };\n',
    replace: "",
    tests: ["test/attribute.test.ts", "-t", "each failing test broke at the first commit"],
  },
  {
    name: "attribute-commits-drop-newest",
    breaks: "a batch longer than six commits isn't run at its newest, the check's own, so what broke a test there goes unsaid",
    file: "scripts/attribute.ts",
    find: "  for (let i = 0; i < most; i++) picked.add(",
    replace: "  for (let i = 0; i < most - 1; i++) picked.add(",
    tests: ["test/attribute.test.ts", "-t", "at most six"],
  },
  {
    name: "attribute-latent-blamed",
    breaks: "a latent survivor, surviving at the batch's base too, is blamed on a commit of the batch",
    file: "scripts/attribute.ts",
    find: 'verdict !== "caught" && !latent.includes(name));',
    replace: 'verdict !== "caught");',
    tests: ["test/attribute.test.ts", "-t", "a failed mutations job names"],
  },
  {
    name: "attribute-oldest-toucher",
    breaks: "a mutation is said changed by the batch's oldest commit that touched it, not its newest, the one whose change it follows",
    file: "scripts/attribute.ts",
    find: "      const by = touching(name, job)[0];",
    replace: "      const by = touching(name, job).at(-1);",
    tests: ["test/attribute.test.ts", "-t", "a failed mutations job names"],
  },
  {
    name: "attribute-errored-shard-unsaid",
    breaks: "a mutations job that failed with no verdict, a shard that errored, is said nothing of",
    file: "scripts/attribute.ts",
    find: "    if (missed.length === 0) {",
    replace: "    if (missed.length === 0 && verdicts === undefined) return [];\n    if (missed.length === 0) {",
    tests: ["test/attribute.test.ts", "-t", "a failed mutations job names"],
  },
  {
    name: "attribute-survivors-job-by-shard-files",
    breaks: "the survivors' job that failed with no verdict is blamed on a commit that changed only the shards' runner, not its own",
    file: "scripts/attribute.ts",
    find: '(/\\/ survivors$/.test(job) ? "survivors" : "shard")',
    replace: '"shard"',
    tests: ["test/attribute.test.ts", "-t", "a failed mutations job names"],
  },
];
