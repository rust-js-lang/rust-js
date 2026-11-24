// What a change is checked by before it's pushed, all at once: lint, the
// test files and corpus cases the selector says the change can affect
// (scripts/select.ts, ADR 1101, amended), each net its paths run, and the
// mutations CI's check judges of it (scripts/judged.ts); everything a push's check runs where the
// selector can't say. `bun run push` runs it. CI's suite runs the same
// selection after each push (DEVELOPMENT.md).
//
//   bun run check:fast                 the change's checks
//   bun run check:fast --all           everything a push's check runs, but what `leftToCi` says is CI's
//   bun run check:fast test/x.test.ts  and these files
//   bun run check:fast --all --profile and how busy the cores were each minute, and each job when
//
// Each run ends saying how busy the machine's cores were while its jobs
// ran; `--profile` says it of each minute, and when each job ran and the
// cores it kept busy, to find where they sat idle on any machine.
//
// `--all` is the check before a push: `test/ci.test.ts` holds it to
// check.yml, each command its jobs run one of `jobs`' or one `leftToCi`
// names, and each test file its suite runs one a job here runs (#732).
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { cpus, loadavg } from "node:os";
import { dirname, join, resolve } from "node:path";
import { compilerCrates, isCompilerSource } from "./compiler-crates";
import { floorBytes } from "../test/bounded";
import { freeBytes } from "../tooling/bounded.js";
import {childWarnings} from "../test/child";
import { changedCases, changePicks, metamorphicMost, runs, touchesAsync, touchesConsumers, touchesCrates, touchesDeclarations, touchesFormatting, touchesIntervals, touchesIterators, touchesMemory, touchesNumbers, touchesStdNet, touchesTwins, type Family } from "./iteratorSlice";
import { OWNER_PAGE, ownerTable, sourceLine } from "./owners";
import { outcome, readOnly, recordSanity, selectChange, selectionFile, shadowMissed, suiteFiles } from "./select";
import { browsers, caseParts, compilerBuilds, machine, memoryBudget, nightly, peakGB, pooledParts } from "./shard";
import { ownCompiler, ownFolder } from "./own-compiler";
import { said, strays } from "./strays";
import { type CheckLogRun, checkLogRun } from "./checkLogs";
import { expected, jobTimesStore, readJobTimes, recordJobTimes } from "./jobTimes";
import { fairShare, holdLease, leaseStore, liveLeases, sharers } from "./leases";
import { elapsed, loadScale, median, shown, timed } from "./timing";
import { devTimes, ownsTimes, slowest } from "./devTimes";

const root = join(import.meta.dir, "..");

/** The push suite's test files, which a selection is of. */
const pushSuite = suiteFiles().length;

/** A step a check runs beside its jobs, a process of its own: its exit, its
 * time from its start to its exit, and what it said, which the check says
 * once its jobs have ended, not among their lines. */
export type Beside = { code: number; seconds: number; stdout: string; stderr: string };

