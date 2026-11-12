// The JSON benchmark of #1542, #1532's S0: one command that reproduces its
// tables, the settings S2 and S5 are judged by fixed here before results are.
//
//   bun scripts/serde-bench.ts            # every row
//   bun scripts/serde-bench.ts --quick    # 1 MB and 10 MB only, one sample each
//
// test/serde-probes/bench.rs's document, 4,000, 40,000 and 400,000 items
// (about 1, 10 and 96 MB of JSON), is written and read as a `Doc` and as a
// `Value` by rust-js's JS on Bun and Node (bench-harness.js), by native
// serde_json in a release build (bench-native.rs), and by each engine's own
// `JSON.parse` and `JSON.stringify`, a ceiling, not the same job; and the
// shapes of bench-shapes.rs, deep, wide, long strings and maps, at those
// sizes, read and written typed (a typed write reads first) and as a
// `Value` (bench-shapes-harness.js, bench-shapes-native.rs). Each row
// is run cold, one operation in a fresh process, and warm, the best of five
// in one, each three times in fresh processes, every sample kept; a
// process's peak memory is read by `/usr/bin/time -l`, beside each
// runtime's own with no work. Then the bundle of each of size-programs.ts'
// programs, minified and gzipped, and its compile time. Raw samples, the
// engines', the compiler's and the machine's versions go to
// target/serde-bench/results.json, the tables to target/serde-bench/tables.md.
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { arch, cpus, platform, release } from "node:os";
import { join } from "node:path";
import { gzipSync } from "node:zlib";
import { buildSerde, compiler, root } from "../test/support";
import { machine } from "./shard";

const probes = join(root, "test", "serde-probes");
const work = join(root, "target", "serde-bench");
const quick = process.argv.includes("--quick");
const sizes = quick ? [4_000, 40_000] : [4_000, 40_000, 400_000];
const samples = quick ? 1 : 3;
const timeout = 600_000;

type Run = { stdout: string; ms: number; peakMB: number };

/** `cmd` run in `work` under `/usr/bin/time -l`: what it printed, its own
 * timing's milliseconds, and its peak memory. */
function measure(cmd: string[]): Run {
  const p = Bun.spawnSync(["/usr/bin/time", "-l", ...cmd], { cwd: work, timeout, stdout: "pipe", stderr: "pipe" });
  const stdout = p.stdout.toString();
  const stderr = p.stderr.toString();
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")} failed:\n${stdout}${stderr}`);
  const ms = Number(/ ([\d.]+) ms /.exec(stdout)?.[1] ?? Number.NaN);
  const peak = Number(/(\d+)\s+maximum resident set size/.exec(stderr)?.[1] ?? Number.NaN);
  return { stdout: stdout.trim(), ms, peakMB: Math.round(peak / 2 ** 20) };
}

