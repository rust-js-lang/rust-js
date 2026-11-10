// What a change is checked by before it's committed, all at once, in a
// minute or so: lint, the corpus in parts, the suites that check the JS
// bodies are lowered to from their MIR (ADR 0364), the snapshots and the
// diagnostics where the change touches what they read, and the mutations it
// adds or changes; of a change the test map can say, only the suites' files
// and the corpus's cases that ran a function it changes (scripts/testMap.ts,
// ADR 1101). CI runs the whole suite and every mutation of what the change
// touched after each push (DEVELOPMENT.md).
//
//   bun run check:fast                 the change's checks
//   bun run check:fast --all           what a push's check runs, but what `leftToCi` says is CI's
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
import { existsSync, linkSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, cpus, loadavg } from "node:os";
import { dirname, join, resolve } from "node:path";
import { currentBuild, prune } from "../test/compiles";
import { changedCases, runs, touchesAsync, touchesConsumers, touchesCrates, touchesDeclarations, touchesFormatting, touchesIterators, touchesMemory, touchesNumbers, touchesStdNet, touchesTwins } from "./iteratorSlice";
import { mappedFiles, pickChange, readersOf } from "./testMap";
import { browsers, caseParts, compilerBuilds, memoryBudget, nightly, peakGB, plainSuiteFiles, pooledParts } from "./shard";
import { said, strays } from "./strays";
import { elapsed, loadScale, median, timed, timings } from "./timing";

const root = join(import.meta.dir, "..");

/** A job of the check, a process of its own: its command, what's added to
 * its environment, the seconds it's given, past which it's stopped and
 * fails, how many of the machine's cores it's said to keep busy, its
 * `weight`, which the pool runs it by until its runs have measured them
 * (#857), the GB it holds at its peak, `memory`, a test's on each core it
 * keeps busy unless it says (scripts/shard.ts's `peakGB`, #1084), and, of
 * one that runs a command of
 * check.yml's, that command, as `test/ci.test.ts` matches it to check.yml's;
 * and of a test file, the `group` of files it's said with. */
export type Job = { name: string; cmd: string[]; env: Record<string, string>; timeout: number; weight: number; memory: number; ci?: string; group?: string };

/** What the check is of: `--all`, or the files the change touches, those
 * the branch changed since `main`, the mutations it adds and the files
 * named; the machine's cores; and, of a plain check whose change the test
 * map can say, the test files and corpus cases it can affect, the cases
 * written to `caseList`, a line each, and those built with std's iterator
 * structs, to `formsList` (scripts/testMap.ts, #877, #876); and of `--all`,
 * those with std's iterator structs alone, `forms`, where the map can say:
 * the corpus whole runs in its plain build, as a push's check runs it, and
 * in std's of the cases whose compiles ran what the change touches, the
 * nightly and CI running it whole (#986). */
