// Where the compiler spends its time, lowering above all (#2302): a sampling
// profile, by samply, of a release build with line tables, over the corpus's
// compiles and the React port's, and a report of it by function.
//
//   bun scripts/profile-lowering.ts corpus [--cases=<n>] [--jobs=<n>]
//       each corpus case compiled as the corpus compiles it, as a program and
//       as a library (`compileCommand`), the first <n> cases, or all
//   bun scripts/profile-lowering.ts port
//       the React port's compiles (`scripts/react-in-rust.ts regen`), each
//       Cargo ran, recorded once and run again where samply sees them
//   bun scripts/profile-lowering.ts report <profile.json.gz> [--top=<n>]
//       the report alone, of a profile either wrote
//
// Each writes `target/profile-lowering/<what>.json.gz`, its symbols beside it
// (`.syms.json`), and the report, `<what>.md`; the profile opens in the
// Firefox Profiler with `samply load`. `--no-build` takes the release build
// as it is.
//
// samply follows a process's children by `DYLD_INSERT_LIBRARIES`, which
// macOS strips from a system binary, `/bin/sh` among them, so every compile
// here is spawned by Bun: Cargo's runs go through its `#!/bin/sh` wrapper,
// so they're recorded first, by a logger the wrapper runs as the compiler,
// then run again from here. samply is `cargo install samply`.

import { chmodSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { gunzipSync } from "node:zlib";
import { machine } from "./shard";

const root = resolve(import.meta.dir, "..");
const out = join(root, "target", "profile-lowering");
const release = join(root, "target", "release", "rust-js");
const flag = (name: string) => process.argv.find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3);
const [command, ...rest] = process.argv.slice(2).filter((arg) => !arg.startsWith("--"));

