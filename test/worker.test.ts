// The worker's builds are their own compiles' (#2301, ADR 1182): a corpus
// case's program and library builds, compiled in one rustc session by
// `rust-js --worker`, in either order, write the same files and print the
// same output, byte for byte, as each compiled in a process of its own;
// where the session runs a build apart, the harness compiles it alone,
// which is its own compile by construction, and the row counts it. Every
// row's jobs go to the one worker this process keeps (`test/workers.ts`),
// so each case's builds follow every case's before them in one process,
// as a test process's compiles do (step 2).
//
// A sample of the corpus here, spread over it, every `library-refused` case
// among it; every case in the nightly's, `RUST_JS_WORKER_CASES=all`, in
// parts, `RUST_JS_CASES=i/n`:
//
//   RUST_JS_WORKER_CASES=all RUST_JS_CASES=1/12 bun test test/worker.test.ts

import { expect, test } from "bun:test";
import { chmodSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";

import { spawnedSync, type Exit } from "./child";
import { cases, corpus, directives } from "./corpus-cases";
import { partSetting } from "./part-settings";
import { compileCommand } from "./programs";
import { root } from "./support";
import { measureKept, workerApart, workerJob, workerPid } from "./workers";

const SAMPLE = 24;
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);

/** The cases that run, each with its edition, and whether it's refused as
 * a library. */
const running = cases.flatMap((name) => {
  const text = readFileSync(join(corpus, name), "utf8");
  const want = directives(text);
  if (typeof want === "string" || want.kind === "compile-fail" || want.kind === "ignore-rust-js") return [];
  return [{ name, edition: want.edition, refused: want.libraryRefused !== undefined }];
});
const sampled =
  partSetting("RUST_JS_WORKER_CASES") === "all"
    ? running
    : running.filter((c, i) => c.refused || i % Math.ceil(running.length / SAMPLE) === 0);
const mine = sampled.filter((_, i) => i % parts === part - 1);

// The cache neither keeps nor gives a compile here: each is compiled.
const env = { RUST_JS_COMPILE_CACHE_MB: "0" };
const timeout = 120_000;

/** What a build did: its exit, its output, and every file the case's
 * directory has after it, but the wrapper all builds read. */
type Done = { code: number | null; stdout: string; stderr: string };
function files(dir: string): Record<string, string> {
  const out: Record<string, string> = {};
  const walk = (at: string) => {
    for (const name of readdirSync(at)) {
      const path = join(at, name);
      if (statSync(path).isDirectory()) walk(path);
      else if (name !== "lib.rs") out[relative(dir, path)] = readFileSync(path).toString("base64");
    }
  };
  walk(dir);
  return out;
}
const done = (p: Exit): Done => ({ code: p.code, stdout: p.stdout, stderr: p.stderr });
function emptied(dir: string) {
  for (const name of readdirSync(dir)) if (name !== "lib.rs") rmSync(join(dir, name), { recursive: true, force: true });
}

/** How often a build of a job was run apart, of those the rows compiled,
 * and each that was, but a refused library's program after it: its case,
 * the job's order, the build and why. */
const apart = { builds: 0, of: 0, unexplained: [] as string[] };

/** The worker each row's jobs went to. */
const pids = new Set<number | undefined>();