const run = (cmd: string[], cwd = work) => {
  const p = Bun.spawnSync(cmd, { cwd, timeout, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")} failed:\n${p.stdout}${p.stderr}`);
  return p.stdout.toString().trim();
};

rmSync(work, { recursive: true, force: true });
mkdirSync(work, { recursive: true });

// The builds: rust-js's JS of bench.rs beside the harness, and natively, a
// release build of bench.rs as a library its driver uses.
copyFileSync(join(probes, "bench-harness.js"), join(work, "bench-harness.js"));
const native = buildSerde("rlib-release");
const compileStart = performance.now();
run([compiler, join(probes, "bench.rs"), "-o", join(work, "bench.js"), "--", "--edition=2024", "-A", "warnings", ...buildSerde()], root);
const benchCompileMs = Math.round(performance.now() - compileStart);
run(["rustc", "--edition=2024", "-O", "-A", "warnings", "--crate-type=rlib", "--crate-name=bench", ...native,
  join(probes, "bench.rs"), "-o", join(work, "libbench.rlib")], root);
run(["rustc", "--edition=2024", "-O", "-A", "warnings", ...native, "--extern", `bench=${join(work, "libbench.rlib")}`, "-L", work,
  join(probes, "bench-native.rs"), "-o", join(work, "bench-native")], root);
// And the shapes, bench-shapes.rs's, the same ways.
copyFileSync(join(probes, "bench-shapes-harness.js"), join(work, "bench-shapes-harness.js"));
run([compiler, join(probes, "bench-shapes.rs"), "-o", join(work, "bench-shapes.js"), "--", "--edition=2024", "-A", "warnings", ...buildSerde()], root);
run(["rustc", "--edition=2024", "-O", "-A", "warnings", "--crate-type=rlib", "--crate-name=shapes", ...native,
  join(probes, "bench-shapes.rs"), "-o", join(work, "libshapes.rlib")], root);
run(["rustc", "--edition=2024", "-O", "-A", "warnings", ...native, "--extern", `shapes=${join(work, "libshapes.rlib")}`, "-L", work,
  join(probes, "bench-shapes-native.rs"), "-o", join(work, "bench-shapes-native")], root);

const runtimes: Record<string, (n: number, mode: string, reps: number) => string[]> = {
  bun: (n, mode, reps) => [process.execPath, "bench-harness.js", String(n), mode, String(reps)],
  node: (n, mode, reps) => ["node", "bench-harness.js", String(n), mode, String(reps)],
  native: (n, mode, reps) => [join(work, "bench-native"), String(n), mode, String(reps)],
};
// What each runtime does, and the engine's own JSON beside rust-js's.
const jobs: { runtime: string; mode: string }[] = [
  ...["bun", "node", "native"].flatMap((runtime) => ["write", "read", "readValue", "writeValue"].map((mode) => ({ runtime, mode }))),
  ...["bun", "node"].flatMap((runtime) => ["parse", "stringify"].map((mode) => ({ runtime, mode }))),
];

const versions = {
  bun: Bun.version,
  node: run(["node", "--version"]),
  rustc: run(["rustc", "--version"]),
  rustJs: run(["git", "rev-parse", "--short", "HEAD"], root),
  machine: { platform: platform(), release: release(), arch: arch(), cpu: cpus()[0]?.model, cores: machine().cores, memoryGB: Math.round(machine().totalGB) },
};

// Each runtime's own peak with no work: a document of one item written once.
run([join(work, "bench-native"), "1", "emit"]);
const baseline = Object.fromEntries(Object.entries(runtimes).map(([name, cmd]) => [name, measure(cmd(1, "write", 1)).peakMB]));

type Row = { size: number; bytes: number; runtime: string; mode: string; cold: Run[]; warm: Run[] };
const rows: Row[] = [];
for (const n of sizes) {
  run([join(work, "bench-native"), String(n), "emit"]);
  const bytes = statSync(join(work, `doc-${n}.json`)).size;
  for (const { runtime, mode } of jobs) {
    const cmd = (reps: number) => runtimes[runtime](n, mode, reps);
    const cold = Array.from({ length: samples }, () => measure(cmd(1)));
    const warm = Array.from({ length: samples }, () => measure(cmd(5)));
    rows.push({ size: n, bytes, runtime, mode, cold, warm });
    console.log(`${n} ${runtime} ${mode}: cold ${cold.map((r) => r.ms).join("/")} ms, warm ${warm.map((r) => r.ms).join("/")} ms, peak ${Math.max(...cold.map((r) => r.peakMB))} MB`);
  }
}

// Each shape at about each size the document is, its `n` scaled from a
// small one's bytes: written and read typed and as a `Value`.
const shapeNames = ["deep", "wide", "long", "map"];
const shapeRuntimes: Record<string, (shape: string, n: number, mode: string, reps: number) => string[]> = {
  bun: (shape, n, mode, reps) => [process.execPath, "bench-shapes-harness.js", shape, String(n), mode, String(reps)],
  node: (shape, n, mode, reps) => ["node", "bench-shapes-harness.js", shape, String(n), mode, String(reps)],
  native: (shape, n, mode, reps) => [join(work, "bench-shapes-native"), shape, String(n), mode, String(reps)],
};
type ShapeRow = { shape: string; n: number; bytes: number; runtime: string; mode: string; cold: Run[]; warm: Run[] };
const shapeRows: ShapeRow[] = [];
for (const shape of shapeNames) {
  run([join(work, "bench-shapes-native"), shape, "16", "emit"]);
  const per = statSync(join(work, `shape-${shape}-16.json`)).size / 16;
  for (const target of rows.filter((r, i, all) => all.findIndex((o) => o.size === r.size) === i).map((r) => r.bytes)) {
    const n = Math.max(1, Math.round(target / per));
    run([join(work, "bench-shapes-native"), shape, String(n), "emit"]);
    const bytes = statSync(join(work, `shape-${shape}-${n}.json`)).size;
    for (const runtime of ["bun", "node", "native"]) {
      for (const mode of ["readTyped", "writeTyped", "readValue", "writeValue"]) {
        const cmd = (reps: number) => shapeRuntimes[runtime](shape, n, mode, reps);
        const cold = Array.from({ length: samples }, () => measure(cmd(1)));
        const warm = Array.from({ length: samples }, () => measure(cmd(5)));
        shapeRows.push({ shape, n, bytes, runtime, mode, cold, warm });
        console.log(`${shape} ${n} ${runtime} ${mode}: cold ${cold.map((r) => r.ms).join("/")} ms, warm ${warm.map((r) => r.ms).join("/")} ms`);
      }
    }
  }
}

// The bundles: each of size-programs.ts' programs compiled, timed, and
// bundled minified, its bytes raw and gzipped.
const sized = join(work, "size");
mkdirSync(sized, { recursive: true });
copyFileSync(join(probes, "size-programs.ts"), join(sized, "size-programs.ts"));
run([process.execPath, "size-programs.ts"], sized);
const bundles = readdirSync(sized).filter((f) => f.endsWith(".rs")).sort().map((file) => {
  const name = file.slice(0, -3);
  const start = performance.now();
  run([compiler, join(sized, file), "-o", join(sized, name, "lib.js"), "--", "--edition=2024", "-A", "warnings", ...buildSerde()], root);
  const compileMs = Math.round(performance.now() - start);
  run([process.execPath, "build", join(sized, name, "lib.js"), "--minify", "--outfile", join(sized, name, "bundle.js")], root);
  const bundle = readFileSync(join(sized, name, "bundle.js"));
  return { name, compileMs, minified: bundle.length, gzipped: gzipSync(bundle).length };
});

writeFileSync(join(work, "results.json"), `${JSON.stringify({ versions, samples, baselinePeakMB: baseline, benchCompileMs, rows, shapeRows, bundles }, null, 2)}\n`);
const best = (runs: Run[]) => Math.min(...runs.map((r) => r.ms));
const peak = (runs: Run[]) => Math.max(...runs.map((r) => r.peakMB));
const table = [
  `# The JSON benchmark (#1542), ${versions.rustJs}`,
  "",
  `Bun ${versions.bun}, Node ${versions.node}, ${versions.rustc}; ${versions.machine.cpu}, ${versions.machine.cores} cores, ${versions.machine.memoryGB} GB.`,
  `Each cell the best of ${samples} processes, cold (one run) and warm (best of 5 in one); peak the largest. Baseline peak with no work: ${Object.entries(baseline).map(([k, v]) => `${k} ${v} MB`).join(", ")}.`,
  "",
  "| items | MB | runtime | mode | cold ms | warm ms | peak MB |",
  "|---|---|---|---|---|---|---|",
  ...rows.map((r) => `| ${r.size} | ${(r.bytes / 2 ** 20).toFixed(1)} | ${r.runtime} | ${r.mode} | ${best(r.cold).toFixed(1)} | ${best(r.warm).toFixed(1)} | ${peak(r.cold)} |`),
  "",
  `bench.rs compiled by rust-js in ${benchCompileMs} ms.`,
  "",
  "| shape | n | MB | runtime | mode | cold ms | warm ms | peak MB |",
  "|---|---|---|---|---|---|---|---|",
  ...shapeRows.map((r) => `| ${r.shape} | ${r.n} | ${(r.bytes / 2 ** 20).toFixed(1)} | ${r.runtime} | ${r.mode} | ${best(r.cold).toFixed(1)} | ${best(r.warm).toFixed(1)} | ${peak(r.cold)} |`),
  "",
  "| program | compile ms | minified bytes | gzipped bytes |",
  "|---|---|---|---|",
  ...bundles.map((b) => `| ${b.name} | ${b.compileMs} | ${b.minified} | ${b.gzipped} |`),
];
writeFileSync(join(work, "tables.md"), `${table.join("\n")}\n`);
console.log(table.join("\n"));
