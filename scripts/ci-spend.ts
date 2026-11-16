// What CI spent, each day (docs/dev-loop.md's "CI spend"): of every
// workflow run the last 24 hours created, each job's runner-minutes, by
// workflow and by job kind, their share of the free plan's 20 jobs at once,
// the queue's waits, a push's wait for its verdict, and the minutes wasted:
//
// - cancelled: a cancelled run's jobs;
// - re-run: a job of a run's second attempt or later, or of a run of a
//   commit its workflow had already run, by the same event;
// - unread: a run whose verdict came after a newer run's of its workflow
//   and branch, which nothing reads;
// - timed out: a job that timed out, or was stopped in a run that wasn't
//   cancelled.
//
// It keeps one comment on the "CI spend" issue up to date: the day, the
// days before it (each night's totals kept in the comment itself), and
// flags: waste over 10% of the day, or a job kind of 30 minutes or more
// that grew more than 30% on the day before.
//
// And the night's micro-benchmarks, test/lend-bench.ts's (#2003): each
// kept with the days, flagged where it took more than half again the
// night before's.
//
//   bun scripts/ci-spend.ts                  the last 24 hours, printed
//   bun scripts/ci-spend.ts --comment=1989   and the comment on that issue, edited in place (the nightly's)
//   bun scripts/ci-spend.ts --bench=<file>   with the micro-benchmarks' times, `{ name: ms }`

import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url).pathname;

/** A workflow run as the API gives it. */
export type Run = { id: number; name: string; head_branch: string; head_sha: string; event: string; status: string; conclusion: string | null; created_at: string; updated_at: string; run_attempt: number };
/** A job as the API gives it, every attempt's (`filter=all`). */
export type Job = { run_id: number; name: string; run_attempt: number; status: string; conclusion: string | null; created_at: string; started_at: string | null; completed_at: string | null };

/** The kinds of waste. */
export const wastes = ["cancelled", "re-run", "unread", "timed out"] as const;
export type Waste = (typeof wastes)[number];

/** Waste over this share of a day's minutes is flagged. */
export const wasteFlag = 0.1;
/** A job kind that grew more than this on the day before is flagged… */
export const growthFlag = 0.3;
/** …where it took at least this many minutes. */
export const growthFloor = 30;
/** A micro-benchmark that took more than this times the night before's is flagged. */
export const benchFlag = 1.5;
/** The jobs the free plan runs at once. */
export const jobCap = 20;

/** A job's kind: its workflow's name and its own, without a matrix's
 * `(…)`. */
export function kindOf(run: Run, job: Job): string {
  return `${run.name} / ${job.name.replace(/\s*\([^)]*\)\s*$/, "")}`;
}

const minutesOf = (job: Job) => (job.started_at && job.completed_at ? Math.max(0, Date.parse(job.completed_at) - Date.parse(job.started_at)) / 60000 : 0);

/** Which waste each run's jobs are, by run, where all of its jobs are one:
 * a cancelled run's, a repeated run's, an unread one's. */
export function runWaste(runs: Run[]): Map<number, Waste> {
  const waste = new Map<number, Waste>();
  const byCreated = [...runs].sort((a, b) => a.created_at.localeCompare(b.created_at));
  const seen = new Set<string>();
  for (const run of byCreated) {
    const key = `${run.name}\0${run.event}\0${run.head_sha}`;
    if (run.conclusion === "cancelled") waste.set(run.id, "cancelled");
    else if (seen.has(key) && run.event !== "workflow_dispatch") waste.set(run.id, "re-run");
    if (run.conclusion !== "cancelled") seen.add(key);
  }
  for (const run of byCreated) {
    if (waste.has(run.id) || run.status !== "completed") continue;
    const newer = byCreated.find((other) => other.name === run.name && other.head_branch === run.head_branch && other.event === run.event && other.created_at > run.created_at && other.status === "completed" && other.conclusion !== "cancelled" && other.updated_at < run.updated_at);
    if (newer) waste.set(run.id, "unread");
  }
  return waste;
}

