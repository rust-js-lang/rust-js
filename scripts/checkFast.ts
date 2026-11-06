// What a change is checked by before it's committed, all at once, in a
// minute or so: lint, the corpus in parts, the suites that check the JS
// bodies are lowered to from their MIR (ADR 0364), the snapshots and the
// diagnostics where the change touches what they read, and the mutations it
// adds or changes. CI runs the whole suite and every mutation of what the
// change touched after each push (DEVELOPMENT.md).
//
//   bun run check:fast                 the change's checks
//   bun run check:fast --all           what a push's check runs, but what `leftToCi` says is CI's
//   bun run check:fast test/x.test.ts  and these files
//
// `--all` is the check before a push: `test/ci.test.ts` holds it to
// check.yml, each command its jobs run one of `jobs`' or one `leftToCi`
// names, and each test file its suite runs one a job here runs (#732).
import { existsSync, linkSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, loadavg } from "node:os";
import { join, resolve } from "node:path";
import { pruneKept } from "../test/kept";
import { runs, touchesIterators } from "./iteratorSlice";
import { browsers, caseParts, nightly } from "./shard";
import { elapsed, loadScale, timed } from "./timing";

const root = join(import.meta.dir, "..");

/** A job of the check, a process of its own: its command, what's added to
 * its environment, the seconds it's given, past which it's stopped and
 * fails, and, of one that runs a command of check.yml's, that command, as
 * `test/ci.test.ts` matches it to check.yml's. */
export type Job = { name: string; cmd: string[]; env: Record<string, string>; timeout: number; ci?: string };

/** What the check is of: `--all`, or the files the change touches, those
 * the branch changed since `main`, the mutations it adds and the files
 * named; and the machine's cores. */
export type Scope = { all: boolean; changed: string[]; branch: string[]; added: string[]; extra: string[]; cores: number };

const job = (name: string, cmd: string[], more: Partial<Job> = {}): Job => ({ name, cmd, env: {}, timeout: 290, ...more });


/** What check.yml runs that `check:fast --all` leaves to CI, by the start
 * of its command, each with why: `test/ci.test.ts` fails on a command of
 * check.yml's that's neither a job's `ci` nor one of these. */
export const leftToCi: { run: string; why: string }[] = [
  { run: "bun scripts/ci.ts", why: "the run's own planning: what its mutations are since, and the last run's timings its shards balance by" },
  { run: "rustup show", why: "a machine's setup: the toolchain, here installed already" },
  { run: "bun install --frozen-lockfile", why: "a machine's setup: the packages, here installed already" },
  { run: "bunx --bun playwright install", why: "a machine's setup: the browser, here installed by `bun run setup`" },
  { run: "cargo build --locked", why: "the compiler, which check:fast builds first, for every job" },
  { run: "BLESS=1", why: "a red run's bless patch: here a `BLESS=1` writes it in place" },
  { run: "bun run wasm", why: "the WASM compiler, a build of rustc's frontend to wasm32 of about half an hour" },
  {
    run: "bun test test/snapshots.test.ts test/playground.test.ts",
    why: "the snapshots and the playground with the WASM compiler (`RUST_JS_REQUIRE_WASM`), which only CI builds; check:fast runs both with the native one",
  },
  {
    run: "uses: ./.github/workflows/rustc-tests.yml",
    why: "rustc's whole suite on 6 machines, and the mutations of all a check's changes: here `bun run test:rustc <dirs>`, and the mutations the change adds",
  },
];

/** The check's jobs, all at once, then `after`'s, on their own: of
 * `--all`, what a push's check runs (check.yml); else the suites the
 * change can touch, as it touches them, `shared` saying whether that's what
 * the snapshots and the diagnostics read. */
