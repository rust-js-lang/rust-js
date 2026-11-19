// The one way to push to `main` (DEVELOPMENT.md, "Pushing"): the branch's
// commits, rebased onto `origin/main`, checked as they'll land, then pushed.
// In order, each phase timed:
//
//   rebase    onto origin/main; a conflict stops it, the rebase aborted, but
//             one each side's change merges mechanically: a blessed baseline,
//             a ratchet, an ADR's two amendments, a line set (`remakeable`)
//   fast      in seconds, what would otherwise refuse it minutes later:
//             push-guard's `undone`, which git's hook runs as it pushes, at
//             the end; and the mutation catalog's test file whole
//             (`FAST_TESTS`), which check:fast runs minutes in
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
//             everything, and of either what fits its budget, those that
//             reach most of the change first (scripts/select.ts's
//             `budgeted`; Tu's rule: quick, then push; CI's check covers
//             the combination). Rebased and checked so until it lands, no limit
//   react     the React port at its pin (`react-in-rust.ts regen`), of a
//             change to what compiles it (`touchesPort`), refused on any
//             difference: a change the port takes moves the pin in this push.
//             It runs beside the round's check, started before it, its
//             output in target/push-react.log: the phase is what's left of
//             it once the check ends
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
// times, outside the repository (scripts/devTimes.ts, its one writer): `<ISO> |
// <machine> | <agent> | push | <sha> | total <s> | <phase> <s>, .. | select
// <outcome> | load <1m/5m/15m> | ok|fail | <note>`, the selection's outcome
// `all (<why>)`, `some (N files, M cases)` or `none` and the map's commit and
// how far behind it is (scripts/select.ts); the machine and agent each
// machine's sessions name (`whoRuns`). Written on every exit, a signal's too.
import { spawn, spawnSync } from "node:child_process";
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { selectionFile } from "./select";
import { devTimes, ownsTimes } from "./devTimes";
import { PORT_DOES_NOT_COMPILE, touchesPort } from "./iteratorSlice";
import { refusal, undone } from "./push-guard";

/** The phases, in the order they run. */
export const phases = ["rebase", "fast", "build", "std", "mutants", "check", "sanity", "react", "files", "push"] as const;

/** The `fast` phase's tests: the mutation catalog's file whole, whose
 * failure, a `find` its code moved past, a test renamed or moved, or a
 * catalog that names another file than its own, check:fast found minutes
 * in: about 4 s on the Mac mini, where its two tests picked alone let the
 * last through (d4c76fa8b's first push). */
export const FAST_TESTS = ["test/mutations.test.ts"];
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

/** Why the push refuses where the React port's regen exits `code`: a
 * crate that doesn't compile, which writes no JS and so no diff, or JS
 * that differs from the pin's. */
export function portRefusal(code: number, log: string): string {
  return code === PORT_DOES_NOT_COMPILE
    ? `the React port doesn't compile at its pin (its run's output in ${log}): its JS isn't there to compare; fix the compiler, or the port and its pin`
    : `the React port differs at its pin (target/react-in-rust.diff; its run's output in ${log}): regenerate the port and move ports/react-in-rust.pin in this push`;
}

export function onlyPushedFiles(changed: string[], pushed: string[]): string[] {
  const ours = new Set(pushed);
  return changed.filter((file) => !ours.has(file));
}

function sh(cmd: string[], quiet = false): { code: number; out: string } {
  const p = spawnSync(cmd[0], cmd.slice(1), { encoding: "utf8", maxBuffer: 256 * 1024 * 1024, stdio: quiet ? "pipe" : ["ignore", "pipe", "pipe"] });
  const out = `${p.stdout ?? ""}${p.stderr ?? ""}`;
  return { code: p.status ?? 1, out };
}
const git = (...args: string[]) => sh(["git", ...args], true);
const lines = (s: string) => s.split("\n").map((l) => l.trim()).filter(Boolean);

class Refused extends Error {}