test.each(mine.map((c) => [c.name, c] as const))("%s: a job's builds are their own compiles', in either order", async (_, c) => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-worker-"));
  try {
    const file = join(corpus, c.name);
    const builds = (["program", "library"] as const).map((library) => compileCommand(file, dir, c.edition, library, []).cmd);
    const alone = builds.map((cmd) => done(spawnedSync(cmd, root, timeout, env)));
    const written = files(dir);
    for (const order of [[0, 1], [1, 0]]) {
      emptied(dir);
      // Kept whatever it holds: the rows hold the worker's reuse, the
      // last row its replacement past its share.
      const answered = await workerJob(order.map((i) => builds[i]), root, timeout, env, Infinity);
      const why = workerApart();
      pids.add(workerPid());
      apart.of += order.length;
      order.forEach((build, k) => {
        const answer = answered[k];
        if (answer === undefined) {
          apart.builds++;
          // Of the sample, only a refused library, given first, prints: the
          // program after it runs apart.
          const printed = c.refused && order[0] === 1 && build === 0 && why[k] === "an earlier build printed";
          if (!printed) apart.unexplained.push(`${c.name}, ${order[0] === 0 ? "program first" : "library first"}, its ${build === 0 ? "program" : "library"}: ${why[k]}`);
          return;
        }
        expect(done(answer)).toEqual(alone[build]);
      });
      // What a build run apart writes is its own compile's: written alone.
      order.forEach((build, k) => {
        if (answered[k] === undefined) spawnedSync(builds[build], root, timeout, env);
      });
      expect(files(dir)).toEqual(written);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("the worker's jobs ran apart only where a build before printed", () => {
  // Each build run apart, but a refused library's program, by its case,
  // its job's order and the worker's reason.
  expect(apart.unexplained).toEqual([]);
  if (apart.of > 0) console.log(`worker: ${apart.builds} of ${apart.of} builds ran apart`);
  // And every row's jobs went to one worker, kept between them.
  if (mine.length > 0) expect([...pids].length).toBe(1);
});

// A case with an impl rust-js never lowers (ADR 0425), a `Future` or an
// `io::Read` of the crate's own: its second build in the session read that
// impl's methods' THIR after `analysis` had stolen it, a panic, and the
// worker died, `it ended (101)`, on both of them in every nightly (#2448).
test("a case's impls never lowered are answered in either order, both builds in one session", async () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-worker-"));
  try {
    for (const name of ["unlowered_impls.rs", "unlowered_helpers.rs"]) {
      const c = running.find((each) => each.name === name)!;
      const builds = (["program", "library"] as const).map((library) => compileCommand(join(corpus, c.name), dir, c.edition, library, []).cmd);
      for (const order of [[0, 1], [1, 0]]) {
        emptied(dir);
        const answered = await workerJob(order.map((i) => builds[i]!), root, timeout, env);
        expect([answered.map((answer) => answer?.code), workerApart()]).toEqual([[0, 0], [undefined, undefined]]);
      }
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// A worker that dies says why: what it said on its own stderr, a panic's
// message, is the reason its builds ran apart (#2448).
test("a worker that ends says what it said as it ended", async () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-worker-"));
  try {
    const dying = join(dir, "dying");
    writeFileSync(dying, "#!/bin/sh\nread job\necho \"thread 'rustc' panicked at here: attempted to read from stolen value\" >&2\nexit 101\n");
    chmodSync(dying, 0o755);
    const answered = await workerJob([[dying, "lib.rs"]], dir, timeout, env);
    expect(answered).toEqual([undefined]);
    expect(workerApart()).toEqual(["it ended (101), saying:\nthread 'rustc' panicked at here: attempted to read from stolen value"]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("each build of a job is given the record of what the session read, as its own compile is", async () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-worker-"));
  try {
    const c = running[0];
    const builds = (["program", "library"] as const).map((library, i) => {
      const { cmd } = compileCommand(join(corpus, c.name), dir, c.edition, library, []);
      return [...cmd, `--emit=dep-info=${join(dir, `${i}.d`)}`];
    });
    const answered = await workerJob(builds, root, timeout, env);
    expect(answered.map((answer) => answer?.code)).toEqual([0, 0]);
    for (const i of [0, 1]) expect(readFileSync(join(dir, `${i}.d`), "utf8")).toContain(c.name);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a kept worker past its share of the machine's memory is replaced, and its next job answered", async () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-worker-"));
  try {
    const c = running[0];
    const builds = (["program", "library"] as const).map((library) => compileCommand(join(corpus, c.name), dir, c.edition, library, []).cmd);
    await workerJob(builds, root, timeout, env);
    const before = workerPid();
    expect(measureKept()).toBeGreaterThan(1);
    // A share of one byte, which it holds more than.
    const answered = await workerJob(builds, root, timeout, env, 1);
    expect(workerPid()).not.toBe(before);
    expect(answered.map((answer) => answer?.code)).toEqual([0, 0]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