export function jobs({ all, changed, branch, added, extra, cores }: Scope): { jobs: Job[]; after: Job[]; shared: boolean } {
  // What the snapshots and the diagnostics read: the compiler and the runtime.
  const shared = all || changed.some((file) => file.startsWith("src/") || file.startsWith("runtime/"));
  const jsx = all || changed.some((file) => /^src\/(lower\/jsx|jsx_syntax)|^react\//.test(file));
  // A change to tidy or preparation is checked after each of tidy's stages
  // and without each of preparation's rewrites, and by the read cases (ADRs
  // 0659, 1087).
  const tidy = all || changed.some((file) => /^src\/(lower\/mir\/tidy|prepare)/.test(file));
  // A change to what a `&mut` is, or to an operation on one, runs the `&mut`
  // matrix's sample against native Rust (ADR 1086).
  const mutRefs = all || changed.some((file) => /^src\/lower\/(representation\/mut_ref|mut_refs|mir\/(assign|entries|places|rvalues|closures)|std_types\/cell)\.rs$/.test(file));
  // A change to the iterators' code, the working tree's or the branch's
  // since `main`, runs the iterator matrix's and the conformance table's
  // slices and the corpus with std's iterator structs, as a push's check
  // does (scripts/iteratorSlice.ts, #731, #766): with `--all` too, which is
  // what the push's check runs, these where its change is the iterators'.
  const iterators = touchesIterators([...changed, ...branch]);

  // Each part of a file of cases, a process of its own (scripts/shard.ts).
  const parts = caseParts(cores);
  const inParts = (name: string, file: string) =>
    Array.from({ length: parts }, (_, i) =>
      // As a push's: of the corpus's, only the cases whose JS as a library
      // differs are built as one (ADR 0360).
      job(`${name} ${i + 1}/${parts}`, ["bun", "test", file], { env: { RUST_JS_CASES: `${i + 1}/${parts}`, RUST_JS_LIBRARIES: "differing" }, ci: "bun run test" }),
    );
  // The suites a change can touch, as it touches them; or, with `--all`,
  // every test file a push's suite runs (scripts/test.ts): the corpus, in
  // its parts above, the rest at once, and after them the browsers'
  // (`browsers`), which starve beside the corpus's compiles, on their own,
  // as the suite runs them: 195 files in about two minutes on the M3 Max,
  // then the browsers' 6 in 53 s. A list of the suites a change to `src/`
  // can break was a second catalog of the test files, and `main` went red
  // three times in a day from files it left out, rest-parameters' and
  // cross-crate-items' (#607); the browsers' files, left to CI, held the
  // pilot's committed JS, red unseen at 336984bff (#673, #732). The
  // nightly's own files are left as the push's suite leaves them; tidy's
  // stages, one of them, run where tidy changes (ADR 0659), and the `&mut`
  // matrix's sample where what a `&mut` is changes (ADR 1086).
  const everyFile = job(
    "every other test file",
    ["bun", "test", "--parallel", ...["test/corpus.test.ts", ...browsers, ...nightly].map((file) => `--path-ignore-patterns=${file}`)],
    { ci: "bun run test" },
  );
  const suites: Job[] = [
    job("architecture, mutation lists", ["bun", "test", "test/architecture.test.ts", "test/architecture-iterators.test.ts", "test/architecture-nones.test.ts", "test/mutations.test.ts"]),
    // What checks the JS bodies are lowered to.
    // Each a file of a concept's (ADR 0572), `test/lowering` all of lowering's.
    job("lowering", ["bun", "test", "test/lowering"]),
    job("compiler", ["bun", "test", "test/compiler.test.ts"]),
    job("bindings", ["bun", "test", "test/bindings.test.ts", "test/bindings-require.test.ts", "test/js-builtins.test.ts", "test/js-errors.test.ts", "test/node-bindings.test.ts", "test/webapi-bindings.test.ts"]),
    ...(jsx ? [job("JSX", ["bun", "test", "test/jsx"])] : []),
    // What pins the JS written, the names it gives and what it refuses; and
    // the guide's examples compiled, std's package, the runtime package's
    // helpers and the readability ratchet, each of which a lowering or a
    // runtime change has turned `main` red with (#212, #222, #253).
    ...(shared
      ? [
          job("guide, packages, readability", ["bun", "test", "test/docs.test.ts", "test/std-package.test.ts", "test/runtime-package.test.ts", "test/readability.test.ts"]),
          job("React's ports", ["bun", "test", "test/port-"]),
          job("snapshots", ["bun", "test", "test/snapshots.test.ts"]),
          job("diagnostics", ["bun", "test", "test/diagnostics.test.ts"]),
        ]
      : []),
  ];
  return {
    jobs: [
      job("fmt", ["bun", "run", "fmt:check"], { ci: "bun run fmt:check" }),
      job("typecheck", ["bun", "run", "typecheck"], { ci: "bun run typecheck" }),
      job("clippy", ["cargo", "clippy", "--locked", "--quiet", "--", "-D", "warnings"], { ci: "cargo clippy --locked -- -D warnings" }),
      // With `--all`, the compiler's unit tests, tidy's passes and
      // decisions, red on `main` unseen after 801d40c55 (#523, #732): its
      // build waits on clippy's lock of `target/`, then takes 13 s here,
      // a cold build of its tests a minute or more. And the Vite example's
      // build, check.yml's `examples`, its committed JS against what
      // rust-js writes, then Vite's: 4 s.
      ...(all
        ? [
            job("cargo test", ["cargo", "test", "--locked", "--quiet"], { timeout: 600, ci: "cargo test --locked" }),
            job("the Vite example's build", ["bun", "run", "--cwd", "examples/vite-react", "build"], { ci: "bun run --cwd examples/vite-react build" }),
          ]
        : []),
      ...inParts("corpus", "test/corpus.test.ts"),
      ...(all ? [everyFile] : suites),
      ...(tidy ? [job("readability passes", ["bun", "test", "test/tidy-stages.test.ts", "test/getter-reads.test.ts"])] : []),
      ...(mutRefs ? [job("the `&mut` matrix's sample", ["bun", "test", "test/mut-ref-matrix.test.ts"])] : []),
      // The slices: 64 and 43 batches in 4 parts, 119-145 s and 63-119 s
      // alone at a load of 14-20; the corpus's every case (2026-11-05).
      ...(iterators
        ? [
            // And the rows of both tables the change names, sized to it
            // (scripts/iteratorRows.ts, #818).
            ...(["matrix", "conformance", "forms", "rows"] as const).map((of) =>
              // The rows a change names: a removal's run whole, some 28,000
              // of the matrix's, about 4 min alone and twice that beside the
              // rest (#856).
              job(of === "rows" ? "the rows the change names" : runs[of][0].name, ["bun", "scripts/iteratorSlice.ts", "run", of, String(parts)], {
                timeout: of === "rows" ? 1200 : 600,
                ci: `bun scripts/iteratorSlice.ts run ${of}`,
              }),
            ),
          ]
        : []),
      ...(added.length > 0 ? [job("mutations the change adds", ["bun", "scripts/mutations.ts", ...new Set(added)])] : []),
      ...extra.map((file) => job(file, ["bun", "test", file])),
    ],
    // 53 s alone on the M3 Max (2026-11-05).
    after: all ? [job("the browsers' test files", ["bun", "test", "--parallel", ...browsers], { timeout: 240, ci: "bun run test" })] : [],
    shared,
  };
}

/** Each of `list` at once, each given the compiler just built: one that
 * builds its own, as each test file and fmt does, links it again, and
 * races the others. Each is said as it ends, against its usual. */
function run(list: Job[], compiler: string) {
  // The highest the load has been since the phase began, as the run's own
  // jobs raise it: a load of 7 as one began was 53 a minute on (2026-11-05).
  let scale = loadScale(loadavg()[0], availableParallelism());
  const watching = setInterval(() => (scale = Math.max(scale, loadScale(loadavg()[0], availableParallelism()))), 5000);
  const ran = Promise.all(
    list.map(async ({ name, cmd, env, timeout: usual }) => {
      const start = Date.now();
      const p = Bun.spawn(cmd, {
        cwd: root,
        env: { ...process.env, RUST_JS_COMPILER: compiler, ...env },
        stdout: "pipe",
        stderr: "pipe",
      });
      let timeout = usual;
      const stop = setInterval(() => {
        timeout = Math.round(usual * scale);
        if (Date.now() - start > timeout * 1000) p.kill();
      }, 1000);
      const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
      const code = await p.exited;
      clearInterval(stop);
      const seconds = (Date.now() - start) / 1000;
      const stopped = code !== 0 && seconds >= timeout ? `, stopped at its ${timeout} s (×${scale.toFixed(1)} for the load)` : "";
      console.log(code === 0 ? `ok   ${timed(`check:fast ${name}`, seconds)}` : `FAIL ${name} (${seconds.toFixed(0)}s${stopped})`);
      return { name, code, log: out + err };
    }),
  );
  return ran.finally(() => clearInterval(watching));
}

if (import.meta.main) {
  // A rebase stopped on a conflict leaves a half-applied tree, which every
  // step would build and test for no reason of the change's own: stopped
  // here, before anything builds (DEVELOPMENT.md's stalls).
  for (const state of ["rebase-merge", "rebase-apply"]) {
    const path = Bun.spawnSync(["git", "rev-parse", "--git-path", state], { cwd: root }).stdout.toString().trim();
    if (path !== "" && existsSync(resolve(root, path))) {
      console.error("a rebase is stopped on a conflict: resolve it first (`git status`), then run check:fast");
      process.exit(1);
    }
  }
  const args = process.argv.slice(2);
  const all = args.includes("--all");
  const extra = args.filter((arg) => !arg.startsWith("--"));

  const git = (gitArgs: string[]) => Bun.spawnSync(["git", ...gitArgs], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const changed = [...git(["diff", "--name-only", "HEAD"]), ...git(["ls-files", "--others", "--exclude-standard"])];
  // And, of what only a push's check runs where it touches it, the
  // iterators', what the branch changed since it left `main`.
  const since = git(["merge-base", "HEAD", "origin/main"])[0];
  const branch = since === undefined ? [] : git(["diff", "--name-only", since, "HEAD"]);

  // The mutations the change adds or changes: each `name` its lists' diff adds.
  const diff = Bun.spawnSync(["git", "diff", "HEAD", "--", "scripts/mutations"], { cwd: root }).stdout.toString();
  const untracked = changed.filter((file) => file.startsWith("scripts/mutations/") && !diff.includes(file));
  const added = [...diff.matchAll(/^\+\s*"?name"?: "([^"]+)"/gm), ...untracked.flatMap((file) => [...readFileSync(join(root, file), "utf8").matchAll(/"?name"?: "([^"]+)"/g)])].map((m) => m[1]);

  // What no test used for a week, before any runs (test/kept.ts), as the
  // whole suite prunes it: only CI runs that, and a Mac's native programs'
  // builds grew 23 GB in two days. Once a day, as it takes some 12 s.
  const pruned = join(root, "target", "kept-pruned");
  if (!existsSync(pruned) || Date.now() - statSync(pruned).mtimeMs > 24 * 60 * 60 * 1000) {
    const pruning = Date.now();
    pruneKept(join(root, "target/native-cache"));
    pruneKept(join(root, "target/corpus-cache"));
    writeFileSync(pruned, "");
    console.log(timed("pruning what tests keep", (Date.now() - pruning) / 1000));
  }

  // Each step's time is kept, and said against its usual (scripts/timing.ts).
  const building = Date.now();
  const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (build.exitCode !== 0) process.exit(1);
  console.log(timed("cargo build", (Date.now() - building) / 1000));

  // The jobs' compiler is the run's own hard link of the one just built.
  // A `cargo build` in the checkout while the run runs, of the agent's next
  // edit, replaces `target/debug/rust-js`. It's gone for part of that, 899
  // of 88,241 looks during one build here, and a test that spawns it then
  // fails at once (ENOENT in 0.1 ms): diagnostics' "a RefCell of a dyn
  // upcast" failed in 0.91 ms (#761). The link is the run's build however
  // the checkout's changes, and goes with the run.
  // Those of runs that ended without exiting, whose process is gone, go too.
  const runs = join(root, "target", "check-fast");
  mkdirSync(runs, { recursive: true });
  for (const pid of readdirSync(runs)) {
    try {
      process.kill(Number(pid), 0);
    } catch {
      rmSync(join(runs, pid), { recursive: true, force: true });
    }
  }
  const own = join(runs, String(process.pid));
  mkdirSync(own, { recursive: true });
  const compiler = join(own, "rust-js");
  linkSync(join(root, "target", "debug", "rust-js"), compiler);
  process.on("exit", () => rmSync(own, { recursive: true, force: true }));

  // And the node bindings the bindings and lowering tests compile against,
  // once, before the jobs that ask for them: built by the first test to ask
  // under the others' load, it outran bun's 5 s test timeout, was killed, and
  // failed every test waiting on it (#227). support.ts's `buildNode`, whose
  // stamp skips it where nothing changed.
  const binding = Date.now();
  const node = Bun.spawnSync(["bun", "-e", "import { buildNode } from './test/support.ts'; buildNode();"], {
    cwd: root,
    env: { ...process.env, RUST_JS_COMPILER: compiler },
    stdout: "inherit",
    stderr: "inherit",
  });
  if (node.exitCode !== 0) process.exit(1);
  console.log(timed("node bindings build", (Date.now() - binding) / 1000));

  const check = jobs({ all, changed, branch, added, extra, cores: availableParallelism() });
  const results = [...(await run(check.jobs, compiler)), ...(await run(check.after, compiler))];
  const failed = results.filter((r) => r.code !== 0);
  for (const { name, log } of failed) console.log(`\n── ${name} ──\n${log.slice(-3000)}`);
  // From its start, its build too.
  const total = elapsed();
  const scope = all ? "check:fast --all" : check.shared ? "check:fast" : "check:fast, the change's";
  console.log(
    `\n${failed.length === 0 ? `all passed: ${timed(scope, total)}` : `${failed.length} failed in ${total.toFixed(0)}s`}${check.shared ? "" : " (JSX, snapshots and diagnostics skipped)"}`,
  );
  process.exit(failed.length === 0 ? 0 : 1);
}
