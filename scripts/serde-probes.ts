// The serde / serde_json probes of #1532's S0: one bounded command that
// makes the programs of test/serde-probes, builds each natively with the
// pinned serde (serde/Cargo.lock) and with rust-js, runs both, and compares
// each outcome with the one recorded: test/serde-probes/outcomes.txt, or
// before there's one, Fable's of f3f4bfd25, which is kept as it was.
//
//   bun scripts/serde-probes.ts                 # every program, on Bun and Node
//   bun scripts/serde-probes.ts num_ints pos_03 # the ones named
//
// An outcome is `same`, `refused`, `compiler-panic`, `different` (a wrong
// answer or a runtime throw) or `native-invalid` (rustc rejects the program,
// so it's no test of rust-js). Of each program whose outcome on Bun isn't
// the recorded one, it prints the program's path and both outcomes; it
// fails if one that was `same` is no longer. The outcomes of this run are
// written to target/serde-probes/outcomes-<engine>.txt, in the recorded
// file's form; `--record` writes Bun's to test/serde-probes/outcomes.txt.
// A compiler that doesn't end in its time is a `compiler-panic`.
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { buildSerde, compiler, root } from "../test/support";
import { atOnce } from "./shard";

const probes = join(root, "test", "serde-probes");
const baseline = join(probes, "outcomes.txt");
const recorded = existsSync(baseline) ? baseline : join(probes, "outcomes-at-f3f4bfd25.txt");
const work = join(root, "target", "serde-probes");
const programs = join(work, "p");
const engines = ["bun", "node"] as const;
type Engine = (typeof engines)[number];
type Outcome = "same" | "refused" | "compiler-panic" | "different" | "native-invalid";

const args = process.argv.slice(2);
const record = args.includes("--record");
const named = args.filter((arg) => !arg.startsWith("--"));

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
 * it; the drop probes are copied as they are. */
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

/** Each engine's outcome of program `name`. */
async function probe(name: string, flags: { native: string[]; js: string[] }): Promise<Record<Engine, Outcome>> {
  const source = join(programs, `${name}.rs`);
  const binary = join(work, "bin", name);
  const js = join(work, "js", `${name}.js`);
  const native = await exec(["rustc", "--edition=2024", "-A", "warnings", ...flags.native, source, "-o", binary], root, 300_000);
  if (native.code !== 0) return { bun: "native-invalid", node: "native-invalid" };
  const expected = ran(await exec([binary], work, 20_000));
  const built = await exec([compiler, source, "-o", js, "--", "--edition=2024", "-A", "warnings", ...flags.js], root, 120_000);
  writeFileSync(join(work, "out", `${name}.rsjs-err`), built.stderr);
  if (built.code !== 0) {
    const outcome: Outcome = /panicked at|internal compiler error/.test(built.stderr) || built.timedOut ? "compiler-panic" : "refused";
    return { bun: outcome, node: outcome };
  }
  // Its JS with its `main` called, as run-one.sh runs it.
  const runner = join(work, "js", `${name}.run.mjs`);
  writeFileSync(runner, `${readFileSync(js, "utf8")}\nmain();\n`);
  const outcomes = {} as Record<Engine, Outcome>;
  for (const engine of engines) {
    const got = ran(await exec([engine === "bun" ? process.execPath : "node", runner], work, 20_000));
    writeFileSync(join(work, "out", `${name}.${engine}`), got);
    outcomes[engine] = got === expected ? "same" : "different";
  }
  writeFileSync(join(work, "out", `${name}.native`), expected);
  return outcomes;
}

const was = new Map(
  readFileSync(recorded, "utf8").split("\n").filter(Boolean).map((line) => {
    const [outcome, name] = line.split(" ");
    return [name, outcome as Outcome];
  }),
);
const all = await make();
const chosen = named.length ? all.filter((name) => named.includes(name)) : all;
for (const name of named) if (!all.includes(name)) throw new Error(`no program ${name}`);
for (const dir of ["bin", "js", "out"]) mkdirSync(join(work, dir), { recursive: true });
const flags = { native: buildSerde("rlib"), js: buildSerde() };
if (!existsSync(compiler)) throw new Error(`${compiler} isn't built: run \`cargo build\``);

// In parallel parts, as many as `atOnce` says of tests, each a program at a time.
const results = new Map<string, Record<Engine, Outcome>>();
const queue = [...chosen];
await Promise.all(Array.from({ length: atOnce("test") }, async () => {
  for (let name = queue.shift(); name !== undefined; name = queue.shift()) {
    results.set(name, await probe(name, flags));
  }
}));

for (const engine of engines) {
  const lines = [...results].map(([name, outcomes]) => `${outcomes[engine]} ${name}`).sort();
  writeFileSync(join(work, `outcomes-${engine}.txt`), `${lines.join("\n")}\n`);
}
const tally = (engine: Engine) => {
  const counts = new Map<Outcome, number>();
  for (const outcomes of results.values()) counts.set(outcomes[engine], (counts.get(outcomes[engine]) ?? 0) + 1);
  return [...counts].sort().map(([outcome, n]) => `${n} ${outcome}`).join(", ");
};
for (const engine of engines) console.log(`${engine}: ${tally(engine)}`);

let regressed = 0;
for (const name of chosen) {
  const now = results.get(name)!;
  const before = was.get(name);
  if (before !== now.bun) {
    if (before === "same") regressed++;
    console.log(`${name}: ${before ?? "unrecorded"} on Bun before, ${now.bun} now: ${resolve(programs, `${name}.rs`)}`);
  }
  if (now.node !== now.bun) console.log(`${name}: ${now.bun} on Bun, ${now.node} on Node: ${resolve(programs, `${name}.rs`)}`);
}
if (record) copyFileSync(join(work, "outcomes-bun.txt"), baseline);
if (regressed) {
  console.log(`${regressed} that gave native Rust's answer no longer do`);
  process.exit(1);
}
