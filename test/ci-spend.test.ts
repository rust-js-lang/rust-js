// What CI spent each day (scripts/ci-spend.ts, docs/dev-loop.md's "CI
// spend"): each waste said of the jobs that are it, the share of the
// 20-job cap, a push's wait for its verdict, the flags, and the days kept in
// the comment.

import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { comment, flags, historyOf, type Job, type Run, spend } from "../scripts/ci-spend";
import { writeTimes } from "./lend-bench";
import { root } from "./support";

const at = (minute: number) => new Date(Date.UTC(2026, 10, 16, 0, minute)).toISOString().replace(/\.\d+Z$/, "Z");
const run = (id: number, over: Partial<Run> = {}): Run => ({ id, name: "Check", head_branch: "main", head_sha: `s${id}`, event: "push", status: "completed", conclusion: "success", created_at: at(id), updated_at: at(id + 30), run_attempt: 1, ...over });
const job = (run_id: number, name: string, start: number, end: number, over: Partial<Job> = {}): Job => ({ run_id, name, run_attempt: 1, status: "completed", conclusion: "success", created_at: at(start), started_at: at(start), completed_at: at(end), ...over });

test("each waste is the jobs that are it, each job counted once", () => {
  const runs = [
    run(1),
    // Cancelled: every job of it.
    run(2, { conclusion: "cancelled" }),
    // Unread: run 4 began later and gave its verdict first.
    run(3, { updated_at: at(90) }),
    run(4, { updated_at: at(60) }),
    // Re-run: a commit its workflow had run, by the same event.
    run(5, { head_sha: "s1", updated_at: at(100) }),
    // A dispatch of the same commit, even again, is asked for with its own
    // inputs, not repeated.
    run(6, { head_sha: "s1", event: "workflow_dispatch", updated_at: at(100) }),
    run(7, { head_sha: "s1", event: "workflow_dispatch", updated_at: at(100) }),
  ];
  const jobs = [
    job(1, "test (1/2)", 1, 11),
    // A job stopped in a run that wasn't cancelled: timed out.
    job(1, "nets", 1, 4, { conclusion: "cancelled" }),
    // A second attempt's job, and the first's it kept, the same job.
    job(1, "lint", 1, 3, { run_attempt: 2 }),
    job(1, "test (1/2)", 1, 11, { run_attempt: 2 }),
    job(2, "test (1/2)", 2, 7),
    job(3, "test (1/2)", 3, 23),
    job(4, "test (1/2)", 4, 14),
    job(5, "lint", 5, 6),
    job(6, "lint", 6, 7),
    job(7, "lint", 7, 8),
  ];
  const day = spend("2026-11-16", runs, jobs);
  expect(day.waste).toEqual({ cancelled: 5, "re-run": 3, unread: 20, "timed out": 3 });
  expect(day.minutes).toBe(10 + 3 + 2 + 5 + 20 + 10 + 1 + 1 + 1);
  // A matrix's jobs are one kind.
  expect(day.kinds["Check / test"]).toBe(45);
});

test("the cap's minutes and a push's wait for its verdict, its batch's", () => {
  const jobs = Array.from({ length: 21 }, (_, i) => job(1, `j${i}`, 0, i === 20 ? 5 : 10));
  const day = spend("2026-11-16", [run(1, { created_at: at(0), updated_at: at(20) }), run(2, { created_at: at(5), conclusion: "cancelled", updated_at: at(6) }), run(3, { created_at: at(10), updated_at: at(40) })], jobs);
  expect([day.peak, day.atCap]).toEqual([21, 10]);
  // Run 2's push waits for run 3's verdict, the next that wasn't cancelled.
  expect([day.verdicts, day.verdictP50, day.verdictP90]).toEqual([3, 30, 30]);
});

test("waste past 10% of the day and a job kind grown past 30% are flagged, and the days are kept in the comment", () => {
  const day = spend("2026-11-16", [run(1), run(2, { conclusion: "cancelled" })], [job(1, "test (1/2)", 0, 80), job(1, "small", 0, 20), job(2, "test (1/2)", 0, 9)]);
  const before = { kinds: { "Check / test": 60, "Check / small": 10 } };
  // 9 of 109 minutes wasted, under 10%; `small`, under 30 minutes, grew too.
  expect(flags(day, before)).toEqual(["Check / test grew 48%, 60 min to 89 min"]);
  const wasteful = spend("2026-11-16", [run(1), run(2, { conclusion: "cancelled" })], [job(1, "test (1/2)", 0, 80), job(2, "test (1/2)", 0, 20)]);
  expect(flags(wasteful)).toEqual(["waste is 20% of the day (20 min of 100 min), past 10%"]);
  const yesterday = { ...wasteful, date: "2026-11-15" };
  const body = comment(day, historyOf(comment(yesterday, [])));
  expect(historyOf(body).map((d) => d.date)).toEqual(["2026-11-15", "2026-11-16"]);
  expect(body).toContain("| Check / test | 89 min | 0% | 100 min |");
  expect(body.startsWith("<!-- rust-js ci-spend -->")).toBe(true);
  // The nightly writes it on the pinned issue, with its micro-benchmarks.
  expect(readFileSync(join(root, ".github/workflows/nightly.yml"), "utf8")).toContain("bun scripts/ci-spend.ts --comment=1989 --bench=target/bench.json");
});

test("a micro-benchmark past 1.5 times the night before's is flagged, and kept with the days (#2003)", () => {
  const day = { ...spend("2026-11-16", [], []), bench: { "$lent each step, a JS iterator": 31, "$lent each step, std's compiled struct": 40 } };
  const before = { kinds: {}, bench: { "$lent each step, a JS iterator": 20, "$lent each step, std's compiled struct": 30 } };
  expect(flags(day, before)).toEqual(["$lent each step, a JS iterator took 31 ms, 20 ms the night before, past 1.5 times"]);
  const body = comment(day, [{ ...spend("2026-11-15", [], []), ...before }]);
  expect(body).toContain("| $lent each step, a JS iterator | 31 | 20 |");
  expect(historyOf(body).at(-1)?.bench).toEqual(day.bench);
});

// The nightly's `ci-spend` job runs the micro-benchmarks in a checkout that
// built nothing, so the result's directory, `target/`, isn't there yet: the
// benchmark makes it, where it threw `ENOENT` and the job failed (#2372).
test("the micro-benchmarks write their times where no directory is made yet", () => {
  const dir = mkdtempSync(join(tmpdir(), "lend-bench-"));
  try {
    const out = join(dir, "target", "bench.json");
    writeTimes(out, { "$lent each step, a JS iterator": 28.6 });
    expect(JSON.parse(readFileSync(out, "utf8"))).toEqual({ "$lent each step, a JS iterator": 28.6 });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
