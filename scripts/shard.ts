// A set of work split across machines that run at once, as CI runs the
// mutations (DEVELOPMENT.md): shard `index` of `count`. (`bun test` splits
// the suite itself, `--shard`, by how long each file took.)

/** Shard `index`, from 1, of `count`: the items, sorted, dealt out in turn,
 * so each is in exactly one shard, whatever order they're given in. */
export function shard<T extends string>(items: T[], index: number, count: number): T[] {
  if (!Number.isInteger(index) || !Number.isInteger(count) || index < 1 || index > count) {
    throw new Error(`a shard is 1 to ${count}, not ${index}`);
  }
  return [...items].sort().filter((_, i) => i % count === index - 1);
}

/** How many parts a file of cases runs in at once on `cores`, each a
 * process of its own, `RUST_JS_CASES=i/n` (test/support.ts): one for each
 * two cores. A case is a compile, then its runs, one core at a time, so
 * the cores a part leaves are the other files'. On 12 cores, 3 parts took
 * 85 s, 6 take 70 s, and 10 took 72 s, the rest twice as long and a test
 * of them timing out: it's bound by its CPU time. */
export function caseParts(cores: number): number {
  return Math.max(2, Math.floor(cores / 2));
}

/** The parts, `i/n`, of a file of cases that shard `index` of `count`
 * runs, `perShard` of them at once: of `count` times as many in all, each
 * part in exactly one shard. */
export function caseShardParts(index: number, count: number, perShard: number): string[] {
  const parts = count * perShard;
  const all = Array.from({ length: parts }, (_, i) => i + 1);
  return all.filter((part) => (part - 1) % count === index - 1).map((part) => `${part}/${parts}`);
}

/** `--shard=2/4` among `args`: the shard and the count, or nothing. */
export function shardArg(args: string[]): { index: number; count: number } | undefined {
  const arg = args.find((a) => a.startsWith("--shard="));
  if (!arg) return undefined;
  const [index, count] = arg.slice("--shard=".length).split("/").map(Number);
  return { index, count };
}

/** Where the last run's shards' timings of each test file are kept, from
 * `target/`, for `bun test --shard` to balance this run's by: `bun
 * scripts/ci.ts timings` downloads them there, and `bun run test` gives
 * each file in it. */
export const shardTimings = "target/shard-timings";

/** What only the nightly's suite runs, `bun run test --nightly`, not each
 * push's (ADR 1030): what the nightly runs in full anyway, tidy's stages,
 * the iterator forms and generated programs; the release's packaging, which
 * qualification runs too; and the ratchets, React's releases and the
 * harness's own checks. Each is CPU a push's corpus waits on, guards no
 * mutation or a few, and failed none of the 80 red checks before it was
 * moved: a mutation that names one still runs it. */
export const nightly = [
  "tidy-stages",
  "iterator-forms",
  "fuzz",
  "packages",
  "create",
  "npm-crates",
  "release",
  "stable",
  "crates-io",
  "registry-crates",
  "react-versions",
  "next-coverage",
  "react-coverage",
  "webapi-coverage",
  "node-coverage",
  "editor-check",
  "harness",
  "history",
  "limitations",
].map((name) => `test/${name}.test.ts`);