/** `cmd` run in `cwd` beside the jobs, a `Beside` once it exits. */
export function beside(cmd: string[], cwd: string): Promise<Beside> {
  const started = Date.now();
  const p = Bun.spawn(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  return Promise.all([p.exited, new Response(p.stdout).text(), new Response(p.stderr).text()]).then(([code, stdout, stderr]) => ({
    code,
    seconds: (Date.now() - started) / 1000,
    stdout,
    stderr,
  }));
}

/** The check's prunes, `prune` the stores' and `compiles` the kept
 * compiles', said once each has ended, each timed against its usual of
 * the same kind, beside the jobs or before them, and what each said: the
 * compiles' what it kept and dropped, the stores' a warning of the budget.
 * Whether both did their work. */
export async function pruned(prune: Promise<Beside>, compiles: Promise<Beside> | undefined, before: boolean): Promise<boolean> {
  const where = before ? "" : ", beside the jobs";
  const stores = await prune;
  console.log(timed(`pruning what tests keep${where}`, stores.seconds));
  const warned = `${stores.stdout}${stores.stderr}`.trim();
  if (warned) console.log(warned);
  if (compiles === undefined) return stores.code === 0;
  const kept = await compiles;
  if (kept.code !== 0) {
    console.log(`pruning kept compiles failed:\n${`${kept.stdout}${kept.stderr}`.trim()}`);
    return false;
  }
  const { size, builds, removed } = JSON.parse(kept.stdout.trim().split("\n").at(-1)!) as { size: number; builds: number; removed: number };
  const dropped = [builds ? "another build's" : "", removed ? `${removed} past the cap or the disk's floor` : ""].filter(Boolean);
  console.log(`${timed("pruning kept compiles", kept.seconds)}: ${(size / 2 ** 20).toFixed(0)} MB kept${dropped.length ? `, ${dropped.join(" and ")} dropped` : ""}`);
  return stores.code === 0;
}

/** Build, with the prune beside it: the jobs need the compiler, so they
 * wait for its build; not for the prune, which goes on beside them, as
 * what it removes is stale, no check's in a day, and looked at again as
 * it goes (`holdKept`, test/kept.ts). On the M1 it was 25.7 s of every
 * check's start (#2498). But where the disk is below its floor, which a
 * `statfs` says, the jobs would keep nothing they make (test/bounded.ts),
 * so they wait for what it frees; and a build that failed waits for it,
 * so nothing's left running as the check stops. Each timed from its own
 * start to its own exit, as each is awaited apart (#1917). Commands are
 * explicit so the start and each child's duration can be tested with real
 * processes without building the compiler or pruning real stores. */
export async function startup(cwd: string, commands: { build: string[]; prune: string[]; compiles?: () => string[] }, belowFloor: () => boolean) {
  const prune = beside(commands.prune, cwd);
  const building = Date.now();
  const build = Bun.spawn(commands.build, { cwd, stdout: "inherit", stderr: "inherit" });
  const code = await build.exited;
  const built = { code, seconds: (Date.now() - building) / 1000 };
  const waited = code !== 0 || belowFloor();
  if (waited) await prune;
  // The kept compiles' prune, of the build just made, ended before the
  // jobs: test/compiles.test.ts keeps entries of other builds in that
  // store and reads them again, and a prune beside it dropped them under
  // it, an ENOENT failing every agent's check (2026-11-24, the race
  // cc8d65739 opened). Its 0.6-25 s is the price of no false red.
  const compiles = code === 0 && commands.compiles ? await beside(commands.compiles(), cwd) : undefined;
  return { build: built, prune, waited, compiles };
}

/** A job of the check, a process of its own: its command, what's added to
 * its environment, the seconds it's given, past which it's stopped and
 * fails, how many of the machine's cores it's said to keep busy, its
 * `weight`, which the pool runs it by until its runs have measured them
 * (#857), the GB it holds at its peak, `memory`, a test's on each core it
 * keeps busy unless it says (scripts/shard.ts's `peakGB`, #1084), and, of
 * one that runs a command of
 * check.yml's, that command, as `test/ci.test.ts` matches it to check.yml's;
 * of a test file, the `group` of files it's said with; and of a job whose
 * work the change sizes, its `size`, the units of it this run runs, which
 * the pool expects it to take the time of, at its last runs' pace
 * (scripts/jobTimes.ts' `expected`, #1364). */
export type Job = {
  name: string;
  cmd: string[];
  env: Record<string, string>;
  timeout: number;
  weight: number;
  memory: number;
  ci?: string;
  group?: string;
  size?: number;
  /** What it runs where the change picks it, which its usual is of:
   * `ranKey`'s. */
  ran?: string;
};

/** What the check is of: `--all`, or the files the change touches, those
 * the branch changed since `main`, the mutations it adds and the files
 * named; the machine's cores; and, of a check the selector can say, the
 * test files and corpus cases it selects, `picked`, the cases written to
 * `caseList`, a line each, and those built with std's iterator structs, to
 * `formsList` (scripts/select.ts, #877, #876); none, `all`, where it
 * can't; and of `--all`, those with std's iterator structs alone, `forms`,
 * where it can say: the corpus whole runs in its plain build, and in std's
 * of the cases whose compiles ran what the change touches, the nightly and
 * CI running it whole (#986); and of a change to the iterators' code, the
 * rows of their tables it names, as `changePicks` picks them (#1364). */
export type Scope = {
  rows?: number;
  all: boolean;
  changed: string[];
  branch: string[];
  /** The change's mutations CI's check judges not yet judged on its tree,
   * and the commit it left `main` at, where a survivor there too is latent
   * (scripts/judged.ts, #2485). */
  mutations?: { names: string[]; base: string };
  extra: string[];
  cores: number;
  picked?: { files: Set<string>; cases: string[]; caseList: string; forms: string[]; formsList: string };
  forms?: { forms: string[]; formsList: string };
  /** A sanity pass's cap: its picked test files and cases alone, no lint,
   * no nets, no builds (Tu's rule: quick, then push). */
  capped?: boolean;
};

/** The corpus's cases, which the corpus with std's iterator structs runs
 * whole where no list picks some. */
const corpusCases = () => readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs")).length;

/** What a job the selection sizes ran, the key its usual time is of
 * (scripts/timing.ts' `compared`): `all` of a whole run; of a selection, `some`,
 * or `capped` of a sanity pass's cap, and a digest of its files, cases and
 * forms, so a part of 8 cases is never said against a whole run's 200. */
export function ranKey(picked: { files: Set<string>; cases: string[]; forms: string[] } | undefined, capped = false): string {
  if (picked === undefined) return "all";
  return listKey(capped ? "capped" : "some", [picked.files, picked.cases, picked.forms]);
}

/** What a whole run's time is of, the key of its usual: the jobs it ran,
 * each with what it ran (`ran`), in any order. A run that adds the
 * change's rows, the std-iterator corpus, the nets or the crate corpus,
 * 7–8 minutes here, was said against `--all` runs without them, 4½–5, and
 * was always "slower than usual". */
export function totalKey(ran: Job[]): string {
  return listKey("jobs", [ran.map((job) => `${job.name}\t${job.ran ?? ""}`)]);
}

/** `kind` and a digest of `lists`, each in any order: what a job sized by
 * the change ran, its cases or mutations, the key of its usual. */
function listKey(kind: string, lists: Iterable<string>[]): string {
  const listed = lists.map((list) => [...list].sort().join("\n")).join("\n\n");
  return `${kind} ${createHash("sha256").update(listed).digest("hex").slice(0, 12)}`;
}

const job = (name: string, cmd: string[], more: Partial<Job> = {}): Job => ({
  name,
  cmd,
  env: {},
  timeout: 290,
  weight: 1,
  memory: (more.weight ?? 1) * peakGB.test,
  ...more,
});


/** What check.yml runs that `check:fast --all` leaves to CI, by the start
 * of its command, each with why: `test/ci.test.ts` fails on a command of
 * check.yml's that's neither a job's `ci` nor one of these. */
export const leftToCi: { run: string; why: string }[] = [
  { run: "bun scripts/ci.ts", why: "the run's own planning: what its mutations are since, and the last run's timings its shards balance by" },
  { run: "bun scripts/attribute.ts", why: "a failed check's attribution, its failing tests at each commit of its batch, which only CI's run has" },
  { run: "still=()", why: "the same attribution's run of the failing tests at one commit of the batch, by that commit's own harness" },
  { run: "rustup show", why: "a machine's setup: the toolchain, here installed already" },
  { run: "bun install --frozen-lockfile", why: "a machine's setup: the packages, here installed already" },
  { run: "bunx --bun playwright install", why: "a machine's setup: the browser, here installed by `bun run setup`" },
  { run: "cargo build --locked", why: "the compiler, which check:fast builds first, for every job" },
  { run: "bun scripts/std-fresh.ts", why: "std/ made again and compared, which `bun run push`'s std phase runs before check:fast" },
  { run: "BLESS=1", why: "a red run's bless patch: here a `BLESS=1` writes it in place" },
  { run: "bun run wasm", why: "the WASM compiler, a build of rustc's frontend to wasm32 of about half an hour" },
  {
    run: "bun scripts/react-in-rust.ts regen",
    why: "React's port at its pin, fetched and its crates built, about 100 s of a runner: here `bun scripts/react-in-rust.ts regen` by hand, as a port's change is one's own (#1188)",
  },
  {
    run: "bun test test/snapshots.test.ts test/snapshots-serde.test.ts test/playground.test.ts",
    why: "the snapshots and the playground with the WASM compiler (`RUST_JS_REQUIRE_WASM`), which only CI builds; check:fast runs both with the native one",
  },
  {
    run: "bun scripts/mutations.ts --wasm",
    why: "the mutations whose tests read the WASM compiler (`needs: \"wasm\"`), which only CI builds: check:fast's mutations say each is judged in the wasm job (#2346)",
  },
  {
    run: "uses: ./.github/workflows/rustc-tests.yml",
    why: "rustc's whole suite on 6 machines, and the mutations of all a check's changes: here `bun run test:rustc <dirs>`, and the change's own, CI's set of it, judged on its tree (scripts/judged.ts)",
  },
];

/** The paths whose change runs the readability passes' oracles, as
 * readability-passes.yml's `paths` say, the one list of them: a push's CI
 * runs them of these alone, and so does `check:fast`, `--all` too (#998). */
export function readabilityPaths(workflow = readFileSync(join(root, ".github/workflows/readability-passes.yml"), "utf8")): string[] {
  const lines = workflow.split("\n");
  const at = lines.findIndex((line) => /^ {4}paths:$/.test(line));
  if (at === -1) throw new Error("readability-passes.yml has no `on.push.paths`, which says what runs the readability passes");
  const paths: string[] = [];
  for (const line of lines.slice(at + 1)) {
    const item = line.match(/^ {6}- '([^']+)'$/);
    if (!item) break;
    paths.push(item[1]!);
  }
  return paths;
}

/** Whether a change of `files` runs the readability passes. */
export function touchesReadability(files: string[], paths = readabilityPaths()): boolean {
  const globs = paths.map((path) => new Bun.Glob(path));
  return files.some((file) => globs.some((glob) => glob.match(file)));
}

/** Of each family of test/tidy-stages.test.ts' variants, whether a change
 * of `files` runs it: the readability passes' of tidy's and preparation's
 * paths, the intervals' of the facts they keep and what writes past them
 * (#2112 B). */
export const variantFamilies: Record<Family, (files: string[]) => boolean> = {
  readability: (files) => touchesReadability(files),
  intervals: touchesIntervals,
};

/** The check's jobs, through the pool at once: of `--all`, or a change the
 * selector can't say, what a push's check runs (check.yml); else the test
 * files and cases it selected, and the nets and builds its paths run. */
