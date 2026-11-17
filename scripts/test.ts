// The whole suite, `bun run test`: every file in parallel, but the ones
// that run a browser, beside a dev server, Vite's, Next.js's, the
// playground's and the pilot's, or on a page of its own, webapi-events'
// (scripts/shard.ts's `browsers`), after the rest, on their own. Together
// with the corpus's compiles they starve each other, and a page or a Fast
// Refresh waits past its test's timeout: each passes alone, and with only
// each other. `bun run test --shard=2/4` runs a quarter of each, as CI
// splits the suite across its machines (DEVELOPMENT.md). The files only the
// nightly runs (scripts/shard.ts's `nightly`) run with `--nightly`.
// `--selection=<file>`, a selection the selector wrote (scripts/select.ts,
// CI's `plan`), runs only the files and the corpus's cases it selected,
// unless it's `all`.
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { stopGroup } from "../test/child";
import { currentBuild, prune } from "../test/compiles";
import { pruneKeptStores } from "../test/kept";
import { balanced, browsers, caseFiles, casePoolJobs, casePoolWidth, nightly, shardArg, shardTimings, testFileSeconds } from "./shard";
import { machine } from "./shard";

const root = join(import.meta.dir, "..");

// The compiler the suite runs: one given, `RUST_JS_COMPILER`, as it is, as
// CI's test shards give the one its cache keeps of the compiler's sources
// (.github/actions/build-rust-js), else this checkout's, built first, as
// test/support.ts's `buildCompiler` decides of a test.
if (!process.env.RUST_JS_COMPILER) {
  const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (build.exitCode !== 0) process.exit(build.exitCode ?? 1);
  process.env.RUST_JS_COMPILER = join(root, "target", "debug", "rust-js");
}

// What no test used for a week, and past each store's cap, before any runs
// (test/kept.ts): native programs' builds, and the corpus's verified runs;
// and the compiles kept (test/compiles.ts): another build's, and past the
// cap.
pruneKeptStores();
prune({ current: currentBuild() });

// A suite past twice its usual minutes says what it's waiting on, each
// minute: the processes then, and, on Linux, what each of Bun's is blocked
// in. Two runs on CI stopped, a file's tests unfinished, with no rust-js or
// Node left running.
const watchdog = Bun.spawn(["sh", "-c", `sleep 720
while true; do
  echo "--- the suite past 12 minutes: its processes"
  ps -eo pid,ppid,pgid,etime,stat,args --forest 2>/dev/null || ps -eo pid,ppid,pgid,etime,stat,args
  for pid in $(pgrep -x bun 2>/dev/null); do echo "bun $pid: $(cat /proc/$pid/wchan 2>/dev/null)"; done
  sleep 60
done`], { stdout: "inherit", stderr: "inherit" });

const atNight = process.argv.includes("--nightly");
const shard = process.argv.slice(2).filter((arg) => arg.startsWith("--shard="));
// What a selection selected, where it says some: each test file to run,
// and the corpus's cases, a line each, `RUST_JS_CASES=i/n:<file>`.
const selectionArg = process.argv.find((arg) => arg.startsWith("--selection="))?.slice("--selection=".length);
const selection: { all: boolean; files: string[]; cases: string[] } | undefined = selectionArg ? JSON.parse(readFileSync(selectionArg, "utf8")) : undefined;
const selected = selection && !selection.all ? selection : undefined;
const caseList = join(root, "target", "selected-cases.txt");
if (selected) {
  mkdirSync(join(root, "target"), { recursive: true });
  writeFileSync(caseList, selected.cases.map((name) => `${name}\n`).join(""));
  console.log(`the selection: ${selected.files.length} test files, ${selected.cases.length} corpus cases`);
}
// The files of cases, the corpus and the metamorphic sample, in small
// parts, each a process of its own, through one pool beside the rest
// (scripts/shard.ts's `casePoolWidth`): a file of 1,300 cases one after
// another, the corpus was most of its shard's time on CI, 13 to 24
// minutes, the rest done in 4 with 3 of 4 cores idle. Its two parts a
// shard of all at once were each shard's longest leg after, 712-724 s,
// shard 1's alone on two of its four cores for 5.5 minutes after the rest
// ended, and shard 2's rest as long as its metamorphic sample, one file of
// one test at a time, 9.5 minutes (check 43700659375). A shard of `m`
// runs its `1/m` of the parts, the metamorphic sample's first, as each
// part of it is longer.
const { index, count } = shardArg(shard) ?? { index: 1, count: 1 };
const { cores } = machine();
// Of a selection, the corpus's parts of the cases it selected, none where
// it selected none, and the metamorphic sample's where it selected it.
const caseJobs = casePoolJobs(index, count, cores)
  .filter(({ file }) => !selected || (file === caseFiles.corpus ? selected.cases.length > 0 || selected.files.includes(file) : selected.files.includes(file)))
  .map((job) => (selected && job.file === caseFiles.corpus ? { ...job, part: `${job.part}:${caseList}` } : job));