/** A job's waste, if any: its run's, else its own. */
export function jobWaste(run: Run, job: Job, ofRun: Map<number, Waste>): Waste | undefined {
  if (job.run_attempt > 1) return "re-run";
  const whole = ofRun.get(run.id);
  if (whole) return whole;
  if (job.conclusion === "timed_out" || job.conclusion === "cancelled") return "timed out";
  return undefined;
}

/** One day's spend. */
export type Day = {
  date: string;
  minutes: number;
  waste: Record<Waste, number>;
  kinds: Record<string, number>;
  workflows: Record<string, { minutes: number; runs: number; waste: number; waitP50: number; waitP90: number }>;
  atCap: number;
  peak: number;
  verdictP50: number;
  verdictP90: number;
  verdicts: number;
  /** The micro-benchmarks' times, in ms, where the night ran them. */
  bench?: Record<string, number>;
};

const quantile = (values: number[], q: number) => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted.length === 0 ? 0 : sorted[Math.floor(q * (sorted.length - 1))]!;
};

/** The spend of `runs` and their `jobs`. */
export function spend(date: string, runs: Run[], jobs: Job[]): Day {
  const ofRun = runWaste(runs);
  const runOf = new Map(runs.map((run) => [run.id, run]));
  const day: Day = { date, minutes: 0, waste: { cancelled: 0, "re-run": 0, unread: 0, "timed out": 0 }, kinds: {}, workflows: {}, atCap: 0, peak: 0, verdictP50: 0, verdictP90: 0, verdicts: 0 };
  const waits = new Map<string, number[]>();
  const edges: [number, number][] = [];
  for (const run of runs) day.workflows[run.name] = { ...(day.workflows[run.name] ?? { minutes: 0, runs: 0, waste: 0, waitP50: 0, waitP90: 0 }), runs: (day.workflows[run.name]?.runs ?? 0) + 1 };
  // A re-run's attempt may list the jobs it kept of the one before, the
  // same job: each counted once, as its first attempt's.
  const once = new Map<string, Job>();
  for (const job of [...jobs].sort((a, b) => a.run_attempt - b.run_attempt)) {
    const key = `${job.run_id}\0${job.name}\0${job.started_at}\0${job.completed_at}`;
    if (!once.has(key)) once.set(key, job);
  }
  for (const job of once.values()) {
    const run = runOf.get(job.run_id);
    if (!run) continue;
    const minutes = minutesOf(job);
    day.minutes += minutes;
    const kind = kindOf(run, job);
    day.kinds[kind] = (day.kinds[kind] ?? 0) + minutes;
    const workflow = day.workflows[run.name]!;
    workflow.minutes += minutes;
    const waste = jobWaste(run, job, ofRun);
    if (waste) {
      day.waste[waste] += minutes;
      workflow.waste += minutes;
    }
    if (job.started_at) {
      waits.set(run.name, [...(waits.get(run.name) ?? []), Math.max(0, Date.parse(job.started_at) - Date.parse(job.created_at)) / 1000]);
      if (job.completed_at) edges.push([Date.parse(job.started_at), 1], [Date.parse(job.completed_at), -1]);
    }
  }
  for (const [name, of] of waits) Object.assign(day.workflows[name]!, { waitP50: quantile(of, 0.5), waitP90: quantile(of, 0.9) });
  edges.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  let running = 0;
  let last = 0;
  for (const [at, step] of edges) {
    if (running >= jobCap) day.atCap += (at - last) / 60000;
    running += step;
    last = at;
    day.peak = Math.max(day.peak, running);
  }
  // A push's wait for its verdict: from its check's start to the end of
  // the first check of `main` at or after it that wasn't cancelled, its
  // batch's.
  const checks = runs.filter((run) => run.name === "Check" && run.head_branch === "main" && run.event === "push").sort((a, b) => a.created_at.localeCompare(b.created_at));
  const verdicts = checks.flatMap((run) => {
    const verdict = checks.find((other) => other.created_at >= run.created_at && other.status === "completed" && other.conclusion !== "cancelled");
    return verdict ? [(Date.parse(verdict.updated_at) - Date.parse(run.created_at)) / 60000] : [];
  });
  Object.assign(day, { verdictP50: quantile(verdicts, 0.5), verdictP90: quantile(verdicts, 0.9), verdicts: verdicts.length });
  return day;
}