export function jobs({ all, changed, branch, mutations, extra, cores, picked, forms, rows, capped }: Scope): { jobs: Job[] } {
  // What the change touches, the working tree's and what the branch
  // committed since `main`: of the working tree's alone, a change committed
  // before its check, as one on `main` is, touched nothing, and its JSX,
  // snapshots and diagnostics were skipped; 216a72342 left bindings.test.ts's
  // snapshot unblessed, `main` red until 8257a8634 (#1731).
  const touched = [...changed, ...branch];
  // Neither a selection nor `--all`: everything, as the selector says
  // where it can't.
  all ||= picked === undefined;
  // What the jobs the selection sizes run, their usual's key.
  const ran = ranKey(all ? undefined : picked, capped);
  // The compiler's unit tests, of a change to its Rust or its build; the
  // Vite example's build, of a change to any but what only tests and docs
  // read (scripts/select.ts's `readOnly`).
  const rust = all || touched.some((file) => (isCompilerSource(file) && file.endsWith(".rs")) || /^(Cargo\.(toml|lock)|build\.rs|rust-toolchain\.toml|vendor\/)/.test(file) || compilerCrates().some((crate) => file === `${crate.dir}/Cargo.toml`));
  const vite = all || touched.some((file) => (file.includes("/") ? !readOnly.includes(file.split("/")[0]!) : !file.endsWith(".md")));
  // A change to tidy or preparation, or to the oracles, is checked after
  // each of tidy's stages and without each of preparation's rewrites, and by
  // the read cases (ADRs 0659, 1087): of `--all` too only then, as a push's
  // CI runs them, readability-passes.yml's paths, the nightly every case
  // (the user, #998: 28–34 core-seconds of every compiler change's check).
  const tidy = variantFamilies.readability(touched);
  // A change to the facts MIR's intervals keep, or to what writes past
  // them, runs the intervals' variant of the same cases' net: its few
  // cases without their proofs (#2112 B), of `--all` too only then.
  const intervals = variantFamilies.intervals(touched);
  // A change to what a `&mut` is, or to an operation on one, runs the `&mut`
  // matrix's sample against native Rust (ADR 1086).
  const mutRefs = all || touched.some((file) => /^src\/lower\/(representation\/mut_ref|mut_refs|mir\/(assign|entries|places|rvalues|closures)|std_types\/cell)\.rs$/.test(file));
  // A change to what the bounds analyses prove, MIR's intervals, THIR's
  // ranges or a loop's range, runs the bounds matrix: a length tested,
  // something between that may change it, then an index (#1877, #1878).
  const bounds =
    all || touched.some((file) => /^src\/lower\/(analysis\/intervals|body_queries\/ranges|recognition\/loop_ranges)|^test\/bounds-matrix/.test(file));
  // A change to how a place holds a `None`, or to what reads or writes
  // one, runs the `None` matrix's sample against native Rust (#855).
  const nones = all || touched.some((file) => /^src\/lower\/(representation\/(nones|options)|bindings\/nullable|std_types\/option|analysis\/nulled_params)/.test(file));
  // A change to the JS object model's owners, what a struct JS may make
  // is, or the readability passes, or to the net itself, runs the object
  // model's sample: a library's values read through counting proxies
  // (#891).
  const objectModel =
    all ||
    tidy ||
    touched.some((file) =>
      /^(src\/lower\/(classes|untagged|field_keys|bindings|representation|analysis\/fn_pointers)|src\/js\/(stmt_walk|expr_walk)|test\/object-model)/.test(file),
    );
  // A change to the iterators' code, the working tree's or the branch's
  // since `main`, runs the iterator matrix's and the conformance table's
  // slices and the corpus with std's iterator structs, as a push's check
  // does (scripts/iteratorSlice.ts, #731, #766): with `--all` too, which is
  // what the push's check runs, these where its change is the iterators'.
  const iterators = touchesIterators(touched);
  // And a change to the formatting code, the formatting grid's slice, as a
  // push's check runs it (#941).
  const formatting = touchesFormatting(touched);
  // And to the numbers' code, the numbers grid's slice (#940).
  const numbers = touchesNumbers(touched);
  // And to the pointers' code, the memory net's slice (#1172).
  const memory = touchesMemory(touched);
  // And to what a `None` is at the JS boundary, the declarations'
  // TypeScript callers, in one part (#822).
  const declarations = touchesDeclarations(touched);
  // And to how a future awaits and drops, the async table (#942).
  const awaits = touchesAsync(touched);
  // And to the iterators' code or the drops', the consumer net.
  const consumers = touchesConsumers(touched);
  // And to std's collections', slices', strings' or cells' code, the std net (#1171).
  const stdNet = touchesStdNet(touched);
  // And to the iterators' code, the runtime's among it, the twin net: the
  // runtime's helpers against std's own bodies (#1205).
  const twins = touchesTwins(touched);
  // And each corpus case it adds or edits, each of its metamorphic
  // rewrites, plain or `--all` (#1165).
  const cases = changedCases(touched);
  // And the crate corpus, of a change to what compiles it, plain too, as a
  // push's check runs it (#1188); `--all`'s always (#998).
  const crates = all || touchesCrates(touched);

  // What a script that runs its own parts at once runs in, the iterator
  // slices' (scripts/shard.ts).
  const parts = caseParts(cores);
  const half = Math.max(1, Math.floor(cores / 2));
  // The mutations' builds of the compiler at once, as many as the cores and
  // the memory hold (#1084).
  const builds = compilerBuilds(cores);
  // Each part of the corpus, a process of its own, a job of the pool's
  // (scripts/shard.ts's `pooledParts`).
  // Of a selection, the cases it selected, in as many parts as there are
  // cores, or fewer where fewer cases: none where it selected none, nor the
  // file's own tests, which the first part runs.
  const corpusParts = !all && picked
    ? picked.cases.length > 0 || picked.files.has("test/corpus.test.ts")
      ? Math.max(1, Math.min(pooledParts(cores), Math.ceil(picked.cases.length / 8)))
      : 0
    : pooledParts(cores);
  const inParts = (name: string, file: string) =>
    Array.from({ length: corpusParts }, (_, i) =>
      // As a push's: each case built as a library too (ADR 0360, #1920).
      job(`${name} ${i + 1}/${corpusParts}`, ["bun", "test", file], {
        env: { RUST_JS_CASES: `${i + 1}/${corpusParts}${!all && picked ? `:${picked.caseList}` : ""}` },
        ci: "bun run test",
        ran,
      }),
    );
  // The suites a change can touch, as it touches them; or, with `--all`,
  // every test file a push's suite runs (scripts/test.ts): the corpus, in
  // its parts above, and each other file, the browsers' too, a job of its
  // own. A list of the suites a change to `src/`
  // can break was a second catalog of the test files, and `main` went red
  // three times in a day from files it left out, rest-parameters' and
  // cross-crate-items' (#607); the browsers' files, left to CI, held the
  // pilot's committed JS, red unseen at 336984bff (#673, #732). The
  // nightly's own files are left as the push's suite leaves them; tidy's
  // stages, one of them, run where tidy changes (ADR 0659), and the `&mut`
  // matrix's sample where what a `&mut` is changes (ADR 1086).
  // Each file a job of the pool's, the longest first: as one job of half
  // the cores' processes, `bun test --parallel`, they began after the
  // corpus and ran on alone, its last files on one core, the cores 35% busy
  // its last minute (#857, 2026-11-07). The browsers' starved beside the
  // corpus's compiles when every job ran at once, and ran after the rest
  // (ADR 1093); in the pool, whose load stays near the cores, they run
  // beside them.
  const testFiles = readdirSync(join(root, "test"))
    .filter((name) => name.endsWith(".test.ts"))
    .map((name) => `test/${name}`);
  // And the metamorphic net's push sample in parts too, `RUST_JS_CASES=i/n`,
  // as many as the corpus's, its own tests the first's: one `bun test` of
  // it, 204 to 238 s on about one core, ran on alone at a check's end, past
  // a consumer's 720 s (#1749).
  // Of a selection, the files it selected.
  const selected = (file: string) => all || picked === undefined || picked.files.has(file);
  const metamorphic = runs.metamorphic[0].file;
  const metamorphicParts = selected(metamorphic) ? pooledParts(cores) : 0;
  const everyFile = [
    ...testFiles
      .filter((file) => file !== "test/corpus.test.ts" && file !== metamorphic && !nightly.includes(file) && selected(file))
      .map((file) =>
        job(file, ["bun", "test", file], { group: browsers.includes(file) ? "the browsers' test files" : "every other test file", ci: "bun run test", ran }),
      ),
    ...Array.from({ length: metamorphicParts }, (_, i) =>
      job(`the metamorphic sample ${i + 1}/${metamorphicParts}`, ["bun", "test", metamorphic], {
        env: { RUST_JS_CASES: `${i + 1}/${metamorphicParts}` },
        ci: "bun run test",
      }),
    ),
  ];
  if (capped) return { jobs: [...inParts("corpus", "test/corpus.test.ts"), ...everyFile, ...extra.map((file) => job(file, ["bun", "test", file]))] };
  return {
    jobs: [
      job("fmt", ["bun", "run", "fmt:check"], { ci: "bun run fmt:check" }),
      job("typecheck", ["bun", "run", "typecheck"], { ci: "bun run typecheck" }),
      job("clippy", ["cargo", "clippy", "--locked", "--quiet", "--all-targets", "--", "-D", "warnings"], { weight: 2, memory: peakGB.compilerCheck, ci: "cargo clippy --locked --all-targets -- -D warnings" }),
      // With `--all`, the compiler's unit tests, tidy's passes and
      // decisions, red on `main` unseen after 801d40c55 (#523, #732): its
      // build waits on clippy's lock of `target/`, then takes 13 s here,
      // a cold build of its tests a minute or more. And the Vite example's
      // build, check.yml's `lint` job's last steps, its committed JS against what
      // rust-js writes, then Vite's: 4 s.
      ...(rust
        ? [
            job("cargo test", ["cargo", "test", "--locked", "--quiet"], { timeout: 600, weight: 2, memory: peakGB.compilerCheck, ci: "cargo test --locked" }),
            // The crates with no rustc in them, built for WASI, as the
            // playground's compiler links them (ADR 1181): the full WASI
            // build is CI's `wasm` job's, minutes of rustc's front end;
            // this is seconds.
            job("the crates for WASI", ["cargo", "check", "--locked", "--quiet", "--target", "wasm32-wasip1", "--workspace", "--exclude", "rust-js", "--exclude", "rust-js-lower", "--exclude", "rust-js-mir", "--exclude", "rust-js-jsx"], {
              memory: peakGB.compilerCheck,
              ci: "cargo check --locked --target wasm32-wasip1 --workspace --exclude rust-js --exclude rust-js-lower --exclude rust-js-mir --exclude rust-js-jsx",
            }),
          ]
        : []),
      ...(vite ? [job("the Vite example's build", ["bun", "run", "--cwd", "examples/vite-react", "build"], { ci: "bun run --cwd examples/vite-react build" })] : []),
      // The crates of crates.io's corpus against docs/crate-corpus.md,
      // half the cores' at once: 22–33 s, which only the Nightly ran, so a
      // refusal that stopped itertools went unseen (#857, #1188). Its top
      // crates by downloads sampled, one in 8 by the commit (#1174).
      ...(crates
        ? [job("the crate corpus", ["bun", "scripts/crate-corpus.ts", "--check", "--sample"], { weight: half, memory: half * peakGB.crate, ci: "bun scripts/crate-corpus.ts --check --sample" })]
        : []),
      ...inParts("corpus", "test/corpus.test.ts"),
      ...everyFile,
      ...(tidy ? [job("readability passes", ["bun", "test", "test/tidy-stages.test.ts", "test/getter-reads.test.ts"], { weight: 2, env: { TIDY_VARIANTS: "readability" } })] : []),
      ...(intervals ? [job(runs.intervals[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "intervals", "1"], { ci: "bun scripts/iteratorSlice.ts run intervals" })] : []),
      ...(mutRefs ? [job("the `&mut` matrix's sample", ["bun", "test", "test/mut-ref-matrix.test.ts"])] : []),
      // In a part a core, `RUST_JS_CASES=i/n`: one `bun test` of its 321
      // programs, one after another, took 270 s.
      ...(bounds
        ? Array.from({ length: pooledParts(cores) }, (_, i) =>
            job(`the bounds matrix ${i + 1}/${pooledParts(cores)}`, ["bun", "test", "test/bounds-matrix.test.ts"], {
              env: { RUST_JS_CASES: `${i + 1}/${pooledParts(cores)}` },
            }),
          )
        : []),
      ...(nones ? [job("the `None` matrix's sample", ["bun", "test", "test/none-matrix.test.ts"])] : []),
      ...(objectModel ? [job("the object model's sample", ["bun", "test", "test/object-model.test.ts"])] : []),
      // The slices: 64 and 43 batches in 4 parts, 119-145 s and 63-119 s
      // alone at a load of 14-20; the corpus's every case (2026-11-05).
      ...(iterators
        ? [
            // And the rows of both tables the change names, sized to it
            // (scripts/iteratorRows.ts, #818).
            ...(["matrix", "conformance", "copies", "paths", "forms", "rows"] as const)
              // Of a change the test map can say, the corpus with std's
              // iterator structs runs the cases it picked, none where it
              // picked none, nor the file's own tests (#876).
              .filter(
                (of) =>
                  of !== "forms" ||
                  (!all && picked ? picked.forms.length > 0 || picked.files.has("test/iterator-forms.test.ts") : !forms || forms.forms.length > 0),
              )
              .map((of) =>
                // The rows a change names: a removal's run whole, some 28,000
                // of the matrix's, about 4 min alone and twice that beside the
                // rest (#856).
                // Each `parts` processes at once, throughout (lily agent, #857).
                job(
                  of === "rows" ? "the rows the change names" : runs[of][0].name,
                  ["bun", "scripts/iteratorSlice.ts", "run", of, String(parts), ...(of === "forms" && ((!all && picked) || forms) ? [((!all && picked) || forms)!.formsList] : [])],
                  {
                    timeout: of === "rows" ? 1200 : 600,
                    weight: parts,
                    ci: `bun scripts/iteratorSlice.ts run ${of}`,
                    // Sized by the change: the rows it names, and the corpus
                    // cases with std's iterator structs it picks, or all of
                    // them (#1364).
                    ...(of === "rows" && rows !== undefined ? { size: rows } : {}),
                    ...(of === "forms" ? { size: ((!all && picked) || forms)?.forms.length ?? corpusCases() } : {}),
                    // Its usual of runs of the same rows or cases.
                    ...(of === "rows" && rows !== undefined ? { ran: `${rows} rows` } : {}),
                    ...(of === "forms" ? { ran: !all && picked ? ran : forms ? listKey("forms", [forms.forms]) : "all" } : {}),
                  },
                ),
              ),
          ]
        : []),
      ...(formatting
        ? [
            job(runs.format[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "format", String(parts)], {
              timeout: 600,
              weight: parts,
              ci: "bun scripts/iteratorSlice.ts run format",
            }),
          ]
        : []),
      ...(numbers
        ? [
            job(runs.numbers[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "numbers", String(parts)], {
              timeout: 600,
              weight: parts,
              ci: "bun scripts/iteratorSlice.ts run numbers",
            }),
          ]
        : []),
      ...(memory
        ? [
            job(runs.memory[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "memory", String(parts)], {
              timeout: 600,
              weight: parts,
              ci: "bun scripts/iteratorSlice.ts run memory",
            }),
          ]
        : []),
      ...(declarations
        ? [
            job(runs.declarations[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "declarations", "1"], {
              timeout: 300,
              ci: "bun scripts/iteratorSlice.ts run declarations 1",
            }),
          ]
        : []),
      ...(awaits
        ? [
            job(runs.async[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "async", "2"], {
              timeout: 300,
              weight: 2,
              ci: "bun scripts/iteratorSlice.ts run async",
            }),
          ]
        : []),
      ...(consumers
        ? [
            job(runs.consumers[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "consumers", String(parts)], {
              timeout: 300,
              weight: parts,
              ci: "bun scripts/iteratorSlice.ts run consumers",
            }),
          ]
        : []),
      ...(stdNet
        ? [
            job(runs.std[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "std", String(parts)], {
              timeout: 300,
              weight: parts,
              ci: "bun scripts/iteratorSlice.ts run std",
            }),
          ]
        : []),
      ...(twins
        ? [
            job("the twin net", ["bun", "scripts/iteratorSlice.ts", "run", "twins", String(parts)], {
              timeout: 300,
              weight: parts,
              ci: "bun scripts/iteratorSlice.ts run twins",
            }),
          ]
        : []),
      ...(cases.length > 0
        ? [
            job(runs.metamorphic[0].name, ["bun", "scripts/iteratorSlice.ts", "run", "metamorphic", String(Math.min(parts, cases.length))], {
              timeout: 600,
              weight: Math.min(parts, cases.length),
              ci: "bun scripts/iteratorSlice.ts run metamorphic",
              // Sized by the cases whose rewrites it runs (#1364).
              size: Math.min(cases.length, metamorphicMost),
              ran: listKey("cases", [cases]),
            }),
          ]
        : []),
      ...(mutations !== undefined && mutations.names.length > 0
        ? [
            job("the change's mutations", ["bun", "scripts/mutations.ts", `--jobs=${builds}`, `--base=${mutations.base}`, ...new Set(mutations.names)], {
              weight: builds,
              memory: builds * peakGB.compilerBuild,
              // Sized by the mutations it runs (#1364).
              size: new Set(mutations.names).size,
              ran: listKey("mutations", [mutations.names]),
            }),
          ]
        : []),
      ...extra.filter((file) => !everyFile.some((each) => each.name === file)).map((file) => job(file, ["bun", "test", file])),
    ],
  };
}