async function main() {
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
  let written = !ownsTimes("push");
  const writeLine = () => {
    if (written) return;
    written = true;
    devTimes("push", { started, phases: timed, selection, ok, note });
  };
  // The round's React regen, where one runs beside its check.
  let regen: Beside | undefined;
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    process.on(signal, () => {
      regen?.stop();
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
      // What git's hook refuses as it pushes, and the catalog's own tests,
      // before minutes of building and checking.
      const found = undone(process.cwd(), base, "HEAD");
      if (found.length > 0) throw new Refused(refusal(found));
      run("fast", ["bun", "test", ...FAST_TESTS]);
      phase("fast");
      if (checked === "" || git("rev-parse", "HEAD").out.trim() !== checked) {
        for (const file of new Set([...remade, ...(bless ? ["docs/readability.txt"] : [])])) regenerate(file);
        run("build", ["cargo", "build", "--quiet"]);
        phase("build");
        // std/ made again and compared, the one check of it, CI's lint's too
        // (scripts/std-fresh.ts), which names the fix where it differs.
        run("std", ["bun", "scripts/std-fresh.ts"]);
        phase("std");
        run("mutants", ["bun", "scripts/mutations.ts", `--typecheck=${base}`]);
        phase("mutants");
        // The selection of what the change can affect, by the test map's
        // diff to this tree: first, and again, of the new tree, past a
        // rebase that brought commits (`sanity`), conflicted or not. None
        // of a change that isn't code goes straight on.
        const changed = lines(git("diff", "--name-only", "origin/main").out);
        const name = checked === "" ? "check" : "sanity";
        // The React gate, beside the check: neither reads what the other
        // writes, and the regen, 35 s a round on the Mac mini after a rebase
        // that moved the compiler, ran after it. Beside `check:fast --all`
        // it took 56 s and the check 262 s for 260 alone: 297 s one after
        // the other, 262 s at once.
        const incoming = reactBase === undefined ? undefined : lines(git("diff", "--name-only", reactBase, base).out);
        const gate = reactGate(changed, incoming);
        if (gate === "reused") console.log("push: the React port as the round before: the rebase's commits touch none of what compiles it");
        if (gate === "run") regen = beside(["bun", "scripts/react-in-rust.ts", "regen"], reactLog);
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
        if (regen !== undefined) {
          const code = await regen.done;
          regen = undefined;
          if (code !== 0) throw new Refused(portRefusal(code, reactLog));
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
    // A check that failed leaves no regen running past the push.
    regen?.stop();
    writeLine();
  }
  process.exit(ok ? 0 : 1);
}

/** Where the React regen beside a round's check writes what it prints. */
const reactLog = "target/push-react.log";

/** A command run beside another, what it prints written to `log`: its
 * exit code once it ends, a way to stop it, and its process group's id.
 * It runs in a group of its own, which `stop` ends whole: killing only the
 * direct child left what it started, the React regen's node, cargo and
 * rust-js, running hours past a failed push. A signal that ends the push
 * ends the group first, as the group no longer shares the terminal's. */
export type Beside = { done: Promise<number>; stop: () => void; group: number };
export function beside(cmd: string[], log: string): Beside {
  mkdirSync("target", { recursive: true });
  const out = openSync(log, "w");
  const child = spawn(cmd[0]!, cmd.slice(1), { stdio: ["ignore", out, out], detached: true });
  closeSync(out);
  const group = child.pid!;
  let ended = false;
  const stop = () => {
    if (ended) return;
    ended = true;
    process.off("exit", stop);
    for (const signal of signals) process.off(signal, ends);
    try {
      process.kill(-group, "SIGTERM");
    } catch {
      // The group has ended already.
    }
  };
  const ends = (signal: NodeJS.Signals) => {
    stop();
    process.kill(process.pid, signal);
  };
  const signals = ["SIGINT", "SIGTERM", "SIGHUP"] as const;
  process.once("exit", stop);
  for (const signal of signals) process.once(signal, ends);
  const done = new Promise<number>((resolve) => {
    child.on("error", () => resolve(1));
    // What it started may outlive it: the group is still ended by `stop`
    // or as the push exits.
    child.on("close", (code) => resolve(code ?? 1));
  });
  return { done, stop, group };
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
 * test/architecture-names.test.ts's WORD_WALKS (#1681 change 3; out of the
 * list while KIN_PARSES, its only one then, was a strict row of none, and
 * back with WORD_WALKS, whose two lowerings at once stopped d8eb74bd1's
 * push). A conflict in that file of anything but such lines still stops
 * the push (`lowerRatchets`).
 * Two lowered at once conflict, each side's lower than the base, and the
 * lower of the two holds of the rebased tree, as each only removes what's
 * counted (ADR 1101, amended). */
export const ratchets = ["test/sig-binders.txt", "test/type-parameter-reads.txt", "test/architecture-names.test.ts"];

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

/** A list of lines a change adds to or takes from, each line standing for
 * itself, its order the list's: a known list, `test/<net>-known.txt`, and
 * the rustc suite's three (ADR 0089). Two changes that each took a line
 * off one conflict on rebase where the lines are neighbours: m1-lisa's
 * #2054 and m6-lily's #2056 off test/metamorphic-known.txt stopped a push
 * (2026-11-18). */
export const lineSet = (file: string) =>
  /^test\/[\w-]+-known\.txt$/.test(file) || ["test/rustc-known-failures.txt", "test/rustc-native-failures.txt", "test/rustc-out-of-scope.txt"].includes(file);

/** `text`, a line set's merged with git's diff3 conflict markers (`git
 * merge-file --diff3`), each conflict resolved as a set: a line either
 * side took off the base's is gone, one either side added is kept, once
 * where both did, in ours' order, theirs' after the line before it in
 * theirs. Undefined where a conflict has no base section, or where each
 * side both added and took off lines, a line each side changed, which a
 * set can't tell from two lines (an id list of test/iterator-matrix-known
 * .txt edited by both), which stays the change's author's to merge. */
export function mergedLineSet(text: string): string | undefined {
  const out: string[] = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i]!.startsWith("<<<<<<< ")) {
      out.push(lines[i]!);
      continue;
    }
    const sides: [string[], string[], string[]] = [[], [], []];
    let side: 0 | 1 | 2 = 0;
    let based = false;
    for (i++; i < lines.length && !lines[i]!.startsWith(">>>>>>> "); i++) {
      const line = lines[i]!;
      if (line.startsWith("||||||| ")) (side = 1), (based = true);
      else if (line === "=======") side = 2;
      else sides[side].push(line);
    }
    if (!based) return undefined;
    const [ours, base, theirs] = sides;
    const count = (of: string[]) => of.reduce((counts, line) => counts.set(line, (counts.get(line) ?? 0) + 1), new Map<string, number>());
    const [o, b, t] = [count(ours), count(base), count(theirs)];
    const edits = (side: Map<string, number>) => {
      const keys = new Set([...side.keys(), ...b.keys()]);
      const at = (line: string) => (side.get(line) ?? 0) - (b.get(line) ?? 0);
      return { adds: [...keys].some((line) => at(line) > 0), takes: [...keys].some((line) => at(line) < 0) };
    };
    const [mine, yours] = [edits(o), edits(t)];
    if (mine.adds && mine.takes && yours.adds && yours.takes) return undefined;
    const left = new Map<string, number>();
    for (const line of new Set([...o.keys(), ...t.keys()])) {
      const [n, m] = [o.get(line) ?? 0, t.get(line) ?? 0];
      left.set(line, Math.min(Math.max(n + m - (b.get(line) ?? 0), 0), Math.max(n, m)));
    }
    const merged: string[] = [];
    for (const line of ours) {
      if (left.get(line)! === 0) continue;
      left.set(line, left.get(line)! - 1);
      merged.push(line);
    }
    let after = -1;
    for (const line of theirs) {
      const found = merged.indexOf(line, after + 1);
      if (found >= 0) after = found;
      else if (left.get(line)! > 0) {
        left.set(line, left.get(line)! - 1);
        merged.splice(++after, 0, line);
      }
    }
    out.push(...merged);
  }
  return out.join("\n");
}

