// The nightly, here: every check nightly.yml runs, on this machine's cores,
// each in parts where it splits, with a timeout of about twice its usual
// time (AGENTS.md). Its jobs are nightly.yml's and check.yml's, by name
// (`test/ci.test.ts` holds them the same); each part's log is in
// `target/nightly/`.
//
//   bun run nightly                 # all of it
//   bun run nightly stages forms    # these jobs, and what they need first
//   bun run nightly --fuzz-start=N  # the generated programs from seed N,
//                                   # not today's, as tonight's nightly's
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";

import { run } from "../test/child";
import { shown } from "./timing";

const root = join(import.meta.dir, "..");
const cores = availableParallelism();
const minutes = 60_000;

/** A part of a job: its command, what it needs done first, how many cores
 * it keeps busy, and how long it may run. */
type Part = { id: string; cmd: string[]; env?: Record<string, string>; after?: string[]; cores: number; timeout: number };

const compiler = { RUST_JS_COMPILER: join(root, "target/debug/rust-js") };
const parts = (job: string, n: number, make: (i: number) => Omit<Part, "id">): Part[] =>
  Array.from({ length: n }, (_, i) => ({ id: `${job}:${i + 1}/${n}`, ...make(i + 1) }));
// Today's generated programs, as nightly.yml's `seeds` gives them: 600 from
// the day's number times 600.
const fuzzStart = Number(process.argv.find((a) => a.startsWith("--fuzz-start="))?.slice("--fuzz-start=".length)) || Math.floor(Date.now() / 86_400_000) * 600;

/** Each job of the nightly, by nightly.yml's and check.yml's names. */
export const jobs: Record<string, Part[]> = {
  build: [{ id: "build", cmd: ["cargo", "build", "--locked", "--quiet"], cores: 6, timeout: 20 * minutes }],
  lint: [
    { id: "lint:typecheck", cmd: ["bun", "run", "typecheck"], cores: 2, timeout: 5 * minutes },
    { id: "lint:fmt", cmd: ["bun", "run", "fmt:check"], cores: 1, timeout: 5 * minutes },
    { id: "lint:clippy", cmd: ["cargo", "clippy", "--locked", "--quiet", "--", "-D", "warnings"], after: ["build"], cores: 4, timeout: 20 * minutes },
    { id: "lint:cargo-test", cmd: ["cargo", "test", "--locked", "--quiet"], after: ["build"], cores: 4, timeout: 20 * minutes },
  ],
  // The suite with the files only the nightly runs (ADR 1030): itself all
  // the cores, its corpus in parts and its pages after.
  test: [{ id: "test", cmd: ["bun", "scripts/test.ts", "--nightly"], env: compiler, after: ["build"], cores, timeout: 40 * minutes }],
  // Each corpus case after each of tidy's stages (ADR 0659).
  stages: parts("stages", cores, (i) => ({
    cmd: ["bun", "test", "--timeout=600000", "test/tidy-stages.test.ts"],
    env: { ...compiler, TIDY_STAGES: "corpus", RUST_JS_CASES: `${i}/${cores}` },
    after: ["build"],
    cores: 1,
    timeout: 30 * minutes,
  })),
  // Each corpus case with std's iterator structs alone (ADR 0908).
  forms: parts("forms", 4, (i) => ({
    cmd: ["bun", "test", "--timeout=600000", "test/iterator-forms.test.ts"],
    env: { ...compiler, ITERATOR_FORMS: "corpus", RUST_JS_CASES: `${i}/4` },
    after: ["build"],
    cores: 1,
    timeout: 20 * minutes,
  })),
  // The day's 600 generated programs (ADR 0092).
  fuzz: parts("fuzz", 12, (i) => ({
    cmd: ["bun", "test", "--timeout=600000", "test/fuzz.test.ts"],
    env: { ...compiler, FUZZ_START: String(fuzzStart + (i - 1) * 50), FUZZ_SEEDS: "50" },
    after: ["build"],
    cores: 1,
    timeout: 20 * minutes,
  })),
  // rustc's run-pass tests against their lists (ADR 0089), with a release
  // build, as the lists are made with: in shards, then checked as one run.
  rustc: [
    { id: "rustc:release", cmd: ["cargo", "build", "--release", "--locked", "--quiet"], cores: 6, timeout: 40 * minutes },
    // rustc's tests at the toolchain's commit, fetched once, as the setup
    // action does: the shards each fetching them raced for the same folder.
    { id: "rustc:tests", cmd: ["bun", "scripts/rustc-tests.ts"], cores: 1, timeout: 15 * minutes },
    ...parts("rustc", 6, (i) => ({
      cmd: ["bun", "scripts/rustc-suite.ts", "--compiler=target/release/rust-js", `--shard=${i}/6`, `--out=target/nightly/rustc-${i}.json`],
      after: ["rustc:release", "rustc:tests"],
      cores: 2,
      timeout: 40 * minutes,
    })),
    {
      id: "rustc:merge",
      cmd: ["sh", "-c", "bun scripts/rustc-suite.ts --merge target/nightly/rustc-*.json"],
      after: Array.from({ length: 6 }, (_, i) => `rustc:${i + 1}/6`),
      cores: 1,
      timeout: 5 * minutes,
    },
  ],
  examples: [{ id: "examples", cmd: ["bun", "run", "--cwd", "examples/vite-react", "build"], after: ["build"], cores: 2, timeout: 10 * minutes }],
  // The browser's compiler, of rustc's front end at the toolchain's commit,
  // as check.yml's `wasm` job fetches it, and its parity with the native.
  wasm: [
    {
      id: "wasm:rustc",
      cmd: [
        "sh",
        "-c",
        `rustup target add wasm32-wasip1 wasm32-unknown-unknown >/dev/null && if [ ! -d wasm/rustc/compiler ]; then rm -rf wasm/rustc && git init -q wasm/rustc && git -C wasm/rustc remote add origin https://github.com/rust-lang/rust.git && git -C wasm/rustc sparse-checkout set --cone compiler library/proc_macro && git -C wasm/rustc fetch -q --depth=1 --filter=blob:none origin "$(bun scripts/toolchain.ts commit)" && git -C wasm/rustc checkout -q FETCH_HEAD; fi`,
      ],
      cores: 1,
      timeout: 15 * minutes,
    },
    { id: "wasm:build", cmd: ["bun", "run", "wasm"], after: ["wasm:rustc"], cores: 8, timeout: 60 * minutes },
    {
      id: "wasm:parity",
      cmd: ["bun", "test", "test/snapshots.test.ts", "test/playground.test.ts"],
      env: { ...compiler, RUST_JS_REQUIRE_WASM: "1" },
      after: ["wasm:build", "build"],
      cores: 4,
      timeout: 20 * minutes,
    },
  ],
  // Every mutation (ADR 0093), each build's on all the cores (ADR 1031).
  mutations: [{ id: "mutations", cmd: ["bun", "scripts/mutations.ts"], after: ["build"], cores, timeout: 150 * minutes }],
};