/** A group's time, its files' own seconds summed: what it's said against
 * its usual by, as a slower file shows in it, not the pool's queue. */
export function groupSeconds(done: { start: number; end: number }[]): number {
  return done.reduce((sum, r) => sum + r.end - r.start, 0);
}

/** What else a group's line says: how many files, the longest of them, and
 * the span from the first's start to the last's end. */
export function groupSaid(done: { job: { name: string }; start: number; end: number }[], span: number): string {
  const longest = done.reduce((a, r) => (r.end - r.start > a.end - a.start ? r : a));
  return `(${done.length} files, summed; the longest ${longest.job.name} ${shown(longest.end - longest.start)}, all ${shown(span)} from the first's start)`;
}

/** When the jobs began, and the machine's busy share every two seconds
 * since. */
export type Clock = { began: number; samples: { at: number; busy: number }[] };

/** A job's run: how it ended, its log, when it started and ended, in
 * seconds from the pool's start, and the CPU seconds it and the children
 * it waited on used, which `--profile` divides by its time for the cores
 * it kept busy. */
type Ran = { job: Job; code: number; log: string; start: number; end: number; cpu: number; cut?: true };

/** The share of the machine's cores busy since the last call, from
 * `os.cpus()`'s counters, which agree with `top`'s on macOS and Linux. */
