// The one way to push to `main` (DEVELOPMENT.md, "Pushing"): the branch's
// commits, rebased onto `origin/main`, checked as they'll land, then pushed.
// In order, each phase timed:
//
//   rebase    onto origin/main; a conflict stops it, the rebase aborted
//   build     the compiler
//   std       std/ made again (`bun run std`), refused where it differs from
//             what's committed
//   check     `check:fast`: the test files and cases the selector says the
//             change can affect (scripts/select.ts, ADR 1101, amended),
//             everything where it can't say; none at all, of a change
//             nothing reads that isn't code (a doc's), straight to the push.
//             Past a rebase that brought commits, `sanity`, `check:fast
//             --sanity=<checked>`: the interaction of the change's own diff
//             and the commits the rebase brought, what both can affect,
//             capped to its own test files and cases where even that says
//             everything (Tu's rule: quick, then push; CI's check covers the
//             combination). Rebased and checked so until it lands, no limit
//   react     the React port at its pin (`react-in-rust.ts regen`), of a
//             change to what compiles it (`touchesPort`), refused on any
//             difference: a change the port takes moves the pin in this push
//   files     `git diff origin/main` holds only the pushed commits' files
//   push      `git push origin HEAD:main`, once `origin/main` is where the
//             check saw it: where it moved meanwhile, the branch is rebased and
//             its sanity pass run again, so no commit lands that no pass saw
//
//   bun run push                        check and push HEAD
//   bun run push --bless-readability    and bless docs/readability.txt first,
//                                       amending the top commit, its diff printed
//
// A tracked change not committed is refused: the check would see it, the push
// wouldn't. Each run, however it ends, appends a line to the machine's log of
// times, outside the repository (scripts/disk.ts's `devTimesLog`): `<ISO> |
// <machine> | <agent> | push | <sha> | total <s> | <phase> <s>, .. | select
// <outcome> | load <1m/5m/15m> | ok|fail | <note>`, the selection's outcome
// `all (<why>)`, `some (N files, M cases)` or `none` and the map's commit and
// how far behind it is (scripts/select.ts); the machine and agent each
// machine's sessions name (`whoRuns`). Written on every exit, a signal's too.
import { spawnSync } from "node:child_process";
import { appendFileSync, existsSync, readFileSync, rmSync } from "node:fs";
import { loadavg } from "node:os";
import { selectionFile } from "./select";
import { devTimesLog, whoRuns } from "./disk";
import { touchesPort } from "./iteratorSlice";

/** The phases, in the order they run. */
export const phases = ["rebase", "build", "std", "check", "sanity", "react", "files", "push"] as const;
export type Phase = (typeof phases)[number];


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
  selection: string;
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
    `select ${run.selection}`,
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
  let selection = "-";
  // The machine's log's line, written once, however the run ends: past
  // its end, a failure's, and a signal's, which stops `spawnSync`'s child
  // and then runs here (m6-lily's push of b355dc824 wrote none).
  let written = false;
  const writeLine = () => {
    if (written) return;
    written = true;
    const who = whoRuns();
    const unset = who.unset.length > 0 ? ` (${who.unset.join(", ")} unset: the host's name)` : "";
    appendFileSync(
      devTimesLog,
      `${timesLine({
        at: new Date(),
        machine: who.machine,
        agent: who.agent,
        sha: git("rev-parse", "--short", "HEAD").out.trim(),
        total: (performance.now() - started) / 1000,
        phases: timed,
        selection,
        load: loadavg(),
        ok,
        note: `${note}${unset}`,
      })}\n`,
    );
  };
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    process.on(signal, () => {
      note = `stopped by ${signal}`;
      writeLine();
      process.exit(1);
    });
  }
  try {
    if (lines(git("status", "--porcelain", "--untracked-files=no").out).length > 0) {
      throw new Refused("a tracked change isn't committed: the check would see it and the push wouldn't; commit it first");
    }
    let checked = "";
    for (;;) {
      rebase(bless);
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
        // The selection of what the change can affect, by the test map's
        // diff to this tree: first, and again, of the new tree, past a
        // rebase that brought commits (`sanity`), conflicted or not. None
        // of a change that isn't code goes straight on.
        const changed = lines(git("diff", "--name-only", "origin/main").out);
        const name = checked === "" ? "check" : "sanity";
        const code = changed.some((file) => /\.(rs|ts|tsx|js|jsx|mjs|cjs)$/.test(file));
        // CI's map of the nearest commit of `main` it recorded, where it's
        // nearer than the machine's (scripts/testMap.ts's `fetchMap`).
        const fetched = sh(["bun", "scripts/testMap.ts", "fetch"], true).out.trim().split("\n").at(-1) ?? "";
        console.log(`push: ${fetched}`);
        const selected = sh(["bun", "scripts/select.ts"], true);
        if (selected.code !== 0) throw new Refused(`the selector failed:\n${selected.out}`);
        const said = selected.out.split("\n")[0]!.replace(/, by the test map of .*$/, "");
        if (said === "none" && !code) {
          selection = "none";
          console.log(`push: the selection is none, and nothing changed is code: no check`);
        } else {
          rmSync(selectionFile, { force: true });
          run(name, ["bun", "run", "check:fast", ...(name === "sanity" ? [`--sanity=${checked}`] : [])]);
          selection = existsSync(selectionFile) ? JSON.parse(readFileSync(selectionFile, "utf8")).outcome : said;
        }
        // A map not fetched says why on the push's line (b355dc824's said
        // only that there was none).
        if (fetched.startsWith("no map fetched")) selection += ` [${fetched}]`;
        console.log(`push: selected ${selection}`);
        phase(name);
        if (touchesPort(changed) && sh(["bun", "scripts/react-in-rust.ts", "regen"]).code !== 0) {
          throw new Refused("the React port differs at its pin (target/react-in-rust.diff): regenerate the port and move ports/react-in-rust.pin in this push");
        }
        phase("react");
        checked = git("rev-parse", "HEAD").out.trim();
      }
      // Of the commit it was rebased onto and checked over, `base`, not
      // `origin/main` as it is now: a fetch while it was checked, a test's or
      // another worktree's, moved it, and its new commits' files were said
      // to be the branch's, refusing a push the next step would have
      // rebased (#1976's, 2026-11-16).
      const stray = onlyPushedFiles(
        lines(git("diff", "--name-only", base).out),
        lines(git("log", "--format=", "--name-only", `${base}..HEAD`).out),
      );
      if (stray.length > 0) throw new Refused(`the diff against origin/main holds files no pushed commit changes:\n${stray.join("\n")}`);
      phase("files");
      git("fetch", "-q", "origin");
      if (git("rev-parse", "origin/main").out.trim() !== base) {
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
    writeLine();
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