export type Scope = {
  all: boolean;
  changed: string[];
  branch: string[];
  added: string[];
  extra: string[];
  cores: number;
  picked?: { files: Set<string>; cases: string[]; caseList: string; forms: string[]; formsList: string };
  forms?: { forms: string[]; formsList: string };
};

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
  { run: "rustup show", why: "a machine's setup: the toolchain, here installed already" },
  { run: "bun install --frozen-lockfile", why: "a machine's setup: the packages, here installed already" },
  { run: "bunx --bun playwright install", why: "a machine's setup: the browser, here installed by `bun run setup`" },
  { run: "cargo build --locked", why: "the compiler, which check:fast builds first, for every job" },
  { run: "BLESS=1", why: "a red run's bless patch: here a `BLESS=1` writes it in place" },
  { run: "bun run wasm", why: "the WASM compiler, a build of rustc's frontend to wasm32 of about half an hour" },
  {
    run: "bun scripts/react-in-rust.ts regen",
    why: "React's port at its pin, fetched and its crates built, about 100 s of a runner: here `bun scripts/react-in-rust.ts regen` by hand, as a port's change is one's own (#1188)",
  },
  {
    run: "bun test test/snapshots.test.ts test/playground.test.ts",
    why: "the snapshots and the playground with the WASM compiler (`RUST_JS_REQUIRE_WASM`), which only CI builds; check:fast runs both with the native one",
  },
  {
    run: "uses: ./.github/workflows/rustc-tests.yml",
    why: "rustc's whole suite on 6 machines, and the mutations of all a check's changes: here `bun run test:rustc <dirs>`, and the mutations the change adds",
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

/** The check's jobs, through the pool at once: of
 * `--all`, what a push's check runs (check.yml); else the suites the
 * change can touch, as it touches them, `shared` saying whether that's what
 * the snapshots and the diagnostics read. */
export function jobs({ all, changed, branch, added, extra, cores, picked, forms }: Scope): { jobs: Job[]; shared: boolean } {
  // What the snapshots and the diagnostics read: the compiler and `@rust-js/std`.
  const shared = all || changed.some((file) => file.startsWith("src/") || file.startsWith("std/"));
  const jsx = all || changed.some((file) => /^src\/(lower\/jsx|jsx_syntax)|^react\//.test(file));
  // A change to tidy or preparation, or to the oracles, is checked after
  // each of tidy's stages and without each of preparation's rewrites, and by
  // the read cases (ADRs 0659, 1087): of `--all` too only then, as a push's
  // CI runs them, readability-passes.yml's paths, the nightly every case
  // (the user, #998: 28–34 core-seconds of every compiler change's check).
  const tidy = touchesReadability([...changed, ...branch]);
  // A change to what a `&mut` is, or to an operation on one, runs the `&mut`
  // matrix's sample against native Rust (ADR 1086).
  const mutRefs = all || changed.some((file) => /^src\/lower\/(representation\/mut_ref|mut_refs|mir\/(assign|entries|places|rvalues|closures)|std_types\/cell)\.rs$/.test(file));
  // A change to how a place holds a `None`, or to what reads or writes
  // one, runs the `None` matrix's sample against native Rust (#855).
  const nones = all || changed.some((file) => /^src\/lower\/(representation\/(nones|options)|bindings\/nullable|std_types\/option|analysis\/nulled_params)/.test(file));
  // A change to the JS object model's owners, what a struct JS may make
  // is, or the readability passes, or to the net itself, runs the object
  // model's sample: a library's values read through counting proxies
  // (#891).
  const objectModel =
    all ||
    tidy ||
    [...changed, ...branch].some((file) =>
      /^(src\/lower\/(classes|untagged|field_keys|bindings|representation|analysis\/fn_pointers)|src\/js\/(stmt_walk|expr_walk)|test\/object-model)/.test(file),
    );
  // A change to the iterators' code, the working tree's or the branch's
  // since `main`, runs the iterator matrix's and the conformance table's
  // slices and the corpus with std's iterator structs, as a push's check
  // does (scripts/iteratorSlice.ts, #731, #766): with `--all` too, which is
  // what the push's check runs, these where its change is the iterators'.
  const iterators = touchesIterators([...changed, ...branch]);
  // And a change to the formatting code, the formatting grid's slice, as a
  // push's check runs it (#941).
  const formatting = touchesFormatting([...changed, ...branch]);
  // And to the numbers' code, the numbers grid's slice (#940).
  const numbers = touchesNumbers([...changed, ...branch]);
  // And to the pointers' code, the memory net's slice (#1172).
  const memory = touchesMemory([...changed, ...branch]);
  // And to what a `None` is at the JS boundary, the declarations'
  // TypeScript callers, in one part (#822).
  const declarations = touchesDeclarations([...changed, ...branch]);
  // And to how a future awaits and drops, the async table (#942).
  const awaits = touchesAsync([...changed, ...branch]);
  // And to the iterators' code or the drops', the consumer net.
  const consumers = touchesConsumers([...changed, ...branch]);
  // And to std's collections', slices', strings' or cells' code, the std net (#1171).
  const stdNet = touchesStdNet([...changed, ...branch]);
  // And to the iterators' code, the runtime's among it, the twin net: the
  // runtime's helpers against std's own bodies (#1205).
  const twins = touchesTwins([...changed, ...branch]);
  // And each corpus case it adds or edits, each of its metamorphic
  // rewrites, plain or `--all` (#1165).
  const cases = changedCases([...changed, ...branch]);
  // And the crate corpus, of a change to what compiles it, plain too, as a
  // push's check runs it (#1188); `--all`'s always (#998).
  const crates = all || touchesCrates([...changed, ...branch]);

  // What a script that runs its own parts at once runs in, the iterator
  // slices' (scripts/shard.ts).
  const parts = caseParts(cores);
  const half = Math.max(1, Math.floor(cores / 2));
  // The mutations' builds of the compiler at once, as many as the cores and
  // the memory hold (#1084).
  const builds = compilerBuilds(cores);
  // Each part of the corpus, a process of its own, a job of the pool's
  // (scripts/shard.ts's `pooledParts`).
  // Of a change the test map can say, the cases it picked, in as many parts
  // as there are cores, or fewer where fewer cases: none where it picked
  // none, nor the file's own tests, which the first part runs.
  const corpusParts = picked
    ? picked.cases.length > 0 || picked.files.has("test/corpus.test.ts")
      ? Math.max(1, Math.min(pooledParts(cores), Math.ceil(picked.cases.length / 8)))
      : 0
    : pooledParts(cores);
  const inParts = (name: string, file: string) =>
    Array.from({ length: corpusParts }, (_, i) =>
      // As a push's: of the corpus's, only the cases whose JS as a library
      // differs are built as one (ADR 0360).
      job(`${name} ${i + 1}/${corpusParts}`, ["bun", "test", file], {
        env: { RUST_JS_CASES: `${i + 1}/${corpusParts}${picked ? `:${picked.caseList}` : ""}`, RUST_JS_LIBRARIES: "differing" },
        ci: "bun run test",
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
  const everyFile = testFiles
    .filter((file) => file !== "test/corpus.test.ts" && !nightly.includes(file))
    .map((file) =>
      job(file, ["bun", "test", file], { group: browsers.includes(file) ? "the browsers' test files" : "every other test file", ci: "bun run test" }),
    );
  // The plain suites (scripts/shard.ts), each file a job of the pool's, said
  // as its suite: as one `bun test` a suite, lowering's 74 files ran one
  // after another, 70 s, the last job of every plain check, the cores half
  // idle 44% of it (#877). Of a change the test map can say, the files it
  // picked, of every suite, JSX's and the shared ones too where it reaches
  // them; else as the change's paths say.
  const suites: Job[] = plainSuiteFiles(testFiles).flatMap(({ name, files, when }) =>
    files
      .filter((file) => (picked ? picked.files.has(file) : when === undefined || (when === "jsx" ? jsx : shared)))
      .map((file) => job(file, ["bun", "test", file], { group: name })),
  );
  return {
    jobs: [
      job("fmt", ["bun", "run", "fmt:check"], { ci: "bun run fmt:check" }),
      job("typecheck", ["bun", "run", "typecheck"], { ci: "bun run typecheck" }),
      job("clippy", ["cargo", "clippy", "--locked", "--quiet", "--", "-D", "warnings"], { weight: 2, memory: peakGB.compilerCheck, ci: "cargo clippy --locked -- -D warnings" }),
      // With `--all`, the compiler's unit tests, tidy's passes and
      // decisions, red on `main` unseen after 801d40c55 (#523, #732): its
      // build waits on clippy's lock of `target/`, then takes 13 s here,
      // a cold build of its tests a minute or more. And the Vite example's
      // build, check.yml's `examples`, its committed JS against what
      // rust-js writes, then Vite's: 4 s.
      ...(all
        ? [
            job("cargo test", ["cargo", "test", "--locked", "--quiet"], { timeout: 600, weight: 2, memory: peakGB.compilerCheck, ci: "cargo test --locked" }),
            job("the Vite example's build", ["bun", "run", "--cwd", "examples/vite-react", "build"], { ci: "bun run --cwd examples/vite-react build" }),
          ]
        : []),
      // The crates of crates.io's corpus against docs/crate-corpus.md,
      // half the cores' at once: 22–33 s, which only the Nightly ran, so a
      // refusal that stopped itertools went unseen (#857, #1188). Its top
      // crates by downloads sampled, one in 8 by the commit (#1174).
      ...(crates
        ? [job("the crate corpus", ["bun", "scripts/crate-corpus.ts", "--check", "--sample"], { weight: half, memory: half * peakGB.crate, ci: "bun scripts/crate-corpus.ts --check --sample" })]
        : []),
      ...inParts("corpus", "test/corpus.test.ts"),
      ...(all ? everyFile : suites),
      ...(tidy ? [job("readability passes", ["bun", "test", "test/tidy-stages.test.ts", "test/getter-reads.test.ts"], { weight: 2 })] : []),
      ...(mutRefs ? [job("the `&mut` matrix's sample", ["bun", "test", "test/mut-ref-matrix.test.ts"])] : []),
      ...(nones ? [job("the `None` matrix's sample", ["bun", "test", "test/none-matrix.test.ts"])] : []),
      ...(objectModel ? [job("the object model's sample", ["bun", "test", "test/object-model.test.ts"])] : []),
      // The slices: 64 and 43 batches in 4 parts, 119-145 s and 63-119 s
      // alone at a load of 14-20; the corpus's every case (2026-11-05).
      ...(iterators
        ? [
            // And the rows of both tables the change names, sized to it
            // (scripts/iteratorRows.ts, #818).
            ...(["matrix", "conformance", "forms", "rows"] as const)
              // Of a change the test map can say, the corpus with std's
              // iterator structs runs the cases it picked, none where it
              // picked none, nor the file's own tests (#876).
              .filter(
                (of) =>
                  of !== "forms" ||
                  (picked ? picked.forms.length > 0 || picked.files.has("test/iterator-forms.test.ts") : !forms || forms.forms.length > 0),
              )
              .map((of) =>
                // The rows a change names: a removal's run whole, some 28,000
                // of the matrix's, about 4 min alone and twice that beside the
                // rest (#856).
                // Each `parts` processes at once, throughout (m6-lily, #857).
                job(
                  of === "rows" ? "the rows the change names" : runs[of][0].name,
                  ["bun", "scripts/iteratorSlice.ts", "run", of, String(parts), ...(of === "forms" && (picked ?? forms) ? [(picked ?? forms)!.formsList] : [])],
                  {
                    timeout: of === "rows" ? 1200 : 600,
                    weight: parts,
                    ci: `bun scripts/iteratorSlice.ts run ${of}`,
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
            }),
          ]
        : []),
      ...(added.length > 0
        ? [
            job("mutations the change adds", ["bun", "scripts/mutations.ts", `--jobs=${builds}`, ...new Set(added)], {
              weight: builds,
              memory: builds * peakGB.compilerBuild,
            }),
          ]
        : []),
      ...extra.filter((file) => !suites.some((suite) => suite.name === file)).map((file) => job(file, ["bun", "test", file])),
    ],
    shared,
  };
}

/** When the jobs began, and the machine's busy share every two seconds
 * since. */
type Clock = { began: number; samples: { at: number; busy: number }[] };

/** A job's run: how it ended, its log, when it started and ended, in
 * seconds from the pool's start, and the CPU seconds it and the children
 * it waited on used, which `--profile` divides by its time for the cores
 * it kept busy. */
type Ran = { job: Job; code: number; log: string; start: number; end: number; cpu: number };

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
 * start, `clock.samples` the machine's busy share every two seconds. */
function run(list: Job[], compiler: string, cores: number, { began, samples }: Clock): Promise<Ran[]> {
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
  // Each step's last runs, read once: a job's usual, or a test file's from
  // what `bun test` and this pool keep of each (`fileTimings`).
  const earlier = timings();
  const files = fileTimings();
  const before = ({ name }: Job) => earlier.filter((t) => t.step === `check:fast ${name}`).map((t) => t.seconds);
  const usual = (job: Job) => {
    if (job.group !== undefined) return job.name in files ? files[job.name] / 1000 : Number.POSITIVE_INFINITY;
    const last = before(job).slice(-10);
    return last.length > 0 ? median(last) : Number.POSITIVE_INFINITY;
  };
  // What it may take, by which the pool starts it: a group's file's usual,
  // or the longest of a job's last five, so that one whose usual is cached,
  // cargo test's under a second, but whose change rebuilds it, 36 s, isn't
  // started last and run alone at the end.
  const longest = (job: Job) => {
    if (job.group !== undefined) return usual(job);
    const last = before(job).slice(-5);
    return last.length > 0 ? Math.max(...last) : Number.POSITIVE_INFINITY;
  };
  // The cores it keeps busy, as its last runs measured them, or, of a job
  // not yet run, as it says: the corpus's parts, said to keep two, kept
  // one, and the browsers' files, said to keep all twelve, kept 3.6, so
  // the cores sat a third idle (2026-11-07). A group's, each file's.
  const weight = ({ name, group, weight: said }: Job) => {
    const measured = earlier.filter((t) => t.step === `check:fast ${group ?? name}` && t.cores !== undefined).slice(-5).map((t) => t.cores!);
    return Math.min(cores, measured.length > 0 ? Math.max(0.25, median(measured)) : said);
  };
  const queue = [...list].sort((a, b) => longest(b) - longest(a));
  const groups = new Map<string, Ran[]>();
  for (const { group } of list) if (group !== undefined) groups.set(group, []);
  let held = 0;
  let heldMemory = 0;
  const budget = memoryBudget();
  let waiting: (() => void)[] = [];
  const room = async (weight: number, memory: number) => {
    while (held > 0 && (held + weight > cores || heldMemory + memory > budget)) await new Promise<void>((wake) => waiting.push(wake));
    held += weight;
    heldMemory += memory;
  };
  const freed = (weight: number, memory: number) => {
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
    for (const job of queue) {
      const { name, cmd, env, timeout: given, group } = job;
      const holds = weight(job);
      await room(holds, job.memory);
      ran.push(
        (async () => {
          const start = Date.now();
          const expected = Math.max(2, scale);
          const p = Bun.spawn(cmd, {
            cwd: root,
            env: { ...process.env, RUST_JS_COMPILER: compiler, RUST_JS_LOAD_SCALE: String(expected), ...env },
            stdout: "pipe",
            stderr: "pipe",
          });
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
          clearInterval(stop);
          freed(holds, job.memory);
          const seconds = (Date.now() - start) / 1000;
          const stopped = code !== 0 && seconds >= timeout ? `, stopped at its ${timeout} s (×${scale.toFixed(1)} for the load)` : "";
          const cpu = Number(p.resourceUsage()?.cpuTime.total ?? 0) / 1e6;
          const result: Ran = { job, code, log: out + err, start: (start - began) / 1000, end: (Date.now() - began) / 1000, cpu };
          if (code !== 0) console.log(`FAIL ${name} (${seconds.toFixed(0)}s${stopped})`);
          else if (group === undefined) console.log(`ok   ${timed(`check:fast ${name}`, seconds, undefined, kept(cpu, start, Date.now()))}`);
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
              console.log(`ok   ${timed(`check:fast ${group}`, span, undefined, each)} (${done.length} files)`);
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
      noteFileTimings(results.filter((r) => r.job.group !== undefined && r.code === 0));
      return results;
    })
    .finally(() => {
      clearInterval(watching);
      clearInterval(sampling);
    });
}

/** Where each test file's time is kept, in ms, as `bun test --timings`
 * keeps it (scripts/test.ts writes it too): the pool starts the longest
 * first, so the last to end isn't one long file alone. */
const fileTimingsPath = join(root, "target", "test-timings.json");

function fileTimings(): Record<string, number> {
  try {
    return JSON.parse(readFileSync(fileTimingsPath, "utf8"));
  } catch {
    return {};
  }
}

function noteFileTimings(results: Ran[]) {
  const kept = fileTimings();
  for (const { job, start, end } of results) kept[job.name] = Math.round((end - start) * 1000);
  mkdirSync(join(root, "target"), { recursive: true });
  writeFileSync(fileTimingsPath, JSON.stringify(kept, null, 2));
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
  const all = args.includes("--all");
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

  // The mutations the change adds or edits, by the runner's own rule: one
  // whose `find` alone changed is the change's too, where a diff's `name`
  // lines missed it (#923).
  const { ownSince } = await import("./mutations");
  const added = (await ownSince("HEAD")).map((m) => m.name);

  // What no test used for a week, and past each store's cap, before any
  // runs (test/kept.ts), as the whole suite prunes it: a Mac's native
  // programs' builds grew 23 GB in two days. In a process of its own while
  // the compiler builds, so it takes none of the run's time: 0.1 s of a
  // store of 2,000 programs, a few seconds where it removes many.
  const pruning = Date.now();
  // Every store inside the checkouts, the crate corpus's projects and the
  // temp root too, and a warning where they're near their budget (#1213).
  const prunedKept = Bun.spawn(["bun", "scripts/disk.ts", "--prune", "--inside", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" }).exited.then((code) => ({ code, at: Date.now() }));

  // Each step's time is kept, and said against its usual (scripts/timing.ts).
  const building = Date.now();
  const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
  if (build.exitCode !== 0) process.exit(1);
  console.log(timed("cargo build", (Date.now() - building) / 1000));
  const keptPruned = await prunedKept;
  if (keptPruned.code !== 0) process.exit(1);
  console.log(timed("pruning what tests keep", (keptPruned.at - pruning) / 1000));

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
  // The apps' Cargo targets' seed, the binding crates checked once for the
  // run by the first test to ask, and gone with it (test/support.ts'
  // `seeded`, #1005).
  process.env.RUST_JS_APPS_SEED = join(own, "apps-seed");

  // The compiles the tests keep (test/compiles.ts, ADR 1099), bounded
  // before any runs: another build's, never found again, and past the
  // cap, the least recently used.
  const pruningCompiles = Date.now();
  const compiles = prune({ current: currentBuild(compiler) });
  const dropped = [compiles.builds ? "another build's" : "", compiles.removed ? `${compiles.removed} past the cap or the disk's floor` : ""].filter(Boolean);
  console.log(`${timed("pruning kept compiles", (Date.now() - pruningCompiles) / 1000)}: ${(compiles.size / 2 ** 20).toFixed(0)} MB kept${dropped.length ? `, ${dropped.join(" and ")} dropped` : ""}`);

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

  // Of a plain check, the files and corpus cases the change can affect, by
  // the test map (scripts/testMap.ts, #877); or, where it can't say, what
  // the change's paths run.
  let picked: Scope["picked"];
  let forms: Scope["forms"];
  // Of `--all`, the cases built with std's iterator structs the map picks,
  // where it can say (#986).
  if (all) {
    const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
    const pick = pickChange(mappedFiles(), cases);
    if (pick.kind === "picked") {
      const formsList = join(own, "forms.txt");
      writeFileSync(formsList, pick.forms.map((name) => `${name}\n`).join(""));
      forms = { forms: pick.forms, formsList };
      console.log(`${pick.why}: ${pick.forms.length} of ${cases.length} corpus cases with std's iterator structs where the change is the iterators'`);
    } else console.log(`${pick.why}: the corpus with std's iterator structs runs whole`);
  }
  if (!all) {
    const cases = readdirSync(join(root, "test", "corpus")).filter((name) => name.endsWith(".rs"));
    const files = mappedFiles();
    const pick = pickChange(files, cases);
    if (pick.kind === "picked") {
      const caseList = join(own, "cases.txt");
      const formsList = join(own, "forms.txt");
      writeFileSync(caseList, pick.cases.map((name) => `${name}\n`).join(""));
      writeFileSync(formsList, pick.forms.map((name) => `${name}\n`).join(""));
      picked = { files: new Set(pick.files), cases: pick.cases, caseList, forms: pick.forms, formsList };
      console.log(
        `${pick.why}: ${pick.files.length} of ${files.length + 2} test files, ${pick.cases.length} of ${cases.length} corpus cases, ${pick.forms.length} of them with std's iterator structs where the change is the iterators'`,
      );
    } else console.log(`${pick.why}: the change's paths say what runs`);
  }
  // Each test file that reads a changed file as text, its source says,
  // `ci.test.ts` of `scripts/`: it can fail on a change it runs no code of,
  // and runs of it, whatever the map or the paths pick (a9aedb875).
  if (!all) {
    const testFiles = readdirSync(join(root, "test"))
      .filter((name) => name.endsWith(".test.ts"))
      .map((name) => `test/${name}`);
    for (const [test, read] of readersOf([...new Set([...changed, ...branch])], testFiles)) {
      if (extra.includes(test)) continue;
      extra.push(test);
      console.log(`${test} reads ${read}: it runs`);
    }
  }
  const check = jobs({ all, changed, branch, added, extra, cores: availableParallelism(), picked, forms });
  const cores = availableParallelism();
  const clock: Clock = { began: Date.now(), samples: [] };
  const results = await run(check.jobs, compiler, cores, clock);
  const failed = results.filter((r) => r.code !== 0);
  profiled(results, clock.samples, args.includes("--profile"));
  // Still left running as it ends: its numbers aren't a measurement.
  const leftAtEnd = said(strays(root));
  if (leftAtEnd) console.warn(`warning: ${leftAtEnd}`);
  for (const { job: { name }, log } of failed) console.log(`\n── ${name} ──\n${log.slice(-3000)}`);
  // From its start, its build too.
  const total = elapsed();
  const scope = all ? "check:fast --all" : picked ? "check:fast, what the change reaches" : check.shared ? "check:fast" : "check:fast, the change's";
  console.log(
    `\n${failed.length === 0 ? `all passed: ${timed(scope, total)}` : `${failed.length} failed in ${total.toFixed(0)}s`}${picked || check.shared ? "" : " (JSX, snapshots and diagnostics skipped)"}`,
  );
  process.exit(failed.length === 0 ? 0 : 1);
}