function busySince(): () => number {
  const read = () =>
    cpus().reduce(
      (sum, { times: t }) => ({ busy: sum.busy + t.user + t.nice + t.sys + t.irq, all: sum.all + t.user + t.nice + t.sys + t.irq + t.idle }),
      { busy: 0, all: 0 },
    );
  let last = read();
  return () => {
    const now = read();
    const share = (now.busy - last.busy) / Math.max(1, now.all - last.all);
    last = now;
    return share;
  };
}

/** `list`'s jobs through a pool of the machine's `cores` (#857): each
 * started once the cores it keeps busy are free, as its last runs measured
 * them, or its `weight` before it has run, and the GB it holds at its peak,
 * its `memory`, are free of the machine's `memoryBudget` (#1084), the
 * longest first, so the check's own jobs keep the load near the cores and
 * the memory under the machine's. All at once, they drove it to two to five times them, each ran slower and tests
 * timed out that pass alone. One heavier than the cores, or the memory,
 * runs alone. Each
 * is given the compiler just built, one that builds its own, as each test
 * file and fmt does, linking it again; and the scale its timeouts are at,
 * `RUST_JS_LOAD_SCALE`, twice the cores' load at least, as a pool's job
 * shares them. Each is said as it ends, against its usual; a `group`'s
 * jobs, each test file, once the last of them ends, as one, from the first's
 * start, `clock.samples` the machine's busy share every two seconds.
 * Given `failFast`, `--retire`'s of a mutant (#2256): at the first job that
 * fails, none more is started and those running are stopped, each said
 * `cut`, which says nothing of the compiler. */
