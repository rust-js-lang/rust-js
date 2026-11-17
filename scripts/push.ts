// The one way to push to `main` (DEVELOPMENT.md, "Pushing"): the branch's
// commits, rebased onto `origin/main`, checked as they'll land, then pushed.
// In order, each phase timed:
//
//   rebase    onto origin/main; a conflict stops it, the rebase aborted
//   build     the compiler
//   std       std/ made again (`bun run std`), refused where it differs from
//             what's committed
//   mutants   each mutation of a file the change touches, or of a list it
//             touches, type-checked (`mutations.ts --typecheck`): a
//             `replace` written against what the change changed, which
//             CI's shards can't build, refused by name
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
import { appendFileSync, existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { loadavg } from "node:os";
import { selectionFile } from "./select";
import { devTimesLog, whoRuns } from "./disk";
import { touchesPort } from "./iteratorSlice";

/** The phases, in the order they run. */
export const phases = ["rebase", "build", "std", "mutants", "check", "sanity", "react", "files", "push"] as const;
export type Phase = (typeof phases)[number];


/** Whether `changed`, the files `git diff origin/main` names, are each one a
 * pushed commit changes, `pushed`'s: none another's, a tree older than
 * `main`'s carries back (scripts/push-guard.ts holds the remote's side). */
/** Whether a round's React gate runs: of a change that touches the port's
 * compile inputs (`touchesPort`), unless an earlier round's passed and the
 * commits the rebase brought since, `incoming`, touch none of them, so the
 * port compiles as it did then: m3-max's long pushes ran it, 54 s, at
 * each of two or three rounds (2026-11-17). */
export function reactGate(changed: string[], incoming: string[] | undefined): "run" | "reused" | "none" {
  if (!touchesPort(changed)) return "none";
  return incoming !== undefined && !touchesPort(incoming) ? "reused" : "run";
}

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
    // The base an earlier round's React gate passed at, where one did.
    let reactBase: string | undefined;
    for (;;) {
      const remade = rebase();
      phase("rebase");
      const base = git("rev-parse", "origin/main").out.trim();
      if (checked === "" || git("rev-parse", "HEAD").out.trim() !== checked) {
        for (const file of new Set([...remade, ...(bless ? ["docs/readability.txt"] : [])])) regenerate(file);
        run("build", ["cargo", "build", "--quiet"]);
        phase("build");
        run("std", ["bun", "run", "std"]);
        const std = lines(git("status", "--porcelain", "--", "std").out);
        if (std.length > 0) throw new Refused(`std/ differs from what's committed, made again from this tree:\n${std.join("\n")}\ncommit it, or rebase onto the commit that does`);
        phase("std");
        run("mutants", ["bun", "scripts/mutations.ts", `--typecheck=${base}`]);
        phase("mutants");
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
        const selected = sh(["bun", "scripts/select.ts", "--push"], true);
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
        const incoming = reactBase === undefined ? undefined : lines(git("diff", "--name-only", reactBase, base).out);
        const gate = reactGate(changed, incoming);
        if (gate === "reused") console.log("push: the React port as the round before: the rebase's commits touch none of what compiles it");
        if (gate === "run") {
          if (sh(["bun", "scripts/react-in-rust.ts", "regen"]).code !== 0) {
            throw new Refused("the React port differs at its pin (target/react-in-rust.diff): regenerate the port and move ports/react-in-rust.pin in this push");
          }
          reactBase = base;
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

/** The blessed baselines, each made again by its test's bless from the
 * tree it's of: what .gitattributes merges as `merge=binary`, a total over
 * its rows that a line-by-line merge would get wrong (test/push.test.ts
 * holds the two lists alike). */
export const regenerated: Record<string, string> = {
  "docs/readability.txt": "test/readability.test.ts",
  "docs/std-coverage.txt": "test/std-coverage.test.ts",
  "builtins/coverage.txt": "test/builtins-coverage.test.ts",
  "next/coverage.txt": "test/next-coverage.test.ts",
  "node/coverage.txt": "test/node-coverage.test.ts",
  "react/coverage.txt": "test/react-coverage.test.ts",
  "webapi/coverage.txt": "test/webapi-coverage.test.ts",
};

/** The ratchets that only fall, each a count several changes lower at
 * once: each file's line `<path>\t<n>`, or a test's `const NAME = n;`,
 * none now, KIN_PARSES at none a strict row (#1681 change 3).
 * Two lowered at once conflict, each side's lower than the base, and the
 * lower of the two holds of the rebased tree, as each only removes what's
 * counted (ADR 1101, amended). */
export const ratchets = ["test/word-reads.txt", "test/sig-binders.txt", "test/type-parameter-reads.txt"];

/** `text`, a ratchet's with git's conflict markers, each conflict resolved
 * by the lower count of each key both sides name, and a key one side alone
 * names kept as it says; or undefined where a conflict holds anything else,
 * which stays the change's author's to merge. */
export function lowerRatchets(text: string): string | undefined {
  const count = /^(?:(\S+)\t(\d+)|const (\w+) = (\d+);)$/;
  const out: string[] = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i]!.startsWith("<<<<<<< ")) {
      out.push(lines[i]!);
      continue;
    }
    const sides: string[][] = [[], []];
    let side = 0;
    let base = false;
    for (i++; i < lines.length && !lines[i]!.startsWith(">>>>>>> "); i++) {
      const line = lines[i]!;
      if (line.startsWith("||||||| ")) base = true;
      else if (line === "=======") (base = false), (side = 1);
      else if (!base) sides[side]!.push(line);
    }
    const merged = new Map<string, { line: string; n: number }>();
    for (const line of [...sides[0]!, ...sides[1]!]) {
      const m = line.match(count);
      if (!m) return undefined;
      const key = m[1] ?? m[3]!;
      const n = Number(m[2] ?? m[4]);
      const was = merged.get(key);
      if (was === undefined || n < was.n) merged.set(key, { line, n });
    }
    out.push(...[...merged.entries()].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)).map(([, { line }]) => line));
  }
  return out.join("\n");
}

