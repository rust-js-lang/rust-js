// The whole suite, `bun run test`: every file in parallel, but the ones
// that run a browser, beside a dev server, Vite's, Next.js's, the
// playground's and the pilot's, or on a page of its own, webapi-events',
// after the rest, on their own. Together
// with the corpus's compiles they starve each other, and a page or a Fast
// Refresh waits past its test's timeout: each passes alone, and with only
// each other. `bun run test --shard=2/4` runs a quarter of each, as CI
// splits the suite across its machines (DEVELOPMENT.md).
import { join } from "node:path";

import { pruneNativeCache } from "../test/native-cache";

// What no test used for a week, before any runs (test/native-cache.ts).
pruneNativeCache(join(import.meta.dir, "../target/native-cache"));

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
const run = (args: string[]) => Bun.spawnSync([process.execPath, "test", "--parallel", ...shard, ...args], { stdout: "inherit", stderr: "inherit" }).exitCode;
const rest = run(["--timings=target/test-timings.json", "--update-timings", ...browsers.map((file) => `--path-ignore-patterns=${file}`)]);
const pages = run(["--timings=target/test-timings.json", ...browsers]);
watchdog.kill();
process.exit(rest || pages);