export function run(
  list: Job[],
  compiler: string,
  cores: number,
  { began, samples }: Clock,
  logs: CheckLogRun = checkLogRun(),
  times = jobTimesStore,
  leases = leaseStore,
  failFast = false,
): Promise<Ran[]> {
  const busy = busySince();
  const sampling = setInterval(() => samples.push({ at: (Date.now() - began) / 1000, busy: busy() }), 2000);
  // The highest the load has been since the phase began, and the load
  // every five seconds.
  let scale = loadScale(loadavg()[0], cores);
  const loads: { at: number; load: number }[] = [];
  const watching = setInterval(() => {
    loads.push({ at: Date.now(), load: loadavg()[0] });
    scale = Math.max(scale, loadScale(loads.at(-1)!.load, cores));
  }, 5000);
  // Each job's last runs on this machine, in any of its checkouts, read
  // once (scripts/jobTimes.ts, #1364): a worktree's own, which a new one
  // hasn't, started every job in its list's order, the iterators' slices
  // last, alone at the end.
  const machine = readJobTimes(times);
  // What it usually takes, and what it may take, by which the pool starts
  // it: the longest of its last runs, so that one whose usual is cached,
  // cargo test's under a second, but whose change rebuilds it, 36 s, isn't
  // started last and run alone at the end; of a job the change sizes, its
  // size at its runs' pace, so the rows a change names aren't started last
  // because the last change named none (#1364).
  const usual = (job: Job) => expected(machine[job.name], job.size).usual;
  const longest = (job: Job) => expected(machine[job.name], job.size).longest;
  // The cores it keeps busy, as its last runs measured them, or, of a job
  // not yet run, as it says: the corpus's parts, said to keep two, kept
  // one, and the browsers' files, said to keep all twelve, kept 3.6, so
  // the cores sat a third idle (2026-11-07).
  const weight = ({ name, weight: said }: Job) => {
    const measured = machine[name]?.cores ?? [];
    return Math.min(share(cores), measured.length > 0 ? Math.max(0.25, median(measured)) : said);
  };
  const queue = [...list].sort((a, b) => longest(b) - longest(a));
  const groups = new Map<string, Ran[]>();
  for (const { group } of list) if (group !== undefined) groups.set(group, []);
  // The jobs running, by their count: what they hold, a sum of measured
  // shares, `0.1 + 0.2 - 0.1 - 0.2`, is `2.8e-17` once they've all ended,
  // not 0, so one that holds every core waited on it with none running to
  // wake it, and the check sat idle until its outer limit (2026-11-14).
  let running = 0;
  let held = 0;
  let heldMemory = 0;
  const budget = memoryBudget();
  let waiting: (() => void)[] = [];
  // Its share of them where concurrent runs hold leases, the cores and the
  // memory divided by the others live and itself (scripts/leases.ts), asked
  // at most each second as a job waits for room.
  let shares = { count: 1, at: 0 };
  const share = (whole: number) => {
    if (Date.now() - shares.at > 1000) shares = { count: sharers(process.env, leases), at: Date.now() };
    return fairShare(whole, shares.count);
  };
  // The next job to start, of those still queued: the first, longest
  // first, that fits the room left, so one that can't yet doesn't hold back
  // those after it that can, as it did (#1762); and of none that fits where
  // none runs, the first, alone, as one heavier than the machine runs.
  const pending = [...queue];
  // Of a fail-fast run, those running, stopped at the first that fails
  // (`failFast`), and whether it has.
  const live = new Set<ReturnType<typeof Bun.spawn>>();
  let cutting = false;
  const next = async (): Promise<Job | undefined> => {
    for (;;) {
      if (pending.length === 0) return undefined;
      const at = pending.findIndex((job) => running === 0 || (held + weight(job) <= share(cores) && heldMemory + job.memory <= share(budget)));
      if (at >= 0) return pending.splice(at, 1)[0];
      await new Promise<void>((wake) => waiting.push(wake));
    }
  };
  const freed = (weight: number, memory: number) => {
    running--;
    held -= weight;
    heldMemory -= memory;
    const woken = waiting;
    waiting = [];
    for (const wake of woken) wake();
  };
  // The cores a run of `cpu` CPU seconds kept busy over its `seconds`,
  // from `from` to `to`: as many as it got, and, where every core was
  // busy then, as many more as the load past the cores kept it waiting for,
  // so a job squeezed once isn't given less room the next time, and
  // squeezed again. Only then: macOS's load counts more than it runs, 16
  // on 12 cores 86% busy (2026-11-07).
  const kept = (cpu: number, from: number, to: number, seconds = (to - from) / 1000) => {
    const mean = (values: number[], otherwise: number) => (values.length > 0 ? values.reduce((a, b) => a + b, 0) / values.length : otherwise);
    const share = mean(samples.filter((s) => s.at >= (from - began) / 1000 && s.at <= (to - began) / 1000).map((s) => s.busy), 0);
    const load = mean(loads.filter((l) => l.at >= from && l.at <= to).map((l) => l.load), loadavg()[0]);
    return (cpu / Math.max(0.1, seconds)) * (share >= 0.95 ? Math.max(1, load / cores) : 1);
  };
  const ran: Promise<Ran>[] = [];
  const started = (async () => {
    while (pending.length > 0) {
      const job = await next();
      if (job === undefined) break;
      const { name, cmd, env, timeout: given, group } = job;
      const holds = weight(job);
      running++;
      held += holds;
      heldMemory += job.memory;
      ran.push(
        (async () => {
          const start = Date.now();
          const expected = Math.max(2, scale);
          const p = Bun.spawn(cmd, {
            cwd: root,
            env: { ...process.env, RUST_JS_COMPILER: compiler, RUST_JS_LOAD_SCALE: String(expected), RUST_JS_LEASED: process.env.RUST_JS_LEASED || String(process.pid), ...env },
            stdout: "pipe",
            stderr: "pipe",
          });
          live.add(p);
          // Its seconds, as the load scales them, and twice its own usual at
          // least: under the pool the load stays near the cores, and the
          // rows a change names, 50 min on 8 cores beside the rest, were
          // stopped at 1,200 s × 2.3 where all at once ×3 had let them be.
          const own = usual(job);
          let timeout = given;
          const stop = setInterval(() => {
            timeout = Math.round(Math.max(given * scale, Number.isFinite(own) ? 2 * own : 0));
            if (Date.now() - start > timeout * 1000) p.kill();
          }, 1000);
          const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
          const code = await p.exited;
          live.delete(p);
          clearInterval(stop);
          freed(holds, job.memory);
          // One a fail-fast run stopped: nothing of the compiler.
          if (cutting && code !== 0) return { job, code, log: out + err, start: (start - began) / 1000, end: (Date.now() - began) / 1000, cpu: 0, cut: true } as Ran;
          const seconds = (Date.now() - start) / 1000;
          const stopped = code !== 0 && seconds >= timeout ? `, stopped at its ${timeout} s (×${scale.toFixed(1)} for the load)` : "";
          const cpu = Number(p.resourceUsage()?.cpuTime.total ?? 0) / 1e6;
          const result: Ran = { job, code, log: out + err, start: (start - began) / 1000, end: (Date.now() - began) / 1000, cpu };
          for (const warning of childWarnings(result.log)) console.warn(name + ": " + warning);
          // Its whole output kept as it fails: a check an outer limit
          // stops keeps it (#1706).
          if (code !== 0) console.log(`FAIL ${name} (${seconds.toFixed(0)}s${stopped}), its whole output: ${logs.keep(name, result.log)}`);
          if (code !== 0 && failFast && !cutting) {
            cutting = true;
            console.log(`stopped at its first failure: ${pending.length} jobs not started, ${live.size} stopped`);
            pending.length = 0;
            for (const other of live) other.kill();
            for (const wake of waiting.splice(0)) wake();
          }
          else if (group === undefined) console.log(`ok   ${timed(`check:fast ${name}`, seconds, { cores: kept(cpu, start, Date.now()), ran: job.ran })}`);
          if (group !== undefined) {
            const done = groups.get(group)!;
            done.push(result);
            if (done.length === list.filter((other) => other.group === group).length && done.every((r) => r.code === 0)) {
              const span = Math.max(...done.map((r) => r.end)) - Math.min(...done.map((r) => r.start));
              const each = kept(
                done.reduce((sum, r) => sum + r.cpu, 0),
                began + Math.min(...done.map((r) => r.start)) * 1000,
                began + Math.max(...done.map((r) => r.end)) * 1000,
                done.reduce((sum, r) => sum + r.end - r.start, 0),
              );
              // Said as its files' own seconds, summed, the longest and the
              // span beside: the span, its first file's start to its last's
              // end, is most of the check where the pool starts its small
              // files last, the browsers' nine 250-276 s whose longest,
              // next.test.ts, takes 62-73 s (2026-11-17).
              console.log(`ok   ${timed(`check:fast ${group}`, groupSeconds(done), { cores: each, ran: done[0]!.job.ran })} ${groupSaid(done, span)}`);
            }
          }
          return result;
        })(),
      );
    }
  })();
  return started
    .then(() => Promise.all(ran))
    .then((results) => {
      // Each job's that passed, for the machine's next run, in any checkout.
      recordJobTimes(
        results
          .filter((r) => r.code === 0)
          .map((r) => ({ name: r.job.name, seconds: r.end - r.start, cores: kept(r.cpu, began + r.start * 1000, began + r.end * 1000), size: r.job.size })),
        { store: times },
      );
      return results;
    })
    .finally(() => {
      clearInterval(watching);
      clearInterval(sampling);
    });
}

/** How busy the machine's cores were while the jobs ran, on average and
 * each minute, and how much of the time under half; with `--profile`, each
 * job's start, end and the cores it kept busy, a group's files as one. */