/** TESTS.md, whose top a change adds a `Changed on ..` line to, which
 * almost every two pushes at once both do (a dozen PR merges by hand,
 * 2026-11-18). */
const changesLog = (file: string) => file === "TESTS.md";

/** `text`, TESTS.md merged with git's diff3 conflict markers, each
 * conflict of its `Changed on ..` lines alone resolved by keeping both
 * sides' new lines, newest first: what the replayed change, theirs, added
 * that the base hadn't, then ours, the commit the rebase is onto, whole, a
 * line it rewrote, a count it raised, as it rewrote it. Undefined where a
 * conflict has no base section, or holds a line that isn't a `Changed on`
 * one or blank, the table's or prose, which stays the change's author's to
 * merge. */
export function mergedChanges(text: string): string | undefined {
  const out: string[] = [];
  const lines = text.split("\n");
  const change = (line: string) => line === "" || line.startsWith("Changed on ");
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i]!.startsWith("<<<<<<< ")) {
      out.push(lines[i]!);
      continue;
    }
    const sides: [string[], string[], string[]] = [[], [], []];
    let side: 0 | 1 | 2 = 0;
    let based = false;
    for (i++; i < lines.length && !lines[i]!.startsWith(">>>>>>> "); i++) {
      const line = lines[i]!;
      if (line.startsWith("||||||| ")) (side = 1), (based = true);
      else if (line === "=======") side = 2;
      else sides[side].push(line);
    }
    if (!based) return undefined;
    const [ours, base, theirs] = sides;
    if (![...ours, ...base, ...theirs].every(change)) return undefined;
    const added = theirs.filter((line) => line !== "" && !base.includes(line));
    const kept = ours.filter((line) => line !== "");
    out.push(...[...added, ...kept].flatMap((line, at) => (at === 0 ? [line] : ["", line])));
    // The blank line that parts the last from what follows, where the
    // conflict took it.
    if ((ours.at(-1) ?? "") === "" && (theirs.at(-1) ?? "") === "") out.push("");
  }
  return out.join("\n");
}

