// The serde / serde_json probes of #1532's S0: one bounded command that
// makes the programs of test/serde-probes, builds each natively with the
// pinned serde (serde/Cargo.lock) and with rust-js, runs both, and compares
// each outcome with the one recorded.
//
//   bun scripts/serde-probes.ts                 # every program, each way, on Bun and Node
//   bun scripts/serde-probes.ts num_ints pos_03 # the ones named
//
// Each program is built three ways, each with its own record:
// - `program`, a program, its `main` run (test/serde-probes/outcomes.txt, or
//   before there's one, the review's of f3f4bfd25, which is kept as it was);
// - `library`, a library whose `entry` runs its `main`, as the corpus builds
//   one (outcomes-library.txt);
// - `split`, its items moved into a library crate of their own by the
//   metamorphic net's `library` rewrite, which the program uses: the library
//   compiled first, the program against its manifest and metadata, a producer
//   and its consumer each compiled on its own (outcomes-split.txt). Natively
//   it's built so too, and that run is its answer.
//
// An outcome is `same`, `refused`, `compiler-panic` (or a compiler that
// doesn't end in its time), `different` (a wrong answer or a runtime throw),
// `native-invalid` (rustc rejects the program, so it's no test of rust-js)
// or, of `split`, `unsplit` (the rewrite has no items to move). Of each
// program whose outcome on Bun isn't the recorded one, it prints the
// program's path and both outcomes, and each whose outcome on Node isn't
// Bun's; it fails if one that was `same` is no longer. This run's outcomes
// are written to target/serde-probes/outcomes-<way>-<engine>.txt, in the
// recorded files' form; `--record` writes Bun's over the recorded ones.
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { buildRewriter, libraryCrate, rewritten } from "../test/metamorphic";
import { buildSerde, compiler, root } from "../test/support";
import { atOnce } from "./shard";

const probes = join(root, "test", "serde-probes");
const work = join(root, "target", "serde-probes");
const programs = join(work, "p");
const engines = ["bun", "node"] as const;
const ways = ["program", "library", "split"] as const;
type Engine = (typeof engines)[number];
type Way = (typeof ways)[number];
type Outcome = "same" | "refused" | "compiler-panic" | "different" | "native-invalid" | "unsplit";
type Outcomes = Record<Way, Record<Engine, Outcome>>;

/** Each way's record. */
const recordOf = (way: Way) => join(probes, way === "program" ? "outcomes.txt" : `outcomes-${way}.txt`);

const args = process.argv.slice(2);
const record = args.includes("--record");
const named = args.filter((arg) => !arg.startsWith("--"));
// A record is of every program: one of some would lose the others'.
if (record && named.length > 0) throw new Error("--record records every program: name none");