async function main() {
  const named = process.argv.slice(2).filter((a) => !a.startsWith("--"));
  const unknown = named.filter((name) => !(name in jobs));
  if (unknown.length > 0) throw new Error(`no job ${unknown.join(", ")}; there are ${Object.keys(jobs).join(", ")}`);
  // The jobs named, and those whose parts theirs need first.
  const all = Object.values(jobs).flat();
  const wanted = new Map<string, Part>();
  const want = (part: Part) => {
    if (wanted.has(part.id)) return;
    wanted.set(part.id, part);
    for (const id of part.after ?? []) want(all.find((p) => p.id === id)!);
  };
  for (const name of named.length > 0 ? named : Object.keys(jobs)) jobs[name].forEach(want);
  const logs = join(root, "target", "nightly");
  rmSync(logs, { recursive: true, force: true });
  mkdirSync(logs, { recursive: true });
  // Each part as soon as what it needs is done and it has the cores.
  const done = new Map<string, { ok: boolean; seconds: number }>();
  const waiting = [...wanted.values()];
  let free = cores;
  const started = Date.now();
  await new Promise<void>((finished) => {
    const next = () => {
      if (waiting.length === 0 && free === cores) return finished();
      for (const part of [...waiting]) {
        const needs = part.after ?? [];
        // One whose need failed fails, unrun.
        if (needs.some((id) => done.get(id)?.ok === false)) {
          waiting.splice(waiting.indexOf(part), 1);
          done.set(part.id, { ok: false, seconds: 0 });
          console.log(`skip ${part.id}: ${needs.filter((id) => !done.get(id)?.ok).join(", ")} failed`);
          continue;
        }
        if (!needs.every((id) => done.has(id))) continue;
        // The cores it keeps busy, or all there are free for one that wants more.
        const takes = Math.min(part.cores, cores);
        if (takes > free) continue;
        waiting.splice(waiting.indexOf(part), 1);
        free -= takes;
        const began = Date.now();
        run(part.cmd, root, part.timeout, part.env).then((exit) => {
          const seconds = (Date.now() - began) / 1000;
          const ok = exit.code === 0 && !exit.timedOut;
          writeFileSync(join(logs, `${part.id.replaceAll("/", "-")}.log`), exit.stdout + exit.stderr);
          done.set(part.id, { ok, seconds });
          console.log(`${ok ? "ok  " : "FAIL"} ${part.id} ${shown(seconds)}${exit.timedOut ? " (timed out)" : ""}`);
          free += takes;
          next();
        });
      }
      if (waiting.length === 0 && free === cores) finished();
    };
    next();
  });
  const failed = [...done].filter(([, d]) => !d.ok).map(([id]) => id);
  console.log(`\n${failed.length === 0 ? "all passed" : `${failed.length} failed: ${failed.join(", ")}`} in ${shown((Date.now() - started) / 1000)}; logs in ${logs}`);
  if (!existsSync(logs) || failed.length > 0) process.exit(1);
}

if (import.meta.main) await main();