function must(cmd: string[], cwd = root, env: Record<string, string> = {}): void {
  const p = Bun.spawnSync(cmd, { cwd, env: { ...process.env, ...env }, stdout: "inherit", stderr: "inherit" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")}: exit ${p.exitCode}`);
}

/** The release build, its line tables kept so samples name their functions. */
function build(): void {
  if (process.argv.includes("--no-build")) return;
  must(["cargo", "build", "--release", "--bin", "rust-js"], root, { CARGO_PROFILE_RELEASE_DEBUG: "line-tables-only" });
}

/** `bun <this> drive <what> ..` recorded by samply into `<what>.json.gz`. */
function record(what: string, args: string[]): string {
  const samply = Bun.which("samply") ?? join(process.env.HOME ?? "", ".cargo/bin/samply");
  if (!existsSync(samply)) throw new Error("no samply: `cargo install samply`");
  mkdirSync(out, { recursive: true });
  const profile = join(out, `${what}.json.gz`);
  must([samply, "record", "--save-only", "--unstable-presymbolicate", "--rate", flag("rate") ?? "1000", "-o", profile, "--", process.execPath, import.meta.path, "drive", what, ...args]);
  return profile;
}

/** Each corpus case compiled as the corpus does, a program and a library. */
async function driveCorpus(): Promise<void> {
  process.env.RUST_JS_COMPILER = release;
  // The corpus's own command, spawned here: the kept compiles
  // (test/compiles.ts) would answer most of them without a run.
  const { compileCommand } = await import("../test/programs");
  const { directives } = await import("../test/corpus-cases");
  const corpus = join(root, "test", "corpus");
  const cases = readdirSync(corpus).filter((name) => name.endsWith(".rs")).sort();
  const count = flag("cases") === undefined ? cases.length : Number(flag("cases"));
  const work = cases.slice(0, count).flatMap((name) => [false, true].map((library) => ({ name, library })));
  const jobs = Number(flag("jobs") ?? machine().cores);
  let next = 0;
  await Promise.all(
    Array.from({ length: jobs }, async () => {
      while (next < work.length) {
        const { name, library } = work[next++];
        const file = join(corpus, name);
        const said = directives(readFileSync(file, "utf8"));
        const edition = typeof said === "string" ? "2024" : said.edition;
        const dir = join(out, "corpus", basename(name, ".rs"));
        mkdirSync(dir, { recursive: true });
        const { cmd } = compileCommand(file, dir, edition, library, []);
        await Bun.spawn(cmd, { cwd: root, stdout: "ignore", stderr: "ignore" }).exited;
      }
    }),
  );
}

/** The port's compiles: recorded through Cargo by a logger run as the
 * compiler, then each run again, in the order Cargo ran them. */
function drivePort(): void {
  const log = join(out, "port-invocations.jsonl");
  const invocations = readFileSync(log, "utf8").split("\n").filter(Boolean).map((line) => JSON.parse(line));
  // What samply gives this process to follow its children by, kept: the
  // compile's own environment is Cargo's, which has none of it.
  const followed = Object.fromEntries(Object.entries(process.env).filter(([key]) => /^(DYLD_|SAMPLY_)/.test(key)));
  for (const { argv, cwd, env } of invocations) {
    Bun.spawnSync([release, ...argv], { cwd, env: { ...env, ...followed }, stdout: "ignore", stderr: "ignore" });
  }
}

function recordPort(): string {
  mkdirSync(out, { recursive: true });
  const log = join(out, "port-invocations.jsonl");
  rmSync(log, { force: true });
  const logger = join(out, "logging-compiler");
  writeFileSync(
    logger,
    `#!${process.execPath}
import { appendFileSync } from "node:fs";
const argv = process.argv.slice(2);
// A compile, Cargo's as \`rustc\` itself or through its wrapper, not its
// probes of the compiler.
const asRustc = argv[0] === "--rustc" || /[\\/]rustc$/.test(argv[0] ?? "");
if (asRustc && !argv.includes("-vV") && !argv.some((a) => a.startsWith("--print")) && !argv.includes("-")) {
  appendFileSync(${JSON.stringify(log)}, JSON.stringify({ argv, cwd: process.cwd(), env: process.env }) + "\\n");
}
const p = Bun.spawnSync([${JSON.stringify(release)}, ...argv], { stdin: "inherit", stdout: "inherit", stderr: "inherit" });
process.exit(p.exitCode ?? 1);
`,
  );
  chmodSync(logger, 0o755);
  // The port's crates compiled whole, its JS left as the pin has it: in a
  // target of their own, so Cargo finds none fresh and runs every compile.
  const target = join(out, "port-target");
  rmSync(target, { recursive: true, force: true });
  Bun.spawnSync([process.execPath, join(root, "scripts/react-in-rust.ts"), "regen"], {
    cwd: root,
    env: { ...process.env, RUST_JS_COMPILER: logger, CARGO_TARGET_DIR: target },
    stdout: "inherit",
    stderr: "inherit",
  });
  if (!existsSync(log)) throw new Error("the port's build ran no compile through the logger");
  return record("port", []);
}

type Profile = {
  libs: { name: string }[];
  threads: {
    processName: string;
    name: string;
    samples: { stack: (number | null)[]; threadCPUDelta?: (number | null)[] };
    stackTable: { prefix: (number | null)[]; frame: number[] };
    frameTable: { address: number[]; func: number[] };
    funcTable: { resource: number[] };
    resourceTable: { lib: (number | null)[] };
  }[];
};
type Syms = { string_table: string[]; data: { debug_name: string; symbol_table: { rva: number; size: number; symbol: number }[] }[] };

/** Each sample's stack of function names, innermost first, of the
 * compiler's processes. */
function stacks(path: string): { names: string[][]; weights: number[] } {
  const profile: Profile = JSON.parse(gunzipSync(readFileSync(path)).toString());
  const syms: Syms = JSON.parse(readFileSync(path.replace(/\.json\.gz$/, ".json.syms.json"), "utf8"));
  const tables = new Map(syms.data.map((lib) => [lib.debug_name, [...lib.symbol_table].sort((a, b) => a.rva - b.rva)]));
  const symbol = (lib: string, rva: number): string => {
    const table = tables.get(lib);
    if (!table) return `${lib}@0x${rva.toString(16)}`;
    let [lo, hi] = [0, table.length - 1];
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (table[mid].rva <= rva) lo = mid + 1;
      else hi = mid - 1;
    }
    const entry = table[hi];
    return entry && rva < entry.rva + Math.max(entry.size, 1) ? syms.string_table[entry.symbol] : `${lib}@0x${rva.toString(16)}`;
  };
  const names: string[][] = [];
  const weights: number[] = [];
  for (const thread of profile.threads.filter((t) => t.processName === "rust-js")) {
    const frameName = thread.frameTable.address.map((address, frame) => {
      const lib = thread.resourceTable.lib[thread.funcTable.resource[thread.frameTable.func[frame]]];
      return lib === null || lib === undefined ? "?" : symbol(profile.libs[lib].name, address);
    });
    const memo = new Map<number, string[]>();
    const of = (stack: number): string[] => {
      const known = memo.get(stack);
      if (known) return known;
      const prefix = thread.stackTable.prefix[stack];
      const made = [frameName[thread.stackTable.frame[stack]], ...(prefix === null ? [] : of(prefix))];
      memo.set(stack, made);
      return made;
    };
    thread.samples.stack.forEach((stack, at) => {
      if (stack === null) return;
      // By the CPU time since the last sample, µs: a thread that waits,
      // the main thread joining its `rustc` thread, is sampled too.
      const cpu = thread.samples.threadCPUDelta?.[at] ?? 0;
      if (cpu <= 0) return;
      names.push(of(stack));
      weights.push(cpu);
    });
  }
  return { names, weights };
}

/** The phase a stack is in, by the first of the compiler's own frames from
 * its outermost in: lowering, preparing and printing, linking; else rustc's. */
const phases: [string, RegExp][] = [
  ["lowering (rust_js::lower)", /\brust_js::lower\b/],
  ["prepare (rust_js::prepare)", /\brust_js::prepare\b/],
  ["print (rust_js::to_oxc, oxc)", /\brust_js::(to_oxc|format|output)\b|\boxc_/],
  ["link (rust_js::link)", /\brust_js::link\b/],
];

/** A function's crate, its path's first segment: `hashbrown`, `rustc_middle`. */
function crateOf(name: string): string {
  const path = name.replace(/^<+/, "").replace(/^&(mut )?/, "");
  const m = /^([A-Za-z_][A-Za-z0-9_]*)::/.exec(path);
  return m ? m[1] : path.startsWith("_") || /^[a-z_]+$/.test(path) ? "libc/system" : "?";
}

function report(path: string): string {
  const top = Number(flag("top") ?? 40);
  const { names, weights } = stacks(path);
  const total = weights.reduce((a, b) => a + b, 0);
  const pct = (n: number) => `${((100 * n) / total).toFixed(1)}%`;
  const lines: string[] = [`# Where the compiler's time goes: ${basename(path)}`, "", `${(total / 1e6).toFixed(1)} CPU seconds of the compiler's processes, ${names.length} samples, each weighed by its thread's CPU time since the last, so a thread that waits counts for nothing.`, ""];
  // Phases, by the outermost of the compiler's frames that names one.
  const byPhase = new Map<string, number>();
  const inLowering: number[] = [];
  names.forEach((stack, at) => {
    const phase = [...stack].reverse().find((name) => phases.some(([, re]) => re.test(name)));
    const label = phase === undefined ? "rustc and the rest" : phases.find(([, re]) => re.test(phase))![0];
    byPhase.set(label, (byPhase.get(label) ?? 0) + weights[at]);
    if (label === phases[0][0]) inLowering.push(at);
  });
  lines.push("| phase | CPU s | share |", "|---|---:|---:|");
  for (const [label, n] of [...byPhase].sort((a, b) => b[1] - a[1])) lines.push(`| ${label} | ${(n / 1e6).toFixed(1)} | ${pct(n)} |`);
  const lowering = inLowering.reduce((a, at) => a + weights[at], 0);
  const ofLowering = (n: number) => `${((100 * n) / Math.max(lowering, 1)).toFixed(1)}%`;
  // Self time in lowering: the innermost frame, whosever it is.
  const selfTime = new Map<string, number>();
  const selfCrate = new Map<string, number>();
  const inclusive = new Map<string, number>();
  for (const at of inLowering) {
    const stack = names[at];
    selfTime.set(stack[0], (selfTime.get(stack[0]) ?? 0) + weights[at]);
    selfCrate.set(crateOf(stack[0]), (selfCrate.get(crateOf(stack[0])) ?? 0) + weights[at]);
    for (const name of new Set(stack.filter((name) => /\brust_js::lower\b/.test(name)))) inclusive.set(name, (inclusive.get(name) ?? 0) + weights[at]);
  }
  const table = (title: string, rows: Map<string, number>, n: number) => {
    lines.push("", `## ${title}`, "", "| function | CPU s | of lowering | of all |", "|---|---:|---:|---:|");
    for (const [name, count] of [...rows].sort((a, b) => b[1] - a[1]).slice(0, n)) {
      lines.push(`| \`${name.replaceAll("|", "\\|").slice(0, 160)}\` | ${(count / 1e6).toFixed(1)} | ${ofLowering(count)} | ${pct(count)} |`);
    }
  };
  table("Lowering's self time, by crate of the innermost frame", selfCrate, 20);
  table("Lowering's self time, by function", selfTime, top);
  table("Lowering's functions by inclusive time", inclusive, top);
  return lines.join("\n") + "\n";
}

function written(path: string): void {
  const text = report(path);
  const md = path.replace(/\.json\.gz$/, ".md");
  writeFileSync(md, text);
  console.log(text);
  console.log(`report: ${md}`);
}

switch (command) {
  case "corpus":
    build();
    written(record("corpus", process.argv.slice(3).filter((arg) => arg.startsWith("--") && !arg.startsWith("--no-build"))));
    break;
  case "port":
    build();
    written(recordPort());
    break;
  case "report":
    written(resolve(rest[0]));
    break;
  case "drive":
    if (rest[0] === "corpus") await driveCorpus();
    else if (rest[0] === "port") drivePort();
    break;
  default:
    throw new Error("bun scripts/profile-lowering.ts corpus|port|report <profile>");
}
