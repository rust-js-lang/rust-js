// A compiler whose output is edited after it's made, the harness's own
// tests' way to give a case wrong JS (test/harness.test.ts): it runs the
// real compiler, then a shell script on each build's output that compiled,
// given as `$1`, and notes each output it edited in a file beside the
// script, so a test can say its edit was made. A compile is given its
// arguments, or, as the corpus harness keeps one, a worker (`--worker`,
// ADR 1182) a job of builds a line at a time: each build compiled alone,
// so the edit reaches the harness by its jobs as by its compiles, as
// test/corpus-mutants.ts' do. A fake that edited a one-shot compile's
// `"$3"` never saw a worker's builds, and passed the case it was to fail
// (#2431).
//
//   bun test/edited-compiler.ts <compiler> <script> <args..>

import { appendFileSync } from "node:fs";

/** What a build printed and how it ended, as a worker answers it. */
type Built = { code: number; stdout: string; stderr: string };

/** The file of the outputs `script` edited, one a line. */
export const editedOf = (script: string) => `${script}.edited`;

function built(compiler: string, script: string, args: string[]): Built {
  const p = Bun.spawnSync([compiler, ...args], { stdout: "pipe", stderr: "pipe", stdin: "inherit" });
  const done = { code: p.exitCode ?? 1, stdout: p.stdout.toString(), stderr: p.stderr.toString() };
  const at = args.indexOf("-o");
  if (done.code !== 0 || at < 0) return done;
  const out = args[at + 1];
  const edit = Bun.spawnSync(["sh", script, out], { stdout: "pipe", stderr: "pipe" });
  if (edit.exitCode !== 0) return { code: edit.exitCode ?? 1, stdout: done.stdout, stderr: `${done.stderr}its edit failed: ${edit.stderr.toString()}` };
  appendFileSync(editedOf(script), `${out}\n`);
  return done;
}

if (import.meta.main) {
  const [compiler, script, ...args] = process.argv.slice(2);
  // Each job answered as it comes: the harness keeps its worker, whose
  // stdin ends only with the test process.
  if (args.length === 1 && args[0] === "--worker") {
    for await (const line of console) {
      if (line.trim() === "") continue;
      const job = JSON.parse(line) as { builds: string[][] };
      await Bun.write(Bun.stdout, `${JSON.stringify({ builds: job.builds.map((build) => built(compiler, script, build)) })}\n`);
    }
    process.exit(0);
  }
  const done = built(compiler, script, args);
  await Bun.write(Bun.stdout, done.stdout);
  await Bun.write(Bun.stderr, done.stderr);
  process.exit(done.code);
}
