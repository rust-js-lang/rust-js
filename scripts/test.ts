// The whole suite, `bun run test`: every file in parallel, but the ones
// that run a browser beside a dev server, Vite's, Next.js's, the
// playground's and the pilot's, after the rest, on their own. Together
// with the corpus's compiles they starve each other, and a page or a Fast
// Refresh waits past its test's timeout: each passes alone, and with only
// each other. `bun run test --shard=2/4` runs a quarter of each, as CI
// splits the suite across its machines (DEVELOPMENT.md).
import { join } from "node:path";

import { pruneNativeCache } from "../test/native-cache";

// What no test used for a week, before any runs (test/native-cache.ts).
pruneNativeCache(join(import.meta.dir, "../target/native-cache"));

const browsers = ["browser", "next", "playground", "pilot", "vite"].map((name) => `test/${name}.test.ts`);
const shard = process.argv.slice(2).filter((arg) => arg.startsWith("--shard="));
const run = (args: string[]) => Bun.spawnSync([process.execPath, "test", "--parallel", ...shard, ...args], { stdout: "inherit", stderr: "inherit" }).exitCode;
const rest = run(["--timings=target/test-timings.json", "--update-timings", ...browsers.map((file) => `--path-ignore-patterns=${file}`)]);
const pages = run(["--timings=target/test-timings.json", ...browsers]);
process.exit(rest || pages);