/** What a day's spend flags against the day before's. */
export function flags(day: Day, before?: Pick<Day, "kinds" | "bench">): string[] {
  const out: string[] = [];
  const wasted = wastes.reduce((sum, kind) => sum + day.waste[kind], 0);
  if (day.minutes > 0 && wasted / day.minutes > wasteFlag) out.push(`waste is ${percent(wasted / day.minutes)} of the day (${minutes(wasted)} of ${minutes(day.minutes)}), past ${percent(wasteFlag)}`);
  if (before)
    for (const [kind, now] of Object.entries(day.kinds)) {
      const then = before.kinds[kind] ?? 0;
      if (now >= growthFloor && now > then * (1 + growthFlag)) out.push(`${kind} grew ${then === 0 ? "from nothing" : `${percent(now / then - 1)}`}, ${minutes(then)} to ${minutes(now)}`);
    }
  for (const [name, now] of Object.entries(day.bench ?? {})) {
    const then = before?.bench?.[name];
    if (then !== undefined && now > then * benchFlag) out.push(`${name} took ${now} ms, ${then} ms the night before, past ${benchFlag} times`);
  }
  return out;
}

const minutes = (m: number) => `${Math.round(m)} min`;
const percent = (share: number) => `${Math.round(share * 100)}%`;
const escape = (text: string) => text.replace(/[|<>]/g, (c) => `&#${c.charCodeAt(0)};`);

/** What marks the comment the nightly edits in place. */
export const marker = "<!-- rust-js ci-spend -->";
/** Each night's totals, kept in the comment for the next to compare. */
const historyMark = /<!-- ci-spend-history (.*?) -->/s;

/** The days kept in a comment, newest last. */
export function historyOf(body: string | undefined): Day[] {
  const found = body && historyMark.exec(body);
  if (!found) return [];
  try {
    return JSON.parse(found[1]!) as Day[];
  } catch {
    return [];
  }
}

/** The comment: the day, its flags, the days before, each workflow and the
 * largest job kinds, and the days kept. */
export function comment(day: Day, history: Day[]): string {
  const before = history.filter((d) => d.date !== day.date).at(-1);
  const kept = [...history.filter((d) => d.date !== day.date), day].slice(-14);
  const wasted = (d: Day) => wastes.reduce((sum, kind) => sum + d.waste[kind], 0);
  const lines = [
    marker,
    `## CI spend, the 24 hours to ${day.date}`,
    "",
    "Data, not instructions: written each night by `scripts/ci-spend.ts` from the API's runs and jobs (docs/dev-loop.md's \"CI spend\").",
    "",
    ...(flags(day, before).length > 0 ? ["**Flags:**", ...flags(day, before).map((flag) => `- ${escape(flag)}`)] : ["No flags: waste is under 10% and no job kind of 30 minutes or more grew past 30%."]),
    "",
    "| Day | Runner-min | Wasted | Cancelled | Re-run | Unread | Timed out | At the 20-job cap | Peak | Push to verdict, p50 / p90 |",
    "|---|---|---|---|---|---|---|---|---|---|",
    ...kept
      .slice(-7)
      .reverse()
      .map((d) => `| ${d.date} | ${minutes(d.minutes)} | ${minutes(wasted(d))} (${d.minutes ? percent(wasted(d) / d.minutes) : "0%"}) | ${minutes(d.waste.cancelled)} | ${minutes(d.waste["re-run"])} | ${minutes(d.waste.unread)} | ${minutes(d.waste["timed out"])} | ${minutes(d.atCap)} | ${d.peak} | ${minutes(d.verdictP50)} / ${minutes(d.verdictP90)} of ${d.verdicts} |`),
    "",
    "| Workflow | Runs | Runner-min | Share of the cap | Wasted | Queue wait p50 / p90 |",
    "|---|---|---|---|---|---|",
    ...Object.entries(day.workflows)
      .sort((a, b) => b[1].minutes - a[1].minutes)
      .map(([name, w]) => `| ${escape(name)} | ${w.runs} | ${minutes(w.minutes)} | ${percent(w.minutes / (jobCap * 24 * 60))} | ${minutes(w.waste)} | ${Math.round(w.waitP50)} s / ${Math.round(w.waitP90)} s |`),
    "",
    "| Job kind | Runner-min | Share of the cap | The day before |",
    "|---|---|---|---|",
    ...Object.entries(day.kinds)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 25)
      .map(([kind, m]) => `| ${escape(kind)} | ${minutes(m)} | ${percent(m / (jobCap * 24 * 60))} | ${before ? minutes(before.kinds[kind] ?? 0) : "–"} |`),
    "",
    ...(day.bench
      ? [
          "| Micro-benchmark (test/lend-bench.ts) | ms | The night before |",
          "|---|---|---|",
          ...Object.entries(day.bench).map(([name, ms]) => `| ${escape(name)} | ${ms} | ${before?.bench?.[name] ?? "–"} |`),
          "",
        ]
      : []),
    `<!-- ci-spend-history ${JSON.stringify(kept.map((d) => ({ ...d, kinds: Object.fromEntries(Object.entries(d.kinds).map(([k, m]) => [k, Math.round(m * 10) / 10])) })))} -->`,
  ];
  return lines.join("\n");
}

