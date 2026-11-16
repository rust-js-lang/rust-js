// The one way to push to `main` (DEVELOPMENT.md, "Pushing"): the branch's
// commits, rebased onto `origin/main`, checked as they'll land, then pushed.
// In order, each phase timed:
//
//   rebase    onto origin/main; a conflict stops it, the rebase aborted
//   build     the compiler
//   std       std/ made again (`bun run std`), refused where it differs from
//             what's committed
//   check     `check:fast --all`; or, past a clean rebase that brought in
//             commits after the whole check passed, `sanity`, plain
//             `check:fast`, what the pushed change touches: CI's check of
//             the push covers the combination (Tu's decision, 2026-11-16)
//   react     the React port at its pin (`react-in-rust.ts regen`), refused on
//             any difference: a change the port takes moves the pin in this push
//   files     `git diff origin/main` holds only the pushed commits' files
//   push      `git push origin HEAD:main`, once `origin/main` is where the
//             check saw it: where it moved meanwhile, the branch is rebased and
//             checked again, so no commit lands that the check didn't see
//
//   bun run push                        check and push HEAD
//   bun run push --bless-readability    and bless docs/readability.txt first,
//                                       amending the top commit, its diff printed
//
// A tracked change not committed is refused: the check would see it, the push
// wouldn't. Each run, however it ends, appends a line to the machine's log of
// times, outside the repository (scripts/disk.ts's `devTimesLog`): `<ISO> |
// <machine> | <agent> | push | <sha> | total <s> | <phase> <s>, .. | load
// <1m/5m/15m> | ok|fail | <note>`.
import { spawnSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { hostname, loadavg } from "node:os";
import { devTimesLog } from "./disk";

/** The phases, in the order they run. */
export const phases = ["rebase", "build", "std", "check", "sanity", "react", "files", "push"] as const;
export type Phase = (typeof phases)[number];

/** How many times the branch is rebased and checked again where `main` moves
 * while it's checked, before the push gives up and says to run it again. */
export const attempts = 3;

/** Whether `changed`, the files `git diff origin/main` names, are each one a
 * pushed commit changes, `pushed`'s: none another's, a tree older than
 * `main`'s carries back (scripts/push-guard.ts holds the remote's side). */
export function onlyPushedFiles(changed: string[], pushed: string[]): string[] {
  const ours = new Set(pushed);
  return changed.filter((file) => !ours.has(file));
}

/** The machine log's line of a run (DEVELOPMENT.md, "Pushing"). */
export function timesLine(run: {
  at: Date;
  machine: string;
  agent: string;
  sha: string;
  total: number;
  phases: [string, number][];
  load: number[];
  ok: boolean;
  note: string;
}): string {
  const phases = run.phases.map(([name, s]) => `${name} ${Math.round(s)}s`).join(", ") || "-";
  const load = run.load.map((l) => l.toFixed(2)).join("/");
  return [
    run.at.toISOString().replace(/\.\d+Z$/, "Z"),
    run.machine,
    run.agent,
    "push",
    run.sha,
    `total ${Math.round(run.total)}s`,
    phases,
    `load ${load}`,
    run.ok ? "ok" : "fail",
    run.note,
  ].join(" | ");
}

function sh(cmd: string[], quiet = false): { code: number; out: string } {
  const p = spawnSync(cmd[0], cmd.slice(1), { encoding: "utf8", maxBuffer: 256 * 1024 * 1024, stdio: quiet ? "pipe" : ["ignore", "pipe", "pipe"] });
  const out = `${p.stdout ?? ""}${p.stderr ?? ""}`;
  return { code: p.status ?? 1, out };
}
const git = (...args: string[]) => sh(["git", ...args], true);
const lines = (s: string) => s.split("\n").map((l) => l.trim()).filter(Boolean);

class Refused extends Error {}

function main() {
  const bless = process.argv.includes("--bless-readability");
  const started = performance.now();
  const timed: [string, number][] = [];
  let mark = performance.now();
  const phase = (name: Phase) => {
    const now = performance.now();
    timed.push([name, (now - mark) / 1000]);
    console.log(`push: ${name} ${((now - mark) / 1000).toFixed(0)} s`);
    mark = now;
  };
  let ok = false;
  let note = "";
  try {
    if (lines(git("status", "--porcelain", "--untracked-files=no").out).length > 0) {
      throw new Refused("a tracked change isn't committed: the check would see it and the push wouldn't; commit it first");
    }
    let checked = "";
    for (let attempt = 1; ; attempt++) {
      const conflicted = rebase(bless);
      phase("rebase");
      const base = git("rev-parse", "origin/main").out.trim();
      if (checked === "" || git("rev-parse", "HEAD").out.trim() !== checked) {
        if (bless) blessReadability();
        run("build", ["cargo", "build", "--quiet"]);
        phase("build");
        run("std", ["bun", "run", "std"]);
        const std = lines(git("status", "--porcelain", "--", "std").out);
        if (std.length > 0) throw new Refused(`std/ differs from what's committed, made again from this tree:\n${std.join("\n")}\ncommit it, or rebase onto the commit that does`);
        phase("std");
        // The whole check first, and again past a rebase that stopped on a
        // conflict; past a clean one, what the change touches.
        const whole = checked === "" || conflicted;
        run(whole ? "check" : "sanity", ["bun", "run", "check:fast", ...(whole ? ["--all"] : [])]);
        phase(whole ? "check" : "sanity");
        if (sh(["bun", "scripts/react-in-rust.ts", "regen"]).code !== 0) {
          throw new Refused("the React port differs at its pin (target/react-in-rust.diff): regenerate the port and move ports/react-in-rust.pin in this push");
        }
        phase("react");
        checked = git("rev-parse", "HEAD").out.trim();
      }
      const stray = onlyPushedFiles(
        lines(git("diff", "--name-only", "origin/main").out),
        lines(git("log", "--format=", "--name-only", "origin/main..HEAD").out),
      );
      if (stray.length > 0) throw new Refused(`the diff against origin/main holds files no pushed commit changes:\n${stray.join("\n")}`);
      phase("files");
      git("fetch", "-q", "origin");
      if (git("rev-parse", "origin/main").out.trim() !== base) {
        if (attempt >= attempts) throw new Refused(`main moved ${attempts} times while this was checked: run \`bun run push\` again`);
        console.log("push: main moved while this was checked; rebasing and checking again");
        continue;
      }
      const pushed = git("push", "-q", "origin", "HEAD:main");
      git("fetch", "-q", "origin");
      if (pushed.code !== 0 || git("rev-parse", "HEAD").out.trim() !== git("rev-parse", "origin/main").out.trim()) {
        throw new Refused(`the push didn't land:\n${pushed.out.trim()}`);
      }
      phase("push");
      console.log(`push: ${git("log", "--oneline", "-1", "origin/main").out.trim()}`);
      ok = true;
      break;
    }
  } catch (e) {
    note = e instanceof Refused ? e.message.split("\n")[0] : String(e).split("\n")[0];
    console.error(`push: not pushed: ${e instanceof Error ? e.message : e}`);
  } finally {
    appendFileSync(
      devTimesLog,
      `${timesLine({
        at: new Date(),
        machine: process.env.RUST_JS_MACHINE ?? hostname(),
        agent: process.env.RUST_JS_AGENT ?? "-",
        sha: git("rev-parse", "--short", "HEAD").out.trim(),
        total: (performance.now() - started) / 1000,
        phases: timed,
        load: loadavg(),
        ok,
        note,
      })}\n`,
    );
  }
  process.exit(ok ? 0 : 1);
}

/** A command of a phase, its output shown; refused where it fails. */
function run(name: Phase, cmd: string[]) {
  const p = spawnSync(cmd[0], cmd.slice(1), { stdio: "inherit" });
  if (p.status !== 0) throw new Refused(`${name} failed: ${cmd.join(" ")}`);
}

/** Onto origin/main; a conflict aborts it. Of docs/readability.txt alone, a
 * blessed baseline (`merge=binary`), with `--bless-readability`: main's, then
 * blessed again from the rebased tree (DEVELOPMENT.md, "Landing a branch").
 * Whether it stopped on a conflict, resolved so. */
function rebase(bless: boolean): boolean {
  git("fetch", "-q", "origin");
  if (git("rebase", "origin/main").code === 0) return false;
  for (;;) {
    const conflicts = lines(git("diff", "--name-only", "--diff-filter=U").out);
    if (conflicts.length === 0) break;
    if (!(bless && conflicts.length === 1 && conflicts[0] === "docs/readability.txt")) {
      git("rebase", "--abort");
      throw new Refused(`the rebase stopped on a conflict, aborted: ${conflicts.join(", ")}`);
    }
    git("checkout", "--ours", "docs/readability.txt");
    git("add", "docs/readability.txt");
    if (sh(["git", "-c", "core.editor=true", "rebase", "--continue"], true).code === 0) break;
  }
  return true;
}

/** docs/readability.txt blessed from this tree, its diff printed, and the top
 * commit amended with it where it changed (`--bless-readability`). */
function blessReadability() {
  spawnSync("bun", ["test", "test/readability.test.ts"], { stdio: "ignore", env: { ...process.env, BLESS: "1" } });
  const diff = git("diff", "--", "docs/readability.txt").out;
  if (diff.trim() === "") return;
  console.log(`push: docs/readability.txt blessed:\n${diff}`);
  git("add", "docs/readability.txt");
  git("commit", "-q", "--amend", "--no-edit");
}

if (import.meta.main) main();
