// The whole suite, `bun run test`: every file in parallel, but the ones
// that run a browser, beside a dev server, Vite's, Next.js's, the
// playground's and the pilot's, or on a page of its own, webapi-events',
// after the rest, on their own. Together
// with the corpus's compiles they starve each other, and a page or a Fast
// Refresh waits past its test's timeout: each passes alone, and with only
// each other. `bun run test --shard=2/4` runs a quarter of each, as CI
// splits the suite across its machines (DEVELOPMENT.md).
import { existsSync } from "node:fs";
import { join } from "node:path";

import { stopGroup } from "../test/child";
import { pruneKept } from "../test/kept";
import { shardTimings } from "./shard";

const root = join(import.meta.dir, "..");

// What no test used for a week, before any runs (test/kept.ts): native
// programs' builds, and the corpus's verified runs.
pruneKept(join(root, "target/native-cache"));
pruneKept(join(root, "target/corpus-cache"));

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

const browsers = ["browser", "next", "playground", "pilot", "vite", "webapi-events"].map((name) => `test/${name}.test.ts`);
const shard = process.argv.slice(2).filter((arg) => arg.startsWith("--shard="));
// How long each file took, this checkout's, and the last CI run's shards'
// where `bun scripts/ci.ts timings` put them: `bun test` balances the
// shards by them all, and writes this run's to the first.
const kept = join(root, shardTimings);
const timings = [
  "--timings=target/test-timings.json",
  ...(existsSync(kept) ? [...new Bun.Glob("**/*.json").scanSync(kept)].sort().map((file) => `--timings=${join(shardTimings, file)}`) : []),
];
// And past 25 minutes, it's stopped, and fails: a test worker on CI spun
// for 34 minutes in a `spawnSync` of rustc, which had ended unreaped, past
// its own timeout (2026-10-15), and the job ran on till it was cancelled.
// What the watchdog printed says where. Each run is a process group of its
// own, so its workers are stopped with it; Ctrl-C is given them too.
const deadline = Date.now() + 25 * 60_000;
let running: number | undefined;
for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.on(signal, () => {
    if (running !== undefined) stopGroup(running, signal);
    process.exit(1);
  });
}
const run = async (args: string[]) => {
  const suite = Bun.spawn([process.execPath, "test", "--parallel", ...shard, ...args], { stdout: "inherit", stderr: "inherit", detached: true });
  running = suite.pid;
  const stop = setTimeout(() => {
    console.log("--- the suite past 25 minutes: stopped, and failed; what it waited on is above");
    stopGroup(suite.pid);
  }, Math.max(0, deadline - Date.now()));
  // A run stopped by a signal has no exit code: it failed.
  const code = (await suite.exited) ?? 1;
  clearTimeout(stop);
  running = undefined;
  return code;
};
const rest = await run([...timings, "--update-timings", ...browsers.map((file) => `--path-ignore-patterns=${file}`)]);
const pages = Date.now() < deadline ? await run([...timings, ...browsers]) : 1;
watchdog.kill();
process.exit(rest || pages);