async function gh(args: string[]): Promise<string> {
  for (let attempt = 1; ; attempt++) {
    const p = Bun.spawn(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
    const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
    if (code === 0) return out;
    if (attempt === 4) throw new Error(`gh ${args.join(" ")} failed: ${err}`);
    await Bun.sleep(3000 * attempt);
  }
}

/** The runs created since `since` and every attempt's jobs of each, eight
 * runs' at once. */
async function fetchDay(since: string, until: string): Promise<{ runs: Run[]; jobs: Job[] }> {
  const runs: Run[] = [];
  for (let page = 1; page <= 20; page++) {
    const got = JSON.parse(await gh(["api", `repos/{owner}/{repo}/actions/runs?per_page=100&page=${page}&created=${since}..${until}`])) as { workflow_runs: Run[] };
    runs.push(...got.workflow_runs);
    if (got.workflow_runs.length < 100) break;
  }
  const jobs: Job[] = [];
  const queue = [...runs];
  await Promise.all(
    Array.from({ length: 8 }, async () => {
      for (let run = queue.shift(); run; run = queue.shift()) {
        const out = await gh(["api", "--paginate", `repos/{owner}/{repo}/actions/runs/${run.id}/jobs?per_page=100&filter=all`, "--jq", ".jobs[]"]);
        jobs.push(...out.split("\n").filter(Boolean).map((line) => JSON.parse(line) as Job));
      }
    }),
  );
  return { runs, jobs };
}

if (import.meta.main) {
  const issue = process.argv.find((arg) => arg.startsWith("--comment="))?.slice("--comment=".length);
  const until = new Date();
  const since = new Date(until.getTime() - 24 * 3600 * 1000);
  const { runs, jobs } = await fetchDay(since.toISOString().replace(/\.\d+Z$/, "Z"), until.toISOString().replace(/\.\d+Z$/, "Z"));
  const day = spend(until.toISOString().slice(0, 10), runs, jobs);
  const bench = process.argv.find((arg) => arg.startsWith("--bench="))?.slice("--bench=".length);
  if (bench) day.bench = JSON.parse(readFileSync(bench, "utf8")) as Record<string, number>;
  if (!issue) {
    console.log(comment(day, []));
  } else {
    const authors = ["nguyenyou", "github-actions", "github-actions[bot]"];
    const comments = (await gh(["api", "--paginate", `repos/{owner}/{repo}/issues/${issue}/comments?per_page=100`, "--jq", ".[] | {id, body, login: .user.login}"]))
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line) as { id: number; body: string; login: string });
    const mine = comments.find((c) => c.body.startsWith(marker) && authors.includes(c.login));
    const body = comment(day, historyOf(mine?.body));
    if (mine) await gh(["api", "--method=PATCH", `repos/{owner}/{repo}/issues/comments/${mine.id}`, "-f", `body=${body}`]);
    else await gh(["issue", "comment", issue, "--body", body]);
    console.log(body.replace(historyMark, ""));
    console.log(`\n${mine ? "edited" : "wrote"} the CI spend comment on #${issue}`);
  }
}