function profiled(results: Ran[], samples: { at: number; busy: number }[], detail: boolean) {
  if (samples.length === 0) return;
  const percent = (share: number) => `${Math.round(share * 100)}%`;
  const mean = (values: number[]) => values.reduce((a, b) => a + b, 0) / Math.max(1, values.length);
  const under = samples.filter((s) => s.busy < 0.5).length / samples.length;
  console.log(`the cores ${percent(mean(samples.map((s) => s.busy)))} busy on average, under half busy ${percent(under)} of the time`);
  if (!detail) return;
  const minutes = Math.ceil(samples.at(-1)!.at / 60);
  const each = Array.from({ length: minutes }, (_, m) => percent(mean(samples.filter((s) => s.at > m * 60 && s.at <= (m + 1) * 60).map((s) => s.busy))));
  console.log(`each minute: ${each.join(", ")}`);
  const clock = (seconds: number) => `${Math.floor(Math.round(seconds) / 60)}:${String(Math.round(seconds) % 60).padStart(2, "0")}`;
  type Row = { name: string; start: number; end: number; cpu: number };
  const rows: Row[] = results
    .filter((r) => r.job.group === undefined)
    .map((r) => ({ name: r.job.name, start: r.start, end: r.end, cpu: r.cpu }))
    .sort((a, b) => a.start - b.start);
  const groups = new Set(results.flatMap((r) => (r.job.group === undefined ? [] : [r.job.group])));
  for (const group of groups) {
    const of = results.filter((r) => r.job.group === group);
    rows.push({ name: `${group} (${of.length})`, start: Math.min(...of.map((r) => r.start)), end: Math.max(...of.map((r) => r.end)), cpu: of.reduce((a, r) => a + r.cpu, 0) });
    const longest = [...of].sort((a, b) => b.end - b.start - (a.end - a.start)).slice(0, 5);
    for (const r of longest) rows.push({ name: `  ${r.job.name}`, start: r.start, end: r.end, cpu: r.cpu });
  }
  console.log(`${"job".padEnd(50)}${"start".padStart(7)}${"end".padStart(7)}${"cores".padStart(7)}`);
  for (const { name, start, end, cpu } of rows) {
    console.log(`${name.padEnd(50)}${clock(start).padStart(7)}${clock(end).padStart(7)}${(cpu / Math.max(0.1, end - start)).toFixed(1).padStart(7)}`);
  }
  // Where a CPU-bound check's cores go: the jobs that used the most of
  // them, a group's files each its own, as the lever on a saturated
  // machine is less work, not another order (#998).
  const total = results.reduce((a, r) => a + r.cpu, 0);
  console.log(`the jobs' CPU, ${Math.round(total)} core-seconds; the most of it:`);
  for (const r of [...results].sort((a, b) => b.cpu - a.cpu).slice(0, 15)) {
    console.log(`${r.job.name.padEnd(50)}${`${Math.round(r.cpu)} s`.padStart(14)}`);
  }
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
  // Another compiler's run of the suite, a mutant's that's to be retired
  // (`bun scripts/mutations.ts --retire=<name>`, ADR 0093, amended): every
  // test file and the corpus whole with its snapshots, as `--all` runs
  // them, with it as the tests' compiler; not the jobs that read the
  // checkout's crate, not its compiler (cargo test, clippy, the formats,
  // the change's mutations), nor what the run's selection or times say.
  const given = args.find((arg) => arg.startsWith("--compiler="))?.slice("--compiler=".length);
  // Its own line in the machine's log, where it isn't a step of a run that
  // writes one, a push's or a retirement's (scripts/devTimes.ts).
  const owns = given === undefined && ownsTimes("check:fast");
  const all = args.includes("--all") || given !== undefined;
  // The machine shared with concurrent runs, each its share, one heavy run
  // at a time (scripts/leases.ts): its wait for the turn apart from its
  // run, which its time and `--limit=<s>` are of (#2517).
  const waited = (holdLease("check") ?? 0) / 1000;
  const extra = args.filter((arg) => !arg.startsWith("--"));
  // What the loop left running, which takes the cores this run times
  // (scripts/strays.ts): said first, and again with its profile.
  const leftRunning = said(strays(root));
  if (leftRunning) console.warn(`warning: ${leftRunning}`);

  const git = (gitArgs: string[]) => Bun.spawnSync(["git", ...gitArgs], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const changed = [...git(["diff", "--name-only", "HEAD"]), ...git(["ls-files", "--others", "--exclude-standard"])];
  // And, of what only a push's check runs where it touches it, the
  // iterators', what the branch changed since it left `main`.
  const since = git(["merge-base", "HEAD", "origin/main"])[0];
  const branch = since === undefined ? [] : git(["diff", "--name-only", since, "HEAD"]);

  // The change's mutations CI's check judges (`changedSince`, its one
  // selection: those it adds or edits, by the runner's own rule, #923, and
  // those whose code or tests it touches), but those judged on this tree
  // already (scripts/judged.ts): what `bun run push` asks of the tree it
  // pushes, where before only those added or edited and not committed ran,
  // none of a push's (#2485).
  const { unjudgedSince } = await import("./judged");
  const unjudged = since === undefined ? undefined : await unjudgedSince(root, since);
  const mutations = unjudged === undefined ? undefined : { names: unjudged.names, base: since! };

  // The install matches the manifests before anything runs (#1817): a
  // rebase over a package or workspace change, `@rust-js/webapi`'s, left
  // `node_modules` without it, and a tsc test failed on its import.
  installed();

  // What no test used for a week, and past each store's cap (test/kept.ts),
  // as the whole suite prunes it: a Mac's native programs' builds grew 23
  // GB in two days. Every store inside the checkouts, the crate corpus's
  // projects and the temp root too, and a warning where they're near their
  // budget (#1213). In a process of its own, beside the build and then the
  // jobs, so it takes none of the run's time (`startup`, #2498).
  // Each step's time is kept, and said against its usual (scripts/timing.ts).
  const prepared = await startup(
    root,
    {
      build: ["cargo", "build", "--quiet"],
      prune: ["bun", "scripts/disk.ts", "--prune", "--inside", "--quiet"],
      // The compiles the tests keep (test/compiles.ts, ADR 1099): another
      // build's, never found again, and past the cap, the least recently
      // used. Not of another compiler's run: the checkout's build's would go.
      // Made once the build is done: the run's own link of it.
      compiles: given === undefined
        ? () => ["bun", "-e", `import { currentBuild, prune } from "./test/compiles.ts"; console.log(JSON.stringify(prune({ current: currentBuild(${JSON.stringify(ownCompiler(root, "check-fast"))}) })));`]
        : undefined,
    },
    () => freeBytes(root) < floorBytes,
  );
  if (prepared.build.code !== 0) process.exit(1);
  console.log(timed("cargo build", prepared.build.seconds));

  // The jobs' compiler is the run's own link of the one just built
  // (scripts/own-compiler.ts): the run's build however the checkout's
  // changes, and gone with the run.
  const own = ownFolder(root, "check-fast");
  const compiler = given ?? ownCompiler(root, "check-fast");
  // The apps' Cargo targets' seed, the binding crates checked once for the
  // run by the first test to ask, and gone with it (test/support.ts'
  // `seeded`, #1005).
  process.env.RUST_JS_APPS_SEED = join(own, "apps-seed");

  const compiles = prepared.compiles === undefined ? undefined : Promise.resolve(prepared.compiles);
  // Below the disk's floor, what either frees before the jobs keep anything.
  const prunedBefore = prepared.waited ? await pruned(prepared.prune, compiles, true) : undefined;
  if (prunedBefore === false) process.exit(1);

  // And the node bindings the bindings and lowering tests compile against,
  // once, before the jobs that ask for them: built by the first test to ask
  // under the others' load, it outran bun's 5 s test timeout, was killed, and
  // failed every test waiting on it (#227). test/node-crate.ts's `buildNode`, whose
  // stamp skips it where nothing changed.
  const binding = Date.now();
  const node = Bun.spawnSync(["bun", "-e", "import { buildNode } from './test/node-crate.ts'; buildNode();"], {
    cwd: root,
    env: { ...process.env, RUST_JS_COMPILER: compiler },
    stdout: "inherit",
    stderr: "inherit",
  });
  if (node.exitCode !== 0) process.exit(1);
  console.log(timed("node bindings build", (Date.now() - binding) / 1000));

  // The test files and corpus cases the change can affect, by the selector
  // (scripts/select.ts, ADR 1101, amended); everything where it can't say.
  // Of `--all`, it says which cases with std's iterator structs run (#986).
  const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
  // A sanity pass past a rebase (`--sanity=<the commit checked before
  // it>`, `bun run push`'s): the interaction of the change and the commits
  // the rebase brought, capped where even that can't be said.
  if (args.includes("--sanity")) throw new Error("--sanity names the commit checked before the rebase: --sanity=<sha>");
  const sanity = args.find((arg) => arg.startsWith("--sanity="))?.slice("--sanity=".length);
  // The push's rule: the change's own, never others' `all` (ADR 1101,
  // amended); CI's check keeps the full rule.
  // A sanity pass's is capped where it can't say, and held to its budget
  // (`budgeted`); what it ran and left kept for a red `main`'s reader.
  const selection = selectChange("origin/main", sanity, "push");
  const capped = selection.capped !== undefined;
  if (sanity && capped) console.log(`the sanity pass: ${outcome(selection)}`);
  if (sanity && given === undefined) recordSanity(selection, sanity);
  let picked: Scope["picked"];
  let forms: Scope["forms"];
  const formsList = join(own, "forms.txt");
  if (!selection.all) writeFileSync(formsList, selection.forms.map((name) => `${name}\n`).join(""));
  if (all) {
    if (!selection.all) forms = { forms: selection.forms, formsList };
    console.log(`${selection.why}: ${selection.all ? "the corpus with std's iterator structs runs whole" : `${selection.forms.length} of ${cases.length} corpus cases with std's iterator structs where the change is the iterators'`}`);
  } else {
    if (!selection.all) {
      const caseList = join(own, "cases.txt");
      writeFileSync(caseList, selection.cases.map((name) => `${name}\n`).join(""));
      picked = { files: new Set(selection.files), cases: selection.cases, caseList, forms: selection.forms, formsList };
    }
    console.log(`the selection: ${outcome(selection)}${selection.all ? "" : `, of ${pushSuite} test files and ${cases.length} corpus cases: ${selection.why}`}`);
  }
  // What it selected, which `bun run push` says on its line.
  const shadow = selection.shadow === undefined ? null : { outcome: outcome(selection.shadow), all: selection.shadow.all, files: selection.shadow.files, cases: selection.shadow.cases, forms: selection.shadow.forms };
  if (given === undefined) writeFileSync(selectionFile, JSON.stringify({ outcome: outcome(selection), all: selection.all, allOwn: selection.allOwn ?? null, shadow }));
  // Of a change to the iterators' code, the rows of their tables it names,
  // which size its job (#1364): some 0.1 s.
  const rows = touchesIterators([...changed, ...branch])
    ? changePicks(caseParts(machine().cores)).reduce((sum, p) => sum + p.rows, 0)
    : undefined;
  const check = jobs({ all, changed, branch, mutations, extra, cores: machine().cores, picked, forms, rows, capped });
  const cores = machine().cores;
  const clock: Clock = { began: Date.now(), samples: [] };
  const logs = checkLogRun();
  const listed = given === undefined ? check.jobs : check.jobs.filter((job) => job.cmd[0] === "bun" && job.cmd[1] === "test");
  const results = await run(listed, compiler, cores, clock, logs, given === undefined ? jobTimesStore : join(own, "job-times"), leaseStore, args.includes("--fail-fast"));
  const failed = results.filter((r) => r.code !== 0 && !r.cut);
  profiled(results, clock.samples, args.includes("--profile"));
  // The prunes beside the jobs, said as they've ended: one that failed
  // fails the check, as it did when it ran before them.
  const prunedOk = prunedBefore ?? (await pruned(prepared.prune, compiles, false));
  const failures = failed.length + (prunedOk ? 0 : 1);
  // Still left running as it ends: its numbers aren't a measurement.
  const leftAtEnd = said(strays(root));
  if (leftAtEnd) console.warn(`warning: ${leftAtEnd}`);
  for (const { job: { name }, log } of failed) console.log(`\n── ${name} ──\n${log.slice(-3000)}`);
  // Of another compiler's run, each test that failed with it, by its file,
  // which the retirement names (scripts/mutations.ts's `--retire`).
  const { failedTests } = await import("./mutations");
  // Of the item layer's selection in shadow, each test that failed here it
  // would have left out, a miss, which the push's line says (#2288).
  if (given === undefined && shadow !== null) {
    const missed = shadowMissed(shadow, failed.map(({ job: { cmd }, log }) => ({ file: cmd[2] ?? "", tests: failedTests(log) })));
    writeFileSync(selectionFile, JSON.stringify({ outcome: outcome(selection), all: selection.all, allOwn: selection.allOwn ?? null, shadow: { ...shadow, missed } }));
  }
  if (given !== undefined) for (const { job: { cmd }, log } of failed) for (const test of failedTests(log)) console.log(`failed with it: ${cmd[2]}\t${test}`);
  // Each failing job's whole output, kept past the next run as it failed
  // (#1622, #1706).
  const keptLogs = logs.kept();
  if (keptLogs !== undefined) console.log(`\neach failing job's whole output: ${keptLogs}`);
  // From its start, its build too, but its wait for the machine's turn,
  // which its line says apart (`turn`), not slower than its usual (#2517).
  const total = elapsed() - waited;
  // How many owners are of each source, as #2200's classification goes.
  console.log(sourceLine(ownerTable(readFileSync(join(root, OWNER_PAGE), "utf8"))));
  const scope = all ? "check:fast --all" : picked ? "check:fast, what the change reaches" : "check:fast, everything";
  console.log(
    `\n${failures === 0 ? `all passed: ${timed(scope, total, { ran: totalKey(listed) })}` : `${failures} failed in ${total.toFixed(0)}s`}`,
  );
  // Its line in the machine's log, its slowest jobs its steps; a mutant's
  // or a retirement's run is its runner's line.
  if (owns) {
    devTimes("check:fast", { started: 0, phases: slowest(jobSeconds(results)), selection: scope, ok: failures === 0, note: failures ? `${failures} failed` : "" });
  }
  process.exit(failures === 0 ? 0 : 1);
}

/** Each job's name and its seconds, from its start to its end, which a
 * run's `Ran` holds in seconds from the run's start: the log's steps
 * (#2322, where they were divided by 1,000 again, each `0s`). */
export function jobSeconds(results: { job: { name: string }; start: number; end: number }[]): [string, number][] {
  return results.map((r) => [r.job.name, r.end - r.start]);
}

/** What the install is of: the root's manifest, each workspace's and the
 * lockfile, as `read` gives each, hashed in order. */
export function manifestsHash(read: (file: string) => string): string {
  const workspaces: string[] = JSON.parse(read("package.json")).workspaces ?? [];
  const hash = createHash("sha256");
  for (const file of ["package.json", "bun.lock", ...workspaces.map((dir) => `${dir}/package.json`)]) hash.update(`${file}\0${read(file)}\0`);
  return hash.digest("hex");
}

/** Why this checkout's install isn't of its manifests, or `undefined`
 * where it is: the manifests changed since the hash its last install
 * stamped, `stamp`, or a workspace the root depends on, `workspace:*`,
 * isn't linked in `node_modules`, as of a checkout whose install predates
 * it, its stamp missing or not (#1817). */
export function installNeeded(read: (file: string) => string, stamp: string | undefined, linked: (name: string) => boolean): string | undefined {
  if (stamp !== manifestsHash(read)) return "the manifests changed since the last install";
  const manifest = JSON.parse(read("package.json"));
  const unlinked = Object.entries({ ...manifest.dependencies, ...manifest.devDependencies })
    .filter(([name, version]) => String(version).startsWith("workspace:") && !linked(name))
    .map(([name]) => name);
  if (unlinked.length) return `${unlinked.join(", ")} isn't linked in node_modules`;
}

/** `bun install --frozen-lockfile` where the install isn't of the
 * manifests (`installNeeded`), as a rebase over a new workspace or package
 * leaves it, before anything runs, else nothing: its stamp,
 * `node_modules/.rust-js-installed`, the manifests' hash it installed. A
 * lockfile the manifests don't match fails it, saying what to run. Of a
 * run whose stdout is an answer, `bun scripts/mutations.ts --browser`'s
 * one line, all it says goes to stderr (`quiet`). */
export function installed(who = "check:fast", quiet = false) {
  const stamp = join(root, "node_modules", ".rust-js-installed");
  const read = (file: string) => readFileSync(join(root, file), "utf8");
  const why = installNeeded(read, existsSync(stamp) ? readFileSync(stamp, "utf8") : undefined, (name) => existsSync(join(root, "node_modules", name)));
  if (!why) return;
  const now = manifestsHash(read);
  const installing = Date.now();
  const install = Bun.spawnSync(["bun", "install", "--frozen-lockfile"], { cwd: root, stdout: quiet ? "pipe" : "inherit", stderr: "inherit" });
  if (quiet && install.stdout) process.stderr.write(install.stdout);
  if (install.exitCode !== 0) {
    console.error(`${who}: \`bun install --frozen-lockfile\` failed: bun.lock doesn't match the package.json files; run \`bun install\` and commit bun.lock`);
    process.exit(1);
  }
  writeFileSync(stamp, now);
  (quiet ? console.error : console.log)(timed("bun install", (Date.now() - installing) / 1000, { detail: why }));
}

