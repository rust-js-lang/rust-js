// The runners a scheduled workflow may hold at once (scripts/runners.ts):
// the free plan's 20 jobs across the account, the Nightly's share of them.
import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { evaluateIf, heldJobs, mostAtOnce, NIGHTLY_RUNNERS, scheduled } from "../scripts/runners";
import { root } from "./support";

// The free plan runs 20 jobs at once across the account, and the Nightly
// held all 20 for most of its run: its three nights before the lanes,
// 74 to 101 minutes past 14, while the pushes' checks and their
// attribution queued behind it. A scheduled workflow holds at most
// `NIGHTLY_RUNNERS`, its worst case as scripts/runners.ts reads its YAML:
// each matrix job as many at once as its `max-parallel` lets, every
// `if:` running but one its caller's inputs decide false.
test("a scheduled workflow holds at most NIGHTLY_RUNNERS of the free plan's runners at once", () => {
  const dir = join(root, ".github/workflows");
  const files = scheduled((name) => readFileSync(join(dir, name), "utf8"), readdirSync(dir).filter((name) => name.endsWith(".yml")));
  expect(files).toContain("nightly.yml");
  for (const file of files) {
    const { jobs, order } = heldJobs(file);
    const unbounded = jobs.filter((job) => !Number.isFinite(job.at)).map((job) => `${job.id}: ${job.why}`);
    expect([file, unbounded]).toEqual([file, []]);
    expect([file, mostAtOnce(jobs, order) <= NIGHTLY_RUNNERS]).toEqual([file, true]);
  }
});

// Its parts: an `if:` the caller's inputs decide is decided, one a run's
// results decide counts as running; a job is ruled out with what it
// needs, unless its `if:` asks a status; and the most at once is of jobs
// no `needs` orders, each its own runners.
test("the runners' bound reads `if:` of inputs alone, and counts jobs no need orders", () => {
  expect(evaluateIf("${{ !inputs.nightly && needs.plan.outputs.iterators == 'true' }}", { nightly: true })).toBe(false);
  expect(evaluateIf("${{ !inputs.nightly && needs.plan.outputs.iterators == 'true' }}", { nightly: false })).toBe("?");
  expect(evaluateIf("${{ needs.mutations.result == 'failure' }}", {})).toBe("?");
  expect(evaluateIf("inputs.tests != ''", {})).toBe(false);
  expect(evaluateIf("${{ inputs.part != 'mutations' && (inputs.nightly || needs.plan.outputs.wasm == 'true') }}", { part: "suite", nightly: true })).toBe(true);
  expect(evaluateIf("always() && inputs.fuzz_seeds != ''", { fuzz_seeds: "600" })).toBe("?");
  const yaml: Record<string, string> = {
    "w.yml": `jobs:
  a: { runs-on: x, if: "\${{ inputs.go }}" }
  b: { runs-on: x, needs: a }
  c: { runs-on: x, needs: a, if: "always()" }
  d: { runs-on: x, strategy: { max-parallel: "\${{ fromJSON(inputs.wide || '3') }}", matrix: { n: [1, 2, 3, 4] } } }
  e: { runs-on: x, needs: d, strategy: { matrix: { n: "\${{ fromJSON(needs.d.outputs.n) }}" } } }
`,
  };
  const read = (name: string) => yaml[name]!;
  const held = (inputs: Record<string, unknown>) => Object.fromEntries(heldJobs("w.yml", inputs, "", read).jobs.map((job) => [job.id, job.at]));
  expect(held({ go: false, wide: "2" })).toEqual({ a: 0, b: 0, c: 1, d: 2, e: Number.POSITIVE_INFINITY });
  expect(held({ go: true })).toEqual({ a: 1, b: 1, c: 1, d: 3, e: Number.POSITIVE_INFINITY });
  // An input is a string, and a matrix whose `max-parallel` is a string
  // gets no job, with no error: nightly 44216138054 ran none of its
  // suite's, rustc tests', test map's or mutations' matrices.
  const bare: Record<string, string> = { "v.yml": "jobs:\n  d: { runs-on: x, strategy: { max-parallel: \"${{ inputs.wide || 3 }}\", matrix: { n: [1, 2] } } }\n" };
  expect(() => heldJobs("v.yml", { wide: "2" }, "", (name) => bare[name]!)).toThrow("write `${{ fromJSON(inputs.<name> || '<n>') }}`");
  const job = (id: string, at: number) => ({ id, at });
  expect(mostAtOnce([job("a", 1), job("b", 1), job("c", 1)], [["a", "b"], ["b", "c"]])).toBe(1);
  expect(mostAtOnce([job("a", 2), job("b", 3)], [])).toBe(5);
  // A diamond: a before b and c, both before d; b and c at once.
  expect(mostAtOnce([job("a", 4), job("b", 2), job("c", 3), job("d", 1)], [["a", "b"], ["a", "c"], ["b", "d"], ["c", "d"]])).toBe(5);
  // A long lane beside a short one: the lane's widest job, not its sum.
  expect(mostAtOnce([job("l1", 3), job("l2", 1), job("s", 2)], [["l1", "l2"]])).toBe(5);
});