// How long each file took, this checkout's, and the last CI run's shards'
// where `bun scripts/ci.ts test-timings` put them: `bun test` balances the
// shards by them all, and writes this run's to the first.
const kept = join(root, shardTimings);
const timings = ["--timings=target/test-timings.json"];
// Each shard's test files, dealt by what each took, the longest first, to
// the shard with the least so far (`balanced`): `bun test --shard` gave
// check 44022102403's shards 101 and 261 of 362 files, 1,127 and 1,894 of
// their measured seconds, 282 s and 474 s of wall, and the second shard
// ended 4.5 minutes after the first.
const seconds = testFileSeconds([
  join(root, "target", "test-timings.json"),
  ...(existsSync(kept) ? [...new Bun.Glob("**/*.json").scanSync(kept)].sort().map((file) => join(kept, file)) : []),
]);
const dealt = (files: string[]) => (count > 1 ? balanced(files, index, count, seconds) : files);
// And past 25 minutes, it's stopped, and fails: a test worker on CI spun
// for 34 minutes in a `spawnSync` of rustc, which had ended unreaped, past
// its own timeout (2026-10-15), and the job ran on till it was cancelled.
// What the watchdog printed says where. Each run is a process group of its
// own, so its workers are stopped with it; Ctrl-C is given them too.
const deadline = Date.now() + 25 * 60_000;
const running = new Set<number>();
for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.on(signal, () => {
    for (const pid of running) stopGroup(pid, signal);
    process.exit(1);
  });
}
const run = async (args: string[], env: Record<string, string> = {}) => {
  const suite = Bun.spawn([process.execPath, "test", ...args], {
    stdout: "inherit",
    stderr: "inherit",
    detached: true,
    env: { ...process.env, ...env },
  });
  running.add(suite.pid);
  const stop = setTimeout(() => {
    console.log("--- the suite past 25 minutes: stopped, and failed; what it waited on is above");
    stopGroup(suite.pid);
  }, Math.max(0, deadline - Date.now()));
  // A run stopped by a signal has no exit code: it failed.
  const code = (await suite.exited) ?? 1;
  clearTimeout(stop);
  running.delete(suite.pid);
  return code;
};
const ignored = [...Object.values(caseFiles), ...browsers, ...(atNight ? [] : nightly)].map((file) => `--path-ignore-patterns=${file}`);
// The nightly's keeps how long each file and test of each run took, as
// bun's JUnit reporter measures them, one report a run in target/junit/,
// which nightly.yml's `test-times` merges (scripts/testTimes.ts).
// And CI's every run's, which a failed check's `attribute` reads the
// failing tests of (scripts/attribute.ts).
const junit = (name: string) => (atNight || selectionArg ? ["--reporter=junit", `--reporter-outfile=target/junit/${name.replace(/\W+/g, "-")}.xml`] : []);
const of = `${index}-of-${count}`;
let restRunning = true;
// Of a selection, the files it selected but the case files' and the
// browsers', named; none where it selected none.
const everyFile = readdirSync(join(root, "test")).filter((name) => name.endsWith(".test.ts")).map((name) => `test/${name}`);
const rest = dealt(
  (selected?.files ?? everyFile).filter(
    (file) => !(Object.values(caseFiles) as string[]).includes(file) && !browsers.includes(file) && (atNight || !nightly.includes(file)),
  ),
);
const rested = rest.length === 0 ? Promise.resolve(0) : run(["--parallel", ...timings, "--update-timings", ...ignored, ...junit(`suite-${of}`), ...rest]);
// The pool: a part starts as one ends, as many at once as `casePoolWidth`
// says while the rest runs, and more as soon as it's ended. One not
// started by the deadline fails. Each case built as a library too, a
// push's and the nightly's (ADR 0360, #1920).
const cases = await new Promise<number[]>((done) => {
  const codes: number[] = [];
  let next = 0;
  let active = 0;
  const fill = () => {
    while (next < caseJobs.length && active < casePoolWidth(cores, restRunning)) {
      const { file, part } = caseJobs[next++]!;
      if (Date.now() >= deadline) {
        codes.push(1);
        continue;
      }
      active++;
      const name = `${file === caseFiles.corpus ? "corpus" : "metamorphic"}-${part}`;
      void run([file, ...junit(name)], { RUST_JS_CASES: part }).then((code) => {
        codes.push(code);
        active--;
        fill();
      });
    }
    if (active === 0 && next >= caseJobs.length) done(codes);
  };
  void rested.then(() => {
    restRunning = false;
    fill();
  });
  fill();
});
const restCode = await rested;
const pageFiles = dealt(selected ? browsers.filter((file) => selected.files.includes(file)) : browsers);
const pages = pageFiles.length === 0 ? 0 : Date.now() < deadline ? await run(["--parallel", ...timings, ...pageFiles, ...junit(`browsers-${of}`)]) : 1;
watchdog.kill();
process.exit(restCode || cases.find((code) => code !== 0) || pages);