/** Whether a rebase's `conflicts` are each a blessed baseline's, which it
 * resolves by making them again, or a ratchet's, by the lower count; any
 * other, hand-written code's too, stops it. */
export function remakeable(conflicts: string[]): boolean {
  return conflicts.every((file) => file in regenerated || ratchets.includes(file));
}

/** Onto origin/main. A conflict in a blessed baseline alone (`regenerated`)
 * is resolved as main's, and made again from the rebased tree: two pushes
 * that each blessed one stopped the second's rebase (2 of m3-max's 5
 * failed pushes of 2026-11-17). Any other conflict aborts it, hand-written
 * code's too. The baselines it resolved so. */
function rebase(): string[] {
  git("fetch", "-q", "origin");
  if (git("rebase", "origin/main").code === 0) return [];
  const remade = new Set<string>();
  for (;;) {
    const conflicts = lines(git("diff", "--name-only", "--diff-filter=U").out);
    if (conflicts.length === 0) break;
    if (!remakeable(conflicts)) {
      git("rebase", "--abort");
      throw new Refused(`the rebase stopped on a conflict, aborted: ${conflicts.join(", ")}`);
    }
    for (const file of conflicts) {
      if (ratchets.includes(file)) {
        const lowered = lowerRatchets(readFileSync(file, "utf8"));
        if (lowered === undefined) {
          git("rebase", "--abort");
          throw new Refused(`the rebase stopped on a conflict in ${file} that isn't a ratchet's count, aborted`);
        }
        writeFileSync(file, lowered);
        git("add", file);
        continue;
      }
      git("checkout", "--ours", file);
      git("add", file);
      remade.add(file);
    }
    if (sh(["git", "-c", "core.editor=true", "rebase", "--continue"], true).code === 0) break;
  }
  return [...remade];
}

/** `file`, a blessed baseline, made again from this tree by its test's
 * bless, its diff printed, and the top commit amended with it where it
 * changed: after a rebase resolved it as main's, and of
 * docs/readability.txt with `--bless-readability`. */
function regenerate(file: string) {
  spawnSync("bun", ["test", regenerated[file]!], { stdio: "ignore", env: { ...process.env, BLESS: "1" } });
  const diff = git("diff", "--", file).out;
  if (diff.trim() === "") return;
  console.log(`push: ${file} blessed:\n${diff}`);
  git("add", file);
  git("commit", "-q", "--amend", "--no-edit");
}

if (import.meta.main) main();
