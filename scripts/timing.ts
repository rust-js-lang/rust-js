// How long each step of the development loop takes, kept as it's run, in
// target/timings.jsonl, so a step that grows slower is seen the day it
// does, not once it hurts (DEVELOPMENT.md):
//
//   bun run timings            each step: its last time, its usual, its slowest
//   bun run timings <step>     that step's runs, the latest last
//
// A step records itself, `timed(step, seconds)`, and says how it compares.
import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const file = join(root, "target", "timings.jsonl");

/** When the script that reads this began, about: it's read as it starts. */
const started = Date.now();

/** Seconds since then, by the clock: what the script and all it waited on
 * took. */
export function elapsed(): number {
  return (Date.now() - started) / 1000;
}

export type Timing = { step: string; seconds: number; at: string; commit: string; detail?: string };

/** Each run recorded, the oldest first. */
export function timings(): Timing[] {
  if (!existsSync(file)) return [];
  return readFileSync(file, "utf8")
    .split("\n")
    .filter(Boolean)
    .flatMap((line) => {
      try {
        return [JSON.parse(line) as Timing];
      } catch {
        return [];
      }
    });
}

/** The middle of `values`. */
export function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** `seconds` as a person reads it: `850 ms`, `38 s`, `3 min 20 s`. */
export function shown(seconds: number): string {
  if (seconds < 1) return `${Math.round(seconds * 1000)} ms`;
  if (seconds < 60) return `${seconds < 10 ? seconds.toFixed(1) : Math.round(seconds)} s`;
  const minutes = Math.floor(seconds / 60);
  const rest = Math.round(seconds - minutes * 60);
  return rest ? `${minutes} min ${rest} s` : `${minutes} min`;
}

/** Whether `seconds` is slower than `usual`, by a quarter and five seconds:
 * less is noise. */
export function slower(seconds: number, usual: number): boolean {
  return seconds > usual * 1.25 && seconds - usual > 5;
}

/** `seconds` against the step's last ten runs: `38 s (usual 36 s)`, and a
 * warning where it's slower. */
export function compared(step: string, seconds: number, earlier = timings()): string {
  const before = earlier.filter((t) => t.step === step).slice(-10).map((t) => t.seconds);
  if (before.length < 3) return `${step} ${shown(seconds)}`;
  const usual = median(before);
  const line = `${step} ${shown(seconds)} (usual ${shown(usual)})`;
  return slower(seconds, usual) ? `${line}  ⚠ slower than usual: why?` : line;
}

/** Record that `step` took `seconds`, and say how it compares. */
export function timed(step: string, seconds: number, detail?: string): string {
  const line = compared(step, seconds);
  const commit = Bun.spawnSync(["git", "rev-parse", "--short", "HEAD"], { cwd: root }).stdout.toString().trim();
  mkdirSync(join(root, "target"), { recursive: true });
  const entry: Timing = { step, seconds: Math.round(seconds * 10) / 10, at: new Date().toISOString(), commit, ...(detail ? { detail } : {}) };
  appendFileSync(file, JSON.stringify(entry) + "\n");
  return line;
}

if (import.meta.main) {
  const [step] = process.argv.slice(2);
  const all = timings();
  if (step) {
    for (const t of all.filter((t) => t.step === step).slice(-20)) {
      console.log(`${t.at.slice(0, 16).replace("T", " ")}  ${t.commit}  ${shown(t.seconds).padStart(10)}${t.detail ? `  ${t.detail}` : ""}`);
    }
  } else {
    const steps = [...new Set(all.map((t) => t.step))].sort();
    console.log(`${"step".padEnd(42)}${"runs".padStart(5)}${"last".padStart(12)}${"usual".padStart(12)}${"slowest".padStart(12)}`);
    for (const name of steps) {
      const runs = all.filter((t) => t.step === name);
      const last = runs.at(-1)!.seconds;
      const before = runs.slice(-11, -1).map((t) => t.seconds);
      const usual = before.length ? median(before) : last;
      const flag = before.length >= 3 && slower(last, usual) ? "  ⚠" : "";
      const slowest = Math.max(...runs.slice(-20).map((t) => t.seconds));
      console.log(
        `${name.padEnd(42)}${String(runs.length).padStart(5)}${shown(last).padStart(12)}${shown(usual).padStart(12)}${shown(slowest).padStart(12)}${flag}`,
      );
    }
    if (steps.length === 0) console.log("no timings yet: they're kept as the loop's commands run");
  }
}