/** A command's output and exit, ended at `timeout` milliseconds. */
async function exec(cmd: string[], cwd: string, timeout: number) {
  const child = Bun.spawn(cmd, { cwd, stdout: "pipe", stderr: "pipe", timeout, killSignal: "SIGKILL" });
  const [stdout, stderr, code] = await Promise.all([
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  return { stdout, stderr, code, timedOut: child.signalCode === "SIGKILL" };
}

/** The programs, made as their generators make them, in an empty directory:
 * each `probes-N.ts` writes `p/<name>.rs`, and reads `probes-3.ts` beside
 * it; the drop probes and the review issues' are copied as they are. */
async function make(): Promise<string[]> {
  rmSync(work, { recursive: true, force: true });
  mkdirSync(programs, { recursive: true });
  for (const file of readdirSync(probes)) {
    if (/^probes-\d\.ts$/.test(file)) copyFileSync(join(probes, file), join(work, file));
  }
  for (const file of readdirSync(probes).filter((f) => /^probes-\d\.ts$/.test(f)).sort()) {
    const made = await exec([process.execPath, file], work, 60_000);
    if (made.code !== 0) throw new Error(`${file} failed:\n${made.stderr}`);
  }
  for (const file of readdirSync(probes).filter((f) => /^(drop|issue)_\d+\.rs$/.test(f))) {
    copyFileSync(join(probes, file), join(programs, file));
  }
  return readdirSync(programs).filter((f) => f.endsWith(".rs")).map((f) => f.slice(0, -3)).sort();
}

/** What a program ran to: its output and its exit, as run-one.sh writes it. */
const ran = (r: { stdout: string; stderr: string; code: number | null; timedOut: boolean }) =>
  `${r.stdout}${r.stderr}exit ${r.timedOut ? "timeout" : r.code}\n`;

const flags = { native: buildSerde("rlib"), js: buildSerde() };
const rustc = ["--edition=2024", "-A", "warnings"];
const both = (outcome: Outcome): Record<Engine, Outcome> => ({ bun: outcome, node: outcome });

/** What a compile by rust-js that failed is. */
const failed = (built: { stderr: string; timedOut: boolean }): Outcome =>
  /panicked at|internal compiler error/.test(built.stderr) || built.timedOut ? "compiler-panic" : "refused";

/** `runner`'s outcome on each engine, against `expected`, its runs kept in `out` as `<name>.<engine>`. */
async function runs(runner: string, expected: string, out: string): Promise<Record<Engine, Outcome>> {
  const outcomes = {} as Record<Engine, Outcome>;
  for (const engine of engines) {
    const got = ran(await exec([engine === "bun" ? process.execPath : "node", runner], work, 20_000));
    writeFileSync(`${out}.${engine}`, got);
    outcomes[engine] = got === expected ? "same" : "different";
  }
  return outcomes;
}

/** Program `name` built and run each way. */
async function probe(name: string): Promise<Outcomes> {
  const source = join(programs, `${name}.rs`);
  const binary = join(work, "bin", name);
  const native = await exec(["rustc", ...rustc, ...flags.native, source, "-o", binary], root, 300_000);
  if (native.code !== 0) return { program: both("native-invalid"), library: both("native-invalid"), split: both("native-invalid") };
  const expected = ran(await exec([binary], work, 20_000));
  writeFileSync(join(work, "out", `${name}.native`), expected);
  return { program: await asProgram(name, source, expected), library: await asLibrary(name, source, expected), split: await asSplit(name, source) };
}

/** As a program, its JS with its `main` called, as run-one.sh runs it. */
async function asProgram(name: string, source: string, expected: string): Promise<Record<Engine, Outcome>> {
  const js = join(work, "js", `${name}.js`);
  const built = await exec([compiler, source, "-o", js, "--", ...rustc, ...flags.js], root, 120_000);
  writeFileSync(join(work, "out", `${name}.rsjs-err`), built.stderr);
  if (built.code !== 0) return both(failed(built));
  const runner = join(work, "js", `${name}.run.mjs`);
  writeFileSync(runner, `${readFileSync(js, "utf8")}\nmain();\n`);
  return runs(runner, expected, join(work, "out", name));
}

/** As a library, `entry` calling its `main`, as the corpus builds one: its
 * text, with `entry` after it, which `include!` can't be given, as the
 * programs begin with an inner attribute. */
async function asLibrary(name: string, source: string, expected: string): Promise<Record<Engine, Outcome>> {
  const dir = join(work, "library", name);
  mkdirSync(dir, { recursive: true });
  const wrapper = join(dir, "lib.rs");
  writeFileSync(wrapper, `${readFileSync(source, "utf8")}\npub fn entry() {\n    main()\n}\n`);
  const built = await exec([compiler, wrapper, "-o", join(dir, "lib.js"), "--library", "--manifest", join(dir, "lib.manifest.json"),
    "--", ...rustc, ...flags.js], root, 120_000);
  writeFileSync(join(dir, "rsjs-err"), built.stderr);
  if (built.code !== 0) return both(failed(built));
  const runner = join(dir, "run.mjs");
  writeFileSync(runner, `import { entry } from "./lib.js";\nentry();\n`);
  return runs(runner, expected, join(dir, "run"));
}

/** Split, its items a library crate of their own its program uses, each
 * compiled on its own: natively, its answer, and by rust-js. */
async function asSplit(name: string, source: string): Promise<Record<Engine, Outcome>> {
  const made = rewritten("library", source, "2024");
  if (made?.library === undefined) return both("unsplit");
  const dir = join(work, "split", name);
  mkdirSync(join(dir, "native"), { recursive: true });
  const library = join(dir, "library.rs");
  const program = join(dir, "program.rs");
  writeFileSync(library, made.library);
  writeFileSync(program, made.program);
  const rlib = join(dir, "native", `lib${libraryCrate}.rlib`);
  const binary = join(dir, "native", "program");
  const lib = await exec(["rustc", ...rustc, ...flags.native, "--crate-type=rlib", `--crate-name=${libraryCrate}`, library, "-o", rlib], root, 300_000);
  const app = lib.code === 0
    ? await exec(["rustc", ...rustc, ...flags.native, "--extern", `${libraryCrate}=${rlib}`, program, "-o", binary], root, 300_000)
    : lib;
  if (app.code !== 0) return both("native-invalid");
  const expected = ran(await exec([binary], work, 20_000));
  const manifest = join(dir, libraryCrate, "lib.manifest.json");
  const rmeta = join(dir, `lib${libraryCrate}.rmeta`);
  const producer = await exec([compiler, library, "-o", join(dir, libraryCrate, "lib.js"), "--library", "--manifest", manifest,
    "--", ...rustc, "--crate-name", libraryCrate, `--emit=metadata=${rmeta}`, ...flags.js], root, 120_000);
  const consumer = producer.code === 0
    ? await exec([compiler, program, "-o", join(dir, "app", "lib.js"), "--dependency", manifest,
      "--", ...rustc, "--extern", `${libraryCrate}=${rmeta}`, "-L", dir, ...flags.js], root, 120_000)
    : producer;
  writeFileSync(join(dir, "rsjs-err"), consumer.stderr);
  if (consumer.code !== 0) return both(failed(consumer));
  const runner = join(dir, "app", "run.mjs");
  writeFileSync(runner, `${readFileSync(join(dir, "app", "lib.js"), "utf8")}\nmain();\n`);
  return runs(runner, expected, join(dir, "run"));
}

/** Each way's record: of `program`, the review's of f3f4bfd25 where there's none yet. */
const was = new Map(ways.map((way) => {
  const file = existsSync(recordOf(way)) ? recordOf(way) : way === "program" ? join(probes, "outcomes-at-f3f4bfd25.txt") : undefined;
  const lines = file === undefined ? [] : readFileSync(file, "utf8").split("\n").filter(Boolean);
  return [way, new Map(lines.map((line) => {
    const [outcome, name] = line.split(" ");
    return [name, outcome as Outcome];
  }))];
}));
if (!existsSync(compiler)) throw new Error(`${compiler} isn't built: run \`cargo build\``);
buildRewriter();
const all = await make();
for (const name of named) if (!all.includes(name)) throw new Error(`no program ${name}`);
const chosen = named.length ? all.filter((name) => named.includes(name)) : all;
for (const dir of ["bin", "js", "out", "library", "split"]) mkdirSync(join(work, dir), { recursive: true });

// In parallel parts, as many as `atOnce` says of tests, each a program at a time.
const results = new Map<string, Outcomes>();
const queue = [...chosen];
await Promise.all(Array.from({ length: atOnce("test") }, async () => {
  for (let name = queue.shift(); name !== undefined; name = queue.shift()) {
    results.set(name, await probe(name));
  }
}));

let regressed = 0;
for (const way of ways) {
  for (const engine of engines) {
    const lines = [...results].map(([name, outcomes]) => `${outcomes[way][engine]} ${name}`).sort();
    writeFileSync(join(work, `outcomes-${way}-${engine}.txt`), `${lines.join("\n")}\n`);
    const counts = new Map<Outcome, number>();
    for (const outcomes of results.values()) counts.set(outcomes[way][engine], (counts.get(outcomes[way][engine]) ?? 0) + 1);
    console.log(`${way}, ${engine}: ${[...counts].sort().map(([outcome, n]) => `${n} ${outcome}`).join(", ")}`);
  }
  const before = was.get(way)!;
  for (const name of chosen) {
    const now = results.get(name)![way];
    const path = resolve(programs, `${name}.rs`);
    if (before.size > 0 && before.get(name) !== now.bun) {
      if (before.get(name) === "same") regressed++;
      console.log(`${way} ${name}: ${before.get(name) ?? "unrecorded"} on Bun before, ${now.bun} now: ${path}`);
    }
    if (now.node !== now.bun) console.log(`${way} ${name}: ${now.bun} on Bun, ${now.node} on Node: ${path}`);
  }
  if (record) copyFileSync(join(work, `outcomes-${way}-bun.txt`), recordOf(way));
}
if (regressed) {
  console.log(`${regressed} that gave native Rust's answer no longer do`);
  process.exit(1);
}
