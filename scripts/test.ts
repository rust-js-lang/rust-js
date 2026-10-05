// The whole suite, `bun run test`: every file in parallel, but the ones
// that run a browser beside a dev server, Vite's, Next.js's, the
// playground's and the pilot's, after the rest, on their own. Together
// with the corpus's compiles they starve each other, and a page or a Fast
// Refresh waits past its test's timeout: each passes alone, and with only
// each other.
const browsers = ["browser", "next", "playground", "pilot", "vite"].map((name) => `test/${name}.test.ts`);
const run = (args: string[]) => Bun.spawnSync([process.execPath, "test", "--parallel", ...args], { stdout: "inherit", stderr: "inherit" }).exitCode;
const rest = run(["--timings=target/test-timings.json", "--update-timings", ...browsers.map((file) => `--path-ignore-patterns=${file}`)]);
const pages = run(["--timings=target/test-timings.json", ...browsers]);
process.exit(rest || pages);
