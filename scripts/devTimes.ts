// The machine's log of the dev loop's times, outside the repository: a
// line a run of `bun run push`, `check:fast`, the corpus run on its own,
// `scripts/mutations.ts` (`--retire` too) and React's port's regen, each
// written by the run itself, however it ends, so no agent has to remember
// to (lisa agent's night of numbers went with a compacted context). The one
// writer: `<ISO> | <machine> | <agent> | <kind> | <sha> | total <s> |
// <step> <s>, .. | select <outcome> | load <1m/5m/15m>, other runs <start>/<end>
// | ok|fail | <note>` (DEVELOPMENT.md, "Pushing").
import { spawnSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { loadavg } from "node:os";
import { devTimesLog } from "../test/bounded";
import { liveLeases } from "./leases";

/** Who a line of the log is of: the machine and the agent its sessions
 * name, `RUST_JS_MACHINE` and `RUST_JS_AGENT`, each machine's own; or,
 * where either isn't set, which, `unset`, and no line is written: one
 * under the host's name sat apart from the same machine's runs, a
 * check:fast's `<host>.local | -` beside its pushes' `m6 | ioX` (#2322). */
export function whoRuns(env: Record<string, string | undefined> = process.env): { machine: string; agent: string } | { unset: string[] } {
  const unset = ["RUST_JS_MACHINE", "RUST_JS_AGENT"].filter((name) => !env[name]);
  if (unset.length > 0) return { unset };
  return { machine: env.RUST_JS_MACHINE!, agent: env.RUST_JS_AGENT! };
}

/** A run's line of the log. */
export type TimesRun = {
  at: Date;
  machine: string;
  agent: string;
  kind: string;
  sha: string;
  total: number;
  phases: [string, number][];
  selection: string;
  load: number[];
  /** How many other runs held a lease of the machine's cores as it began
   * and as it ended (scripts/leases.ts): a run that shared them took them
   * apart, so a run of twice its time says why, an overlapped corpus's
   * 1,190 s where a quiet one's 658 s (2026-11-21). */
  others?: [number, number];
  ok: boolean;
  note: string;
};

/** How many runs other than this process hold a lease of the machine's
 * cores now. */
export function otherRuns(pid = process.pid): number {
  try {
    return liveLeases().filter((lease) => lease.pid !== pid).length;
  } catch {
    return 0;
  }
}

/** The log's line of a run. */
export function timesLine(run: TimesRun): string {
  const phases = run.phases.map(([name, s]) => `${name} ${Math.round(s)}s`).join(", ") || "-";
  const load = run.load.map((l) => l.toFixed(2)).join("/") + (run.others ? `, other runs ${run.others[0]}/${run.others[1]}` : "");
  return [
    run.at.toISOString().replace(/\.\d+Z$/, "Z"),
    run.machine,
    run.agent,
    run.kind,
    run.sha,
    `total ${Math.round(run.total)}s`,
    phases,
    `select ${run.selection}`,
    `load ${load}`,
    run.ok ? "ok" : "fail",
    run.note,
  ].join(" | ");
}

/** Append a run of `kind`'s line to the machine's log, from its start,
 * `started` (`performance.now()`), its steps, `phases`, the slowest first
 * where there are many, and whether it passed. The checkout's `HEAD` is
 * its sha, `cwd`'s. Never fails the run it's of. */
export function devTimes(
  kind: string,
  { started, phases = [], selection = "-", ok, note = "", cwd = process.cwd() }: { started: number; phases?: [string, number][]; selection?: string; ok: boolean; note?: string; cwd?: string },
): void {
  try {
    const who = whoRuns();
    if ("unset" in who) {
      console.error(`${kind}'s times not written to ${devTimesLog}: ${who.unset.join(" and ")} unset, which name the machine and the agent of its line`);
      return;
    }
    const sha = spawnSync("git", ["rev-parse", "--short", "HEAD"], { cwd, encoding: "utf8" }).stdout?.trim() || "-";
    const began = Number(process.env.RUST_JS_TIMED_OTHERS);
    const others: [number, number] | undefined = Number.isFinite(began) && process.env.RUST_JS_TIMED_OTHERS !== undefined ? [began, otherRuns()] : undefined;
    const line = timesLine({ at: new Date(), machine: who.machine, agent: who.agent, kind, sha, total: (performance.now() - started) / 1000, phases, selection, load: loadavg(), others, ok, note });
    appendFileSync(devTimesLog, `${line}\n`);
  } catch {
    // A log that can't be written says nothing of the run.
  }
}

/** Whether a run of `kind` writes its own line: not where it's a step of
 * a run that writes one, a push's check:fast, a retirement's port regen or
 * check:fast's corpus parts, which the environment says (`RUST_JS_TIMED_BY`,
 * which a run that writes one sets for what it runs). */
export function ownsTimes(kind: string, env: Record<string, string | undefined> = process.env, others = otherRuns): boolean {
  if (env.RUST_JS_TIMED_BY) return false;
  env.RUST_JS_TIMED_BY = kind;
  // The other runs as it begins, which its line gives beside those as it
  // ends (`TimesRun`'s `others`).
  env.RUST_JS_TIMED_OTHERS = String(others());
  return true;
}

/** Of steps' times, the `n` slowest, slowest first: a long run's line. */
export function slowest(steps: [string, number][], n = 5): [string, number][] {
  return [...steps].sort((a, b) => b[1] - a[1]).slice(0, n);
}
