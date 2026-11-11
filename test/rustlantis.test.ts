// The Rustlantis net (ADR 1125): programs of custom MIR, UB-free and
// deterministic by construction, made by Rustlantis (test/rustlantis.ts)
// from a seed each, small ones (test/rustlantis/config.toml), each run
// natively for wasm32-wasip1, rust-js's `usize`, and as JS in both builds;
// each prints every local it dumps, `{:?}`, so a write no later statement
// reads still shows. Every program runs as native Rust does, or rust-js
// refuses it, or what it does is an open issue's, test/rustlantis-known.txt;
// anything else fails, with the command that makes it again.
//
//   bun test test/rustlantis.test.ts                                           # the list's own tests
//   RUSTLANTIS_START=1 RUSTLANTIS_SEEDS=200 RUST_JS_CASES=1/4 bun test test/rustlantis.test.ts  # 200 seeds, a quarter
//
// The nightly runs 1,000 seeds of the day's, in 4 parts.
import { afterAll, beforeAll, expect, test } from "bun:test";
import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { compileFailure, runSync } from "./child";
import { partSetting } from "./part-settings";
import { agree, execute, jsLimit, runJs, runtimes, show, type Run } from "./programs";
import { configDir, generator, known, knownAs, type Known } from "./rustlantis";
import { buildCompiler, compiler, fixture, root } from "./support";

// The first seed, the day's of the nightly's, beside how many.
const startSetting = process.env.RUSTLANTIS_START;
const seedsSetting = partSetting("RUSTLANTIS_SEEDS");
const start = Number(startSetting ?? "1");
const count = Number(seedsSetting ?? "0");
if (!Number.isSafeInteger(start) || start < 0 || !Number.isSafeInteger(count) || count < 0) {
  throw new Error(`RUSTLANTIS_START=${startSetting} RUSTLANTIS_SEEDS=${seedsSetting}: whole numbers`);
}
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const seeds = Array.from({ length: count }, (_, i) => start + i).filter((_, i) => i % parts === part - 1);
const list = known();
const bootstrap = { RUSTC_BOOTSTRAP: "1" };
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];
const compileTimeout = 300_000;

let generate = "";
beforeAll(() => {
  if (seeds.length === 0) return;
  buildCompiler();
  generate = generator();
}, 1_200_000);

/** What each seed came to: `ran`, or what holds it, `refused #1384`. */
const tally = new Map<string, number>();
const count1 = (what: string) => tally.set(what, (tally.get(what) ?? 0) + 1);
afterAll(() => {
  if (tally.size) console.log(`rustlantis, seeds ${start} to ${start + count - 1}, part ${part}/${parts}: ${[...tally].sort().map(([k, n]) => `${k} ${n}`).join(", ")}`);
});

