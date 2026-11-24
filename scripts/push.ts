// The one way to push to `main` (DEVELOPMENT.md, "Pushing"): the branch's
// commits, rebased onto `origin/main`, checked as they'll land, then pushed.
// In order, each phase timed:
//
//   rebase    onto origin/main; a conflict stops it, the rebase aborted, but
//             one each side's change merges mechanically: a blessed baseline,
//             a ratchet, an ADR's two amendments, a line set (`remakeable`),
//             and std/'s generated files, taken as main's and written again
//             by `bun run std` once the round has built (`remadeStd`, #2259)
//   fast      in seconds, what would otherwise refuse it minutes later:
//             push-guard's `undone`, which git's hook runs as it pushes, at
//             the end; and the mutation catalog's test file whole
//             (`FAST_TESTS`), which check:fast runs minutes in; past a
//             rebase, only where its commits touch what they read
//             (`fastAgain`, #2258)
//   build     the compiler
//   std       std/ made again (`bun run std`), refused where it differs from
//             what's committed
//   mutants   each mutation of a file the change touches, or of a list it
//             touches, type-checked (`mutations.ts --typecheck`): a
//             `replace` written against what the change changed, which
//             CI's shards can't build, refused by name
//   check     `check:fast`: the test files and cases the selector says the
//             change can affect (scripts/select.ts, ADR 1101, amended); none
//             with `--skip-check=<why>`, allowed only (a) where the selector
//             is known to over-select, a change to select.ts, the test map
//             or CI config it can't judge, or (b) with Tu's or the lead's
//             say-so for that push, the reason naming which; said with the
//             selection on the push's line, CI's check of the push running
//             every test, a red one fixed before anything else,
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
//             it once the check ends. It runs the round's own link of the
//             compiler built (`portRegen`), which no build in the check
//             takes from under it (#2317)
//   files     `git diff origin/main` holds only the pushed commits' files
//   files     the diff against origin/main its commits' alone; and the
//             tree as the round checked it: a file written since the round
//             settled, though put back, or one made since, is what the
//             check saw and the push wouldn't carry, refused (#2415)
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
// `all (<why>)`, `some (N files, M cases)` or `none`, each round's in turn,
// `check <outcome>; sanity <outcome>` (#2258), and the map's commit and
// how far behind it is (scripts/select.ts); the machine and agent each
// machine's sessions name (`whoRuns`). Written on every exit, a signal's too.
import { spawn, spawnSync } from "node:child_process";
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { isCompilerSource } from "./compiler-crates";
import { ownCompiler } from "./own-compiler";
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
 * last through (d4c76fa8b's first push). And the table of owners' file,
 * about 1.3 s: an answer a change adds without a row or a helper mark
 * (ADR 1175). */
export const FAST_TESTS = ["test/mutations.test.ts", "test/architecture-owners.test.ts"];
export type Phase = (typeof phases)[number];


/** Whether `changed`, the files `git diff origin/main` names, are each one a
 * pushed commit changes, `pushed`'s: none another's, a tree older than
 * `main`'s carries back (scripts/push-guard.ts holds the remote's side). */
/** Whether a round's React gate runs: of a change that touches the port's
 * compile inputs (`touchesPort`), unless an earlier round's passed and the
 * commits the rebase brought since, `incoming`, touch none of them, so the
 * port compiles as it did then: max agent's long pushes ran it, 54 s, at
 * each of two or three rounds (2026-11-17). */
export function reactGate(changed: string[], incoming: string[] | undefined): "run" | "reused" | "none" {
  if (!touchesPort(changed)) return "none";
  return incoming !== undefined && !touchesPort(incoming) ? "reused" : "run";
}

/** Whether a round type-checks the change's mutants again: the first
 * always; past a rebase, where the commits it brought, `incoming`, touch a
 * Rust source, a mutation list or the toolchain, any of which a `replace`
 * may read. Otherwise each `replace` type-checks as it did the round
 * before, its 22–93 s spent again on a moving `main` (lily agent's health
 * check, 2026-11-20). */
/** Whether a round runs the `fast` phase's tests again: the first always;
 * past a rebase, where the commits it brought, `incoming`, touch what they
 * read: the mutation catalog, its runner and each file a mutation names
 * (`named`), any of the compiler's sources, whose answers the table of
 * owners holds, that table, its reader, or the tests themselves. Otherwise
 * each passes as it did the round before, its 17–54 s spent again on a
 * moving `main` (#2258). Push-guard's rule, in the process, runs each round. */
export function fastAgain(incoming: string[] | undefined, named: ReadonlySet<string>): boolean {
  if (incoming === undefined) return true;
  return incoming.some(
    (file) =>
      isCompilerSource(file) ||
      file.startsWith("scripts/mutations") ||
      named.has(file) ||
      FAST_TESTS.includes(file) ||
      file === "docs/architecture.md" ||
      file === "scripts/owners.ts",
  );
}

export function mutantsAgain(incoming: string[] | undefined): boolean {
  if (incoming === undefined) return true;
  return incoming.some((file) => (isCompilerSource(file) && file.endsWith(".rs")) || file.startsWith("scripts/mutations") || /^(?:Cargo\.(?:toml|lock)|rust-toolchain\.toml|build\.rs)$/.test(file));
}

/** Whether a push git refused was refused because `main` moved while it
 * was pushed: the server's refusal of the ref, a lock or a fast-forward
 * (`out`, git's words), and `origin/main` moved off `base` since. Then
 * it's rebased and checked again, the round past a rebase's sanity pass,
 * as a move seen before the push is; any other refusal, a hook's that
 * met a move by chance among them, is refused. #2299's cut's first round
 * passed its whole check, and main moved in the 152 s git's pre-push hook
 * took: "cannot lock ref", the push given up, a whole check again (#2338). */
export function movedWhilePushed(code: number, out: string, base: string, main: string): boolean {
  const raced = /\[remote rejected\][^\n]*cannot lock ref|\[rejected\][^\n]*\((?:fetch first|non-fast-forward)\)/.test(out);
  return code !== 0 && raced && main !== base;
}

/** Why a round past a rebase checks nothing, where the selector's
 * interaction of the change and the commits the rebase brought, `said`,
 * is none: no test both reach, so each test's outcome is the round
 * before's or the incoming pushers' (ADR 1101, amended); `main`'s CI
 * check runs whole after the push. */
export function sanitySkipped(said: string): string | undefined {
  return said === "none" || said.startsWith("none,") ? "incoming reach nothing this change reaches" : undefined;
}

/** Why the push refuses where the React port's regen exits `code`: a
 * crate that doesn't compile, which writes no JS and so no diff, or JS
 * that differs from the pin's. */
export function portRefusal(code: number, log: string): string {
  return code === PORT_DOES_NOT_COMPILE
    ? `the React port doesn't compile at its pin (its run's output in ${log}): its JS isn't there to compare; fix the compiler, or the port and its pin`
    : `the React port differs at its pin (target/react-in-rust.diff; its run's output in ${log}): regenerate the port and move ports/react-in-rust.pin in this push`;
}

/** Of `changed`, the files a rebase changed since this run's script was
 * loaded, those it can't run on as it is: a module this process has
 * loaded already, `loaded`, whose rebased importers, loaded later, would
 * meet its old exports (#2272's `lockHeld`, "Export named 'lockHeld' not
 * found", #2275), and the install's manifests. One not loaded yet loads
 * as rebased, consistent with what it imports. */
export function staleModules(changed: string[], loaded: string[]): string[] {
  const held = new Set(loaded);
  return changed.filter((file) => held.has(file) || file === "package.json" || file === "bun.lock");
}

/** What a round passed, which push run again from the rebased tree
 * carries on from (#2286): the commit it checked, the base it was rebased
 * onto, and where its React gate passed. */
export type Round = { checked: string; base: string; react?: string };

/** The environment push runs again in from the rebased tree: its own, but
 * that this run times it, which the run again does itself, its line the
 * one written (`ownsTimes`); the round it passed, where it did, so the
 * run again checks only what the rebase brought, as a round past a
 * rebase does, not everything again (#2286); and the files the rebase
 * resolved as main's, `remade`, which the run again makes again from the
 * rebased tree, std/'s generated files and blessed baselines: its own
 * rebase finds nothing left to resolve, and std/ stayed main's, a helper
 * the change added gone (#2417). */
export function againEnv(env: NodeJS.ProcessEnv, round?: Round, before?: Before, remade: string[] = []): NodeJS.ProcessEnv {
  const again = { ...env };
  delete again.RUST_JS_TIMED_BY;
  delete again.RUST_JS_TIMED_OTHERS;
  delete again.RUST_JS_PUSH_ROUND;
  delete again.RUST_JS_PUSH_BEFORE;
  delete again.RUST_JS_PUSH_REMADE;
  if (round) again.RUST_JS_PUSH_ROUND = [round.checked, round.base, round.react ?? ""].join(":");
  if (before) again.RUST_JS_PUSH_BEFORE = JSON.stringify(before);
  if (remade.length > 0) again.RUST_JS_PUSH_REMADE = remade.join("\n");
  return again;
}

/** The files the rebase of the push before push run again resolved as
 * main's (`againEnv`), none where it resolved none. */
export function givenRemade(env: NodeJS.ProcessEnv): string[] {
  return (env.RUST_JS_PUSH_REMADE ?? "").split("\n").filter(Boolean);
}

/** What the push before push run again took, its seconds and its phases,
 * which the run again's one line counts as its own: its parent writes
 * none, and 0851a9307's said 740 s of a push that took some 2,100 s. */
export type Before = { seconds: number; phases: [string, number][] };

/** The time push run again was given (`againEnv`), or none. */
export function givenBefore(env: NodeJS.ProcessEnv): Before {
  try {
    const given = JSON.parse(env.RUST_JS_PUSH_BEFORE ?? "") as Before;
    if (Number.isFinite(given.seconds) && Array.isArray(given.phases)) return given;
  } catch {}
  return { seconds: 0, phases: [] };
}

/** The round push run again was given (`againEnv`), or none. */
export function givenRound(env: NodeJS.ProcessEnv): Round | undefined {
  const [checked, base, react] = (env.RUST_JS_PUSH_ROUND ?? "").split(":");
  return checked && base ? { checked, base, ...(react ? { react } : {}) } : undefined;
}

/** Why push run again can't carry on from `round` at `base`, a full
 * check its answer, or `undefined` where it can: the round's base an
 * ancestor of this one, the tree clean, and the branch's commits over
 * this base patch for patch those it checked over its own. A rebase
 * rewrites them, so the checked commit itself is never this one's
 * ancestor; one that resolved a conflict changes a patch, and is checked
 * whole (#2286). `git` runs a git command in the checkout. */
export function roundRefused(round: Round, base: string, git: (...args: string[]) => { code: number; out: string }): string | undefined {
  if (git("merge-base", "--is-ancestor", round.base, base).code !== 0) return `its round's base ${round.base.slice(0, 9)} isn't an ancestor of ${base.slice(0, 9)}`;
  if (git("status", "--porcelain", "--untracked-files=no").out.trim() !== "") return "its tree has a change";
  const patches = (range: string) => {
    const diff = git("log", "--reverse", "-p", "--format=commit %H", range);
    if (diff.code !== 0) return undefined;
    const ids = spawnSync("git", ["patch-id", "--stable"], { input: diff.out, encoding: "utf8" }).stdout;
    return ids.split("\n").filter(Boolean).map((line) => line.split(" ")[0]).join(" ");
  };
  const checked = patches(`${round.base}..${round.checked}`);
  const now = patches(`${base}..HEAD`);
  if (checked === undefined || checked === "" || checked !== now) return `its commits aren't patch for patch those its round checked`;
  return undefined;
}

/** The work tree as a round checks it: when it settled, before the round
 * builds, past the push's own amends, and the untracked files it held then
 * (#2415). */
export type Settled = { at: number; untracked: ReadonlySet<string> };

/** Of the tree as it is, its tracked files' and its untracked files' times
 * of their last write, `tracked` and `untracked`, those that differ from the
 * tree `settled` was: one written since, though put back as it was, which a
 * build or a test may have read meanwhile, and an untracked one made since,
 * a corpus case a test lists. What the check saw isn't what the push
 * carries: an agent's edit beside its push was built by its check (#2415). */
export function changedWhileChecked(
  settled: Settled,
  tracked: ReadonlyMap<string, number>,
  untracked: ReadonlyMap<string, number>,
): string[] {
  const written = (files: ReadonlyMap<string, number>) => [...files].filter(([, at]) => at > settled.at).map(([file]) => file);
  const made = [...untracked.keys()].filter((file) => !settled.untracked.has(file));
  return [...new Set([...written(tracked), ...written(untracked), ...made])].sort();
}

/** The tree's files, `ls-files`' of `which`, each with the time of its last
 * write; one gone since it was listed is left out. */
function filesWritten(which: string[]): Map<string, number> {
  const out = new Map<string, number>();
  for (const file of git("ls-files", "-z", ...which).out.split("\0").filter(Boolean)) {
    try {
      out.set(file, statSync(file).mtimeMs);
    } catch {
      // Gone: a tracked file deleted shows in `git status`, refused there.
    }
  }
  return out;
}

const untrackedArgs = ["--others", "--exclude-standard"];

/** Why a push can't start with `untracked`, the tree's untracked files git
 * doesn't ignore: each would be read by its check, a corpus case a test
 * lists, and not pushed (#2415); `undefined` where there are none. Scratch
 * files go where git ignores them, under `target/`. */
export function untrackedRefusal(untracked: string[]): string | undefined {
  if (untracked.length === 0) return undefined;
  return `untracked ${untracked.join(", ")} would be tested but not pushed; commit it, delete it, or move it under target/`;
}

/** The checkout's files this process has loaded, by their path in it. */
export function loadedModules(): string[] {
  const root = process.cwd();
  return Object.keys(require.cache)
    .filter((path) => path.startsWith(`${root}/`))
    .map((path) => path.slice(root.length + 1));
}

/** A check:fast's selection as the push's line says it: what ran, and the
 * item layer's in shadow, where there was one, with how many tests that
 * failed it left out, which the switch is read from (#2288). */
export function shadowSaid(written: { outcome: string; shadow?: { outcome: string; missed?: string[] } | null }): string {
  if (!written.shadow) return written.outcome;
  const missed = written.shadow.missed === undefined ? "" : `, missed ${written.shadow.missed.length}${written.shadow.missed.length > 0 ? ` (${written.shadow.missed.slice(0, 3).join(", ")})` : ""}`;
  return `${written.outcome} [items: ${written.shadow.outcome}${missed}]`;
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
  // `--skip-check=<why>`: the check phase's tests not run, allowed only (a)
  // where the selector is known to over-select, a change to select.ts, the
  // test map or CI config it can't judge, or (b) with Tu's or the lead's
  // say-so for that push, the reason naming which (DEVELOPMENT.md's
  // "Pushing"); said, with what the selector would have run, on the push's
  // line. Every other phase runs, and CI's check of the push runs every
  // test whole, a red one fixed before anything else (Tu's rule, 2026-11-24).
  const skipArg = process.argv.find((arg) => arg.startsWith("--skip-check"));
  const skipCheck = skipArg?.startsWith("--skip-check=") ? skipArg.slice("--skip-check=".length).trim() : undefined;
  if (skipArg !== undefined && !skipCheck) {
    console.error("push: --skip-check needs its reason, naming which it is: --skip-check=\"(a) <the selector over-selects: select.ts, the test map or CI config>\" or --skip-check=\"(b) <Tu's or the lead's say-so for this push>\" (DEVELOPMENT.md's \"Pushing\")");
    process.exit(2);
  }
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
  // Each round's selection, said on the line in turn, the check's and each
  // sanity pass's (#2258).
  const rounds: string[] = [];
  // The files the mutation catalog names, which the `fast` phase's tests
  // read (`fastAgain`), asked once.
  let named: ReadonlySet<string> | undefined;
  // The machine's log's line, written once, however the run ends: past
  // its end, a failure's, and a signal's, which stops `spawnSync`'s child
  // and then runs here (lily agent's push of b355dc824 wrote none).
  let written = !ownsTimes("push");
  // The push before this one ran again, its time and phases this line's too.
  const before = givenBefore(process.env);
  const writeLine = () => {
    if (written) return;
    written = true;
    devTimes("push", { started: started - before.seconds * 1000, phases: [...before.phases, ...timed], selection, ok, note });
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
    const untracked = untrackedRefusal([...filesWritten(untrackedArgs).keys()]);
    if (untracked !== undefined) throw new Refused(untracked);
    let checked = "";
    // The commit `checked` was rebased onto, whose incoming commits since a
    // later round asks of.
    let checkedBase = "";
    // The base an earlier round's React gate passed at, where one did.
    let reactBase: string | undefined;
    // The commit this script and what it imports were loaded from.
    const loadedAt = git("rev-parse", "HEAD").out.trim();
    // The round push before it passed, where it ran again from the rebased
    // tree, carried on from once it's this branch's own (#2286).
    let given = givenRound(process.env);
    // The round's map fetch, beside it (#2385).
    let fetching: Beside | undefined;
    // The tree as the round checking it settled (#2415).
    let settled: Settled | undefined;
    // What the push before it ran again resolved as main's, made again in
    // its first round (#2417).
    let carried = givenRemade(process.env);
    for (;;) {
      const remade = [...new Set([...rebase(), ...carried])];
      carried = [];
      phase("rebase");
      if (given !== undefined) {
        const refused = roundRefused(given, git("rev-parse", "origin/main").out.trim(), git);
        if (refused === undefined) {
          console.log(`push: carrying on from the round before it ran again: ${given.checked.slice(0, 9)} checked over ${given.base.slice(0, 9)}`);
          ({ checked, base: checkedBase, react: reactBase } = given);
        } else console.log(`push: a full check: the round it was given can't be carried on from: ${refused}`);
        given = undefined;
      }
      // A rebase that changed what this process has loaded: push runs again
      // from the rebased tree, with what it was given, its line its own
      // (#2275). Its run starts at the rebased commit, so it doesn't again.
      const stale = staleModules(lines(git("diff", "--name-only", loadedAt, "HEAD").out), loadedModules());
      if (stale.length > 0) {
        console.log(`push: the rebase changed what push runs, ${stale.slice(0, 3).join(", ")}${stale.length > 3 ? ` and ${stale.length - 3} more` : ""}: running again from the rebased tree`);
        written = true;
        regen?.stop();
        const round = checked === "" ? undefined : { checked, base: checkedBase, react: reactBase };
        const took = { seconds: before.seconds + (performance.now() - started) / 1000, phases: [...before.phases, ...timed] };
        const again = spawnSync(process.execPath, [process.argv[1]!, ...process.argv.slice(2)], { stdio: "inherit", env: againEnv(process.env, round, took, remade) });
        process.exit(again.status ?? 1);
      }
      // CI's map of the nearest commit of `main` it recorded, where it's
      // nearer than the machine's (scripts/testMap.ts's `fetchMap`), fetched
      // beside the round's fast tests, build, std and mutants, and waited
      // for at its selection: a push waited 12 min on it there (#2385).
      // One a round before still at it is ended first: both write its store.
      fetching?.stop();
      fetching = beside(["bun", "scripts/testMap.ts", "fetch"], mapFetchLog);
      const base = git("rev-parse", "origin/main").out.trim();
      // What git's hook refuses as it pushes, and the catalog's own tests,
      // before minutes of building and checking.
      const found = undone(process.cwd(), base, "HEAD");
      if (found.length > 0) throw new Refused(refusal(found));
      const since = checkedBase === "" ? undefined : lines(git("diff", "--name-only", checkedBase, base).out);
      named ??= new Set((await import("./mutations")).mutations.map((m) => m.file));
      if (fastAgain(since, named)) run("fast", ["bun", "test", ...FAST_TESTS]);
      else console.log("push: the fast tests as the round before: the rebase's commits touch nothing they read");
      phase("fast");
      if (checked === "" || git("rev-parse", "HEAD").out.trim() !== checked) {
        for (const file of new Set([...remade, ...(bless ? ["docs/readability.txt"] : [])])) if (file in regenerated) regenerate(file);
        settled = { at: Date.now(), untracked: new Set(filesWritten(untrackedArgs).keys()) };
        run("build", ["cargo", "build", "--quiet"]);
        phase("build");
        // std/'s generated files the rebase resolved as main's, written
        // again by this tree's compiler (#2259).
        if (remade.some(stdGenerated)) remadeStd();
        // std/ made again and compared, the one check of it, CI's lint's too
        // (scripts/std-fresh.ts), which names the fix where it differs.
        run("std", ["bun", "scripts/std-fresh.ts"]);
        phase("std");
        if (mutantsAgain(since)) run("mutants", ["bun", "scripts/mutations.ts", `--typecheck=${base}`]);
        else console.log("push: the mutants as the round before: the rebase's commits touch no Rust source, mutation list or toolchain");
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
        if (gate === "run") {
          const { cmd, env } = portRegen(process.cwd());
          regen = beside(cmd, reactLog, env);
        }
        const code = changed.some((file) => /\.(rs|ts|tsx|js|jsx|mjs|cjs)$/.test(file));
        // The map fetched beside the round, above.
        await fetching!.done;
        const fetched = readFileSync(mapFetchLog, "utf8").trim().split("\n").at(-1) ?? "";
        console.log(`push: ${fetched}`);
        // Past a rebase, the interaction's: the selector's and check:fast's.
        const asked = name === "sanity" ? [`--sanity=${checked}`] : [];
        const selected = sh(["bun", "scripts/select.ts", "--push", ...asked], true);
        if (selected.code !== 0) throw new Refused(`the selector failed:\n${selected.out}`);
        const said = selected.out.split("\n")[0]!.replace(/, by the test map of .*$/, "");
        const skipped = name === "sanity" ? sanitySkipped(said) : undefined;
        if (skipped !== undefined) {
          selection = `none (${skipped})`;
          console.log(`push: the sanity pass checks nothing: ${skipped}`);
        } else if (said === "none" && !code) {
          selection = "none";
          console.log(`push: the selection is none, and nothing changed is code: no check`);
        } else if (skipCheck !== undefined) {
          selection = `skipped (${skipCheck}), where it would run ${said}`;
          console.log(`push: the check skipped, as asked: ${skipCheck}; CI's check of the push runs every test`);
        } else {
          rmSync(selectionFile, { force: true });
          run(name, ["bun", "run", "check:fast", ...asked]);
          selection = existsSync(selectionFile) ? shadowSaid(JSON.parse(readFileSync(selectionFile, "utf8"))) : said;
        }
        // A map not fetched says why on the push's line (b355dc824's said
        // only that there was none).
        if (fetched.startsWith("no map fetched")) selection += ` [${fetched}]`;
        console.log(`push: selected ${selection}`);
        rounds.push(`${name} ${selection}`);
        selection = rounds.join("; ");
        phase(name);
        if (regen !== undefined) {
          const code = await regen.done;
          regen = undefined;
          if (code !== 0) throw new Refused(portRefusal(code, reactLog));
          reactBase = base;
        }
        phase("react");
        checked = git("rev-parse", "HEAD").out.trim();
        checkedBase = base;
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
      // The tree as the round checked it: a change meanwhile, put back or not,
      // was built and tested, and isn't pushed (#2415).
      const touched =
        settled === undefined ? [] : changedWhileChecked(settled, filesWritten([]), filesWritten(untrackedArgs));
      if (touched.length > 0 || lines(git("status", "--porcelain", "--untracked-files=no").out).length > 0) {
        throw new Refused(`the tree changed while it was checked; nothing pushed. Leave the checkout alone during a push, then push again:\n${touched.join("\n")}`);
      }
      phase("files");
      // Something of its own to push: none, its commits lost, is refused,
      // where a push of `origin/main` as it was said it had landed.
      if (lines(git("rev-list", `${base}..HEAD`).out).length === 0) throw new Refused("nothing to push: HEAD is origin/main, the branch's commits aren't on it");
      git("fetch", "-q", "origin");
      if (git("rev-parse", "origin/main").out.trim() !== base) {
        console.log("push: main moved while this was checked; rebasing and checking again");
        continue;
      }
      const pushed = git("push", "-q", "origin", "HEAD:main");
      git("fetch", "-q", "origin");
      if (movedWhilePushed(pushed.code, pushed.out, base, git("rev-parse", "origin/main").out.trim())) {
        console.log("push: main moved while this was pushed; rebasing and checking again");
        continue;
      }
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

/** The React regen a round runs beside its check, of the round's own link
 * of the compiler its build phase built (scripts/own-compiler.ts), which
 * the regen runs as it's given, building none: the check's steps may build
 * the checkout's meanwhile, and a relink took it from under the port's
 * `cargo check` ("could not execute .. No such file", #2317). */
export function portRegen(root: string): { cmd: string[]; env: Record<string, string> } {
  return { cmd: ["bun", "scripts/react-in-rust.ts", "regen"], env: { RUST_JS_COMPILER: ownCompiler(root, "push") } };
}

/** A command run beside another, what it prints written to `log`: its
 * exit code once it ends, a way to stop it, and its process group's id;
 * `env`, what its environment adds to this one's. It runs in a group of its
 * own, which `stop` ends whole: killing only the direct child left what
 * it started, the React regen's node, cargo and rust-js, running hours
 * past a failed push. A signal that ends the push
 * ends the group first, as the group no longer shares the terminal's. */
export type Beside = { done: Promise<number>; stop: () => void; group: number };
/** Where the round's map fetch, beside it, writes what it did (#2385). */
const mapFetchLog = "target/push-map-fetch.log";

export function beside(cmd: string[], log: string, env: Record<string, string> = {}): Beside {
  mkdirSync("target", { recursive: true });
  const out = openSync(log, "w");
  const child = spawn(cmd[0]!, cmd.slice(1), { stdio: ["ignore", out, out], detached: true, env: { ...process.env, ...env } });
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
 * itself, its order the list's: a known list, `test/<net>-known.txt`, an
 * issue's file of one kept as a directory, `test/known/<list>/<issue>.txt`
 * (#2260), and the rustc suite's three (ADR 0089). Two changes that each
 * took a line off one conflict on rebase where the lines are neighbours:
 * lisa agent's #2054 and lily agent's #2056 off the metamorphic net's list
 * stopped a push (2026-11-18). */
export const lineSet = (file: string) =>
  /^test\/[\w-]+-known\.txt$/.test(file) ||
  /^test\/known\/[\w-]+\/[\w-]+\.txt$/.test(file) ||
  ["test/rustc-known-failures.txt", "test/rustc-native-failures.txt", "test/rustc-out-of-scope.txt"].includes(file);

/** `text`, a line set's merged with git's diff3 conflict markers (`git
 * merge-file --diff3`), each conflict resolved as a set: a line either
 * side took off the base's is gone, one either side added is kept, once
 * where both did, in ours' order, theirs' after the line before it in
 * theirs. Undefined where a conflict has no base section, or where each
 * side both added and took off lines, a line each side changed, which a
 * set can't tell from two lines (an id list of an iterator matrix issue's
 * file edited by both), which stays the change's author's to merge. */
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
  return conflicts.every((file) => file in regenerated || stdGenerated(file) || ratchets.includes(file) || decision(file) || lineSet(file) || changesLog(file));
}

/** One of std/'s files `bun run std` writes: its runtime, its compiled
 * modules and their maps, its manifest and its entry and types, which a
 * rebase's conflict resolves by writing them again from the rebased tree
 * (`remadeStd`), never by hand (#2259); not its crate, its README or its
 * package.json, written by hand. */
export const stdGenerated = (file: string) => /^std\/[^/]+\.(?:js|js\.map|d\.ts|json)$/.test(file) && file !== "std/package.json";

/** An ADR, docs/decisions/<n>-<name>.md, which a change amends by
 * appending to its end. */
const decision = (file: string) => /^docs\/decisions\/\d+-[^/]+\.md$/.test(file);

/** An ADR both sides only appended to, each its amendment at the end of
 * `base`, the text they shared: `base`, then `ours`' amendment, the commit
 * the rebase is onto, then `theirs`, the one replayed, each whole, as two
 * pushes amending ADR 1150 at once stopped max agent's twice (2026-11-18).
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
 * that each blessed one stopped the second's rebase (2 of max agent's 5
 * failed pushes of 2026-11-17). Any other conflict aborts it, hand-written
 * code's too. The baselines it resolved so. */
export function rebase(): string[] {
  git("fetch", "-q", "origin");
  const started = git("rebase", "origin/main");
  if (started.code === 0) return [];
  const remade = new Set<string>();
  // A step that stopped with no conflict to resolve, a commit it couldn't
  // make, a working tree's file in the way: never returned as done, which
  // left the rebase stopped and pushed `origin/main` as it was, the
  // branch's commits not in it, said to have landed (2026-11-21).
  const stopped = (said: string) => {
    git("rebase", "--abort");
    return new Refused(`the rebase stopped with no conflict to resolve, aborted:\n${said.trim()}`);
  };
  if (lines(git("diff", "--name-only", "--diff-filter=U").out).length === 0) throw stopped(started.out);
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
    const next = sh(["git", "-c", "core.editor=true", "rebase", "--continue"], true);
    if (next.code === 0) break;
    if (lines(git("diff", "--name-only", "--diff-filter=U").out).length === 0) throw stopped(next.out);
  }
  return [...remade];
}

/** std/'s generated files written again by `bun run std` from the rebased
 * tree, just built, after a rebase resolved them as main's (`stdGenerated`),
 * its diff's files printed, and the top commit amended with them where they
 * changed; refused only where the writing fails (#2259). */
function remadeStd() {
  const made = spawnSync("bun", ["scripts/std.ts"], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (made.status !== 0) throw new Refused(`std/ couldn't be made again after the rebase resolved it as main's:\n${(made.stdout + made.stderr).slice(-2000)}`);
  const changed = lines(git("diff", "--name-only", "--", "std").out);
  if (changed.length === 0) return;
  console.log(`push: std/ made again after the rebase: ${changed.join(", ")}`);
  git("add", "std");
  git("commit", "-q", "--amend", "--no-edit");
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