/** Whether a rebase's `conflicts` are each a blessed baseline's, which it
 * resolves by making them again, a ratchet's, by the lower count, an
 * ADR's, by both sides' amendments (`appendedBoth`), a line set's, as
 * a set (`mergedLineSet`), or TESTS.md's `Changed on` lines, both sides'
 * (`mergedChanges`); any other, hand-written code's too, stops it. */
export function remakeable(conflicts: string[]): boolean {
  return conflicts.every((file) => file in regenerated || ratchets.includes(file) || decision(file) || lineSet(file) || changesLog(file));
}

/** An ADR, docs/decisions/<n>-<name>.md, which a change amends by
 * appending to its end. */
const decision = (file: string) => /^docs\/decisions\/\d+-[^/]+\.md$/.test(file);

/** An ADR both sides only appended to, each its amendment at the end of
 * `base`, the text they shared: `base`, then `ours`' amendment, the commit
 * the rebase is onto, then `theirs`, the one replayed, each whole, as two
 * pushes amending ADR 1150 at once stopped m3-max's twice (2026-11-18).
 * Undefined where either side changed `base` itself, which stays the
 * change's author's to merge. */
export function appendedBoth(base: string, ours: string, theirs: string): string | undefined {
  if (!ours.startsWith(base) || !theirs.startsWith(base)) return undefined;
  const mine = ours.slice(base.length);
  const yours = theirs.slice(base.length);
  const joined = mine.endsWith("\n") || mine === "" ? mine : `${mine}\n`;
  return base + joined + yours;
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
      if (decision(file)) {
        const merged = appendedBoth(git("show", `:1:${file}`).out, git("show", `:2:${file}`).out, git("show", `:3:${file}`).out);
        if (merged === undefined) {
          git("rebase", "--abort");
          throw new Refused(`the rebase stopped on a conflict in ${file} that isn't two amendments appended to its end, aborted`);
        }
        writeFileSync(file, merged);
        git("add", file);
        continue;
      }
      if (lineSet(file)) {
        const merged = mergedLineSet(spawnSync("git", ["merge-file", "-p", "--diff3", "--object-id", `:2:${file}`, `:1:${file}`, `:3:${file}`], { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 }).stdout);
        if (merged === undefined) {
          git("rebase", "--abort");
          throw new Refused(`the rebase stopped on a conflict in ${file} where each side changed a line, aborted`);
        }
        writeFileSync(file, merged);
        git("add", file);
        continue;
      }
      if (changesLog(file)) {
        const merged = mergedChanges(spawnSync("git", ["merge-file", "-p", "--diff3", "--object-id", `:2:${file}`, `:1:${file}`, `:3:${file}`], { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 }).stdout);
        if (merged === undefined) {
          git("rebase", "--abort");
          throw new Refused(`the rebase stopped on a conflict in ${file} beyond its \`Changed on\` lines, aborted`);
        }
        writeFileSync(file, merged);
        git("add", file);
        continue;
      }
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

if (import.meta.main) await main();