/** What `seed`'s program came to, or why it fails the net. */
function judge(seed: number): { held: string } | { failed: string } {
  const dir = fixture(`rustlantis-${seed}`);
  const made = Bun.spawnSync([generate, "-d", `${seed}`], { cwd: configDir, stdout: "pipe", stderr: "pipe", timeout: 60_000 });
  if (made.exitCode !== 0) return { failed: `the generator failed:\n${made.stderr.toString()}` };
  // `entry`, which the JS's runner calls as the corpus's (test/corpus-run.ts).
  const source = `${made.stdout.toString()}\npub fn entry() {\n    main()\n}\n`;
  const file = join(dir, "program.rs");
  writeFileSync(file, source);
  const wasm = join(dir, "native.wasm");
  const built = Bun.spawnSync(["rustc", file, "--target=wasm32-wasip1", "--edition=2021", "-Coverflow-checks=off", "-Awarnings", "-o", wasm], {
    cwd: root,
    env: { ...process.env, ...bootstrap },
    stdout: "pipe",
    stderr: "pipe",
    timeout: compileTimeout,
  });
  if (built.exitCode !== 0 || !existsSync(wasm)) return { failed: `native wasm32 didn't build it:\n${built.stderr.toString().slice(0, 4000)}` };
  const outcome = join(dir, "native.json");
  const native = execute([process.execPath, join(root, "test", "wasi-run.ts"), wasm, outcome], outcome);
  if (typeof native.outcome === "string" || !("value" in native.outcome)) return { failed: `native wasm32 didn't run it to its end: ${show(native.outcome)}\n${native.stderr}` };
  for (const [build, flags] of builds) {
    const js = join(dir, `${build}.js`);
    const compiled = runSync([compiler, file, "-o", js, ...flags, "--", "--edition=2021", "-Awarnings"], root, compileTimeout, bootstrap);
    if (compiled.code !== 0) {
      const { kind } = compileFailure(compiled, compileTimeout);
      if (kind === "crashed") {
        const held = knownAs(list, ["crashed"], compiled.stderr);
        if (held) return { held: `crashed ${held.by}` };
        return { failed: `rust-js crashed in the ${build} build:\n${compiled.stderr.slice(-4000)}` };
      }
      const refusals = [...new Set(compiled.stderr.split("\n").filter((line) => line.startsWith("error: rust-js")))];
      const unknown = refusals.filter((line) => !knownAs(list, ["refused"], line));
      if (refusals.length === 0 || unknown.length) return { failed: `rust-js refused it in the ${build} build, as no line of test/rustlantis-known.txt says:\n${(unknown.length ? unknown : [compiled.stderr]).join("\n")}` };
      const by = [...new Set(refusals.map((line) => (knownAs(list, ["refused"], line) as Known).by))].sort().join(" ");
      return { held: `refused ${by}` };
    }
    for (const [name, runtime] of runtimes) {
      const ran: Run = runJs(runtime, js, dir, `${name}-${build}`, jsLimit(native));
      if (agree(ran, native)) continue;
      const threw = typeof ran.outcome === "string" || !("value" in ran.outcome);
      const held = threw ? knownAs(list, ["threw"], `${ran.stderr}${show(ran.outcome)}`) : knownAs(list, ["wrong"], source);
      if (held) return { held: `${held.kind} ${held.by}` };
      return {
        failed: `the ${build} build's JS ${threw ? "threw" : "printed otherwise"} under ${name}, as no line of test/rustlantis-known.txt says:\nnative: ${show(native.outcome)}\n${native.stdout}\n${name}: ${show(ran.outcome)}\n${ran.stdout}${ran.stderr}`,
      };
    }
  }
  return { held: "ran" };
}

for (const seed of seeds) {
  test(`seed ${seed}`, () => {
    const verdict = judge(seed);
    if ("held" in verdict) {
      count1(verdict.held);
      return;
    }
    count1("failed");
    throw new Error(
      `rustlantis seed ${seed}: ${verdict.failed}\n\nAgain: RUSTLANTIS_START=${seed} RUSTLANTIS_SEEDS=1 RUST_JS_KEEP_FIXTURES=1 bun test test/rustlantis.test.ts`,
    );
  }, 900_000);
}

// The list's own: each line's kind and what holds it say what they are
// (`known`), and its patterns hold what the experiment found of each.
test("the Rustlantis net's list holds what each of its issues is", () => {
  const of = (kind: Known["kind"], text: string) => knownAs(list, [kind], text)?.by;
  // A wide literal's cast, #1380's, runs as Rust does: none holds it.
  expect(of("wrong", "_2 = 4706315301130638003_i64 as u32;")).toBeUndefined();
  expect(of("wrong", "_3 = (-57850311006394435500531941819513103853_i128) as isize;")).toBeUndefined();
  expect(of("wrong", "RET = -(-121074413017490068518913278327093031545_i128);\n_4 = RET as i8;")).toBeUndefined();
  expect(of("refused", "error: rust-js does not support calling `std::intrinsics::bswap` yet")).toBe("#1384");
  expect(of("refused", "error: rust-js does not support a use of a local before any write of it whole yet: ..")).toBe("custom-mir");
  expect(of("refused", "error: rust-js does not support calling `std::env::args` yet")).toBeUndefined();
  expect(of("crashed", "thread 'rustc' has overflowed its stack")).toBe("#1383");
});
