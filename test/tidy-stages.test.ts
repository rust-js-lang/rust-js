// The readability rewrites, each proved to change nothing a program does
// (ADRs 0659, 1087): a program's JS as tidy leaves it after each stage,
// `--tidy-until` its last pass; as preparation leaves it without each of
// its rewrites that changed it, `--prepare-without`; and with neither at
// its least, tidy stopped after its first stage and preparation without
// any rewrite; each run as the corpus runs it, prints and ends as native
// Rust does. Each is a rewrite of a body already correct, so what it
// leaves must be correct too: a variant after which a program does
// otherwise, or runs on where Rust ends, names the stage or the rewrite
// that made it so (`bun scripts/tidy-trace.ts <case>` shows tidy's).
//
//   case.rs ─┬─ rustc ──► native ──────────────────────────────────────┐
//            └─ rust-js --tidy-until <stage> | --prepare-without <r> ──► node ─┴─► the same?
//
//   bun test test/tidy-stages.test.ts                                        # a few cases, every variant
//   TIDY_STAGES=corpus RUST_JS_CASES=1/6 bun test test/tidy-stages.test.ts   # the corpus, a sixth of it
//   TIDY_STAGES=programs FUZZ_START=1000 FUZZ_SEEDS=100 bun test test/tidy-stages.test.ts   # generated programs (ADR 0092)

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { generate, print, type Program } from "./generate";
import { agree, compileJs, runJs, runNative, runtimes, show, type Run } from "./programs";
import { reduce, size } from "./reduce";
import { buildCompiler, caseShard, compiler, contentDirectory, fixture, run, target, writeWhole } from "./support";
import { record, runKey, shared, verified } from "./verified-runs";

beforeAll(() => {
  buildCompiler();
});

// Of the suite's own run, these: tidy's work is in their control flow,
// assignments, matches, early exits and drops, and preparation's in their
// tests, fallbacks, loops and objects.
const few = [
  "assignment_order.rs",
  "calculator.rs",
  "catch_unwind_drops.rs",
  // A trait's default, a class's prototype's method, holding a handle on
  // `this.count` until `drops`: its getter reads `this$`, not the handle's
  // own `this` (#459).
  "class_trait_methods.rs",
  "conditional_regions.rs",
  // A drop's guard inlined while it's a temporary, `_3`, named by no number.
  "drop_guard_name_shared.rs",
  // A `for` over a text escape, stepped by `$next` until `for…of` is made.
  "escape_iterators.rs",
  "interpreter.rs",
  "labeled_blocks.rs",
];
const which = process.env.TIDY_STAGES;
if (which !== undefined && which !== "corpus" && which !== "programs") {
  throw new Error(`TIDY_STAGES=${which}: \`corpus\`, \`programs\`, or unset for a few cases`);
}

/** A setting that's a whole number, or `otherwise`. */
function whole(name: string, otherwise: number): number {
  const value = process.env[name];
  if (value === undefined) return otherwise;
  if (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value))) throw new Error(`${name}=${value}: a whole number is a setting`);
  return Number(value);
}

const chosen = which === "corpus" ? caseShard(cases) : which === "programs" ? [] : few;
const start = whole("FUZZ_START", 1);
const seeds = which === "programs" ? Array.from({ length: whole("FUZZ_SEEDS", 12) }, (_, i) => start + i) : [];

/** The last pass of each of tidy's stages, in the order they run: what
 * `--tidy-until` stops after once a stage is done. A pass is
 * `stage/pass`, a stage run again `again/stage/pass`. */
let ends: string[] | undefined;
function stageEnds(): string[] {
  if (ends) return ends;
  const last = new Map<string, string>();
  for (const pass of run([compiler, "--tidy-passes"]).trim().split("\n")) {
    last.set(pass.slice(0, pass.lastIndexOf("/")), pass);
  }
  return (ends = [...last.values()]);
}

/** Each of preparation's rewrites, as `--prepare-without` names it. */
let rewrites: string[] | undefined;
function preparePasses(): string[] {
  return (rewrites ??= run([compiler, "--prepare-passes"]).trim().split("\n"));
}

/** A way to compile a program: its name, and rust-js's flags for it. */
type Variant = { name: string; flags: string[] };

/** A program's variants, of those of preparation's rewrites that changed
 * it, `fired`: tidy stopped after each stage; preparation without each
 * of those, as one that changed nothing leaves the JS as it is; and both
 * at their least. */
function variants(fired: string[]): Variant[] {
  const all = preparePasses();
  return [
    ...stageEnds().map((end) => ({ name: `after ${end}`, flags: ["--tidy-until", end] })),
    ...fired.map((pass) => ({ name: `without preparation's ${pass}`, flags: ["--prepare-without", pass] })),
    {
      name: `after ${stageEnds()[0]}, without any of preparation's rewrites`,
      flags: ["--tidy-until", stageEnds()[0], ...all.flatMap((pass) => ["--prepare-without", pass])],
    },
  ];
}

// The cases whose least variant doesn't run as native Rust does, each
// with its issue: a list that only shrinks. Each case's JS as built is
// right; its least variant runs what a later pass takes away.
const leastDiffers: Record<string, string> = {
  // A value cell's rewrite leaves a dead tuple of the clones it took away,
  // `const _10 = [_11, _14$1];`, which tidy's later stages, and
  // preparation's `unread constants`, remove (#641).
  "value_cell_set_after_its_call.rs": "#641",
};

/** A compile's JS, its map's line left out: what's run, and kept by. */
const code = (js: string) => readFileSync(js, "utf8").replace(/^\/\/# sourceMappingURL=.*$/m, "");

/** How `file`'s variants differ from `native`, each a line or more, or
 * none: `final` is its finished JS's error, a variant's refusal its own. */
function differences(file: string, dir: string, edition: string, native: Run, only?: string): string[] {
  const problems: string[] = [];
  const firedFile = join(dir, "fired.txt");
  const full = compileJs(file, dir, edition, false, {}, ["--prepare-fired", firedFile]);
  if (!("js" in full)) return [`rust-js failed: ${full.error.split("\n").slice(0, 6).join("\n")}`];
  const fired = readFileSync(firedFile, "utf8").split("\n").filter(Boolean);
  const finalEnd = stageEnds().at(-1);
  const least = variants([]).at(-1)!.name;
  const listed = leastDiffers[basename(file)];
  for (const { name, flags } of variants(fired).filter((variant) => only === undefined || variant.name === only)) {
    const compiled = compileJs(file, dir, edition, false, {}, flags);
    if (!("js" in compiled)) {
      // What runs before tidy is the same at every stage, so a stage the
      // case is refused at, and the finished one isn't, is refused by a
      // phase after tidy that reads a body as tidy finishes it: a class's
      // constructor, the struct literal it returns, or an exported cell,
      // read only by what it holds. That phase refuses the body as it
      // would however it was reached, so no JS is made to be wrong, and
      // there's none to run (ADR 0659, amended). A crash isn't a refusal,
      // nor is one of the finished stage, nor JS rust-js wrote and can't
      // read back, `const 3 = ..`, which is no phase's refusal but a
      // stage's broken body; nor one of preparation's, which runs after
      // every phase that refuses.
      const unreadable = compiled.error.includes("rust-js can't read the module it wrote");
      const tidied = flags[0] === "--tidy-until" && flags[1] !== finalEnd;
      if (compiled.kind === "rejected" && tidied && !unreadable) continue;
      problems.push(`${name}: rust-js failed: ${compiled.error.split("\n").slice(0, 6).join("\n")}`);
      continue;
    }
    // The same JS, of any variant, runs the same: it's run once.
    const key = runKey(["readability variant", code(compiled.js), native.bytes.stdout, native.bytes.stderr, show(native.outcome), shared()]);
    if (verified(key)) continue;
    const [runtime, cmd] = runtimes[0];
    const ran = runJs(cmd, compiled.js, dir, `${runtime}-${flags.join("-").replaceAll(/[^A-Za-z0-9]+/g, "_").slice(0, 80)}`);
    const agrees = agree(ran, native);
    if (name === least && listed !== undefined) {
      if (agrees) problems.push(`${name}: runs as native Rust does now; take ${basename(file)} off \`leastDiffers\` (${listed})`);
      continue;
    }
    if (agrees) record(key, `${file} ${name}`);
    else problems.push(`${name}: ${show(ran.outcome)}, printing\n${ran.stdout.slice(0, 400)}\nwhere native Rust ended ${show(native.outcome)}, printing\n${native.stdout.slice(0, 400)}`);
  }
  return problems;
}

test("the cases a run of a few names are the corpus's", () => {
  for (const name of few) expect(cases).toContain(name);
});

test("a stage's end is the last of its passes", () => {
  buildCompiler();
  const ends = stageEnds();
  expect(ends.length).toBeGreaterThan(10);
  expect(ends).toContain("finish/stepped slices");
  expect(ends.some((end) => end.startsWith("again/"))).toBe(true);
});

// What's checked is each stage's JS, not the finished JS each time: tidy
// stopped after its first stage leaves a body as no later stage does; and
// so is preparation's without a rewrite it made.
test("a case's JS after tidy's first stage isn't its JS after the last, nor without a rewrite that fired", () => {
  buildCompiler();
  const [first, ...rest] = stageEnds();
  const dir = fixture("stages-control");
  const at = (name: string, flags: string[]) => {
    const compiled = compileJs(join(corpus, name), dir, "2024", false, {}, flags);
    if (!("js" in compiled)) throw new Error(compiled.error);
    // Its map is named for the stage, as its file is: the code's what's compared.
    return code(compiled.js);
  };
  expect(at("conditional_regions.rs", ["--tidy-until", first])).not.toBe(at("conditional_regions.rs", ["--tidy-until", rest.at(-1)!]));
  // Of a case whose rewrites are made in place, a block's, and where
  // their conditions say, `undefined left out`'s.
  const fired = join(dir, "fired.txt");
  const full = at("interpreter.rs", ["--prepare-fired", fired]);
  const each = readFileSync(fired, "utf8").split("\n").filter(Boolean);
  expect(each).toEqual(expect.arrayContaining(["undefined left out", "elses after exits"]));
  for (const pass of each) expect([pass, at("interpreter.rs", ["--prepare-without", pass])]).not.toEqual([pass, full]);
});

// A variant is made only of a rewrite that fired, so one that didn't must
// leave the JS as it is: what `--prepare-fired` says is each that changed
// anything, or a rewrite's variant would go unchecked.
test("a rewrite of preparation's that fired nowhere leaves a case's JS as it is", () => {
  buildCompiler();
  const dir = fixture("stages-unfired");
  // Of a case whose rewrites of blocks and modules fire, every rewrite;
  // and of one whose `tests` rewrite shows in its JS, `if (!!x)`, that.
  const asked: [string, string[] | undefined][] = [
    ["calculator.rs", undefined],
    ["option_bool_tested.rs", ["tests"]],
  ];
  for (const [name, only] of asked) {
    const file = join(corpus, name);
    const fired = join(dir, "fired.txt");
    const compiled = compileJs(file, dir, "2024", false, {}, ["--prepare-fired", fired]);
    if (!("js" in compiled)) throw new Error(compiled.error);
    const full = code(compiled.js);
    const each = new Set(readFileSync(fired, "utf8").split("\n").filter(Boolean));
    const unfired = (only ?? preparePasses()).filter((pass) => !each.has(pass));
    if (only === undefined) expect(unfired.length).toBeGreaterThan(10);
    const changed = unfired.filter((pass) => {
      const without = compileJs(file, dir, "2024", false, {}, ["--prepare-without", pass]);
      if (!("js" in without)) throw new Error(without.error);
      return code(without.js) !== full;
    });
    expect([name, changed]).toEqual([name, []]);
  }
}, 300_000);

// A name that's no case is the test above's to report.
for (const name of chosen.filter((name) => cases.includes(name))) {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  // What doesn't run as Rust does yet, or doesn't run, has no stage to
  // check; a malformed directive is the corpus's to report.
  if (typeof want === "string" || want.kind === "compile-fail" || want.kind === "ignore-rust-js") continue;
  test(`${basename(name, ".rs")} after each of tidy's stages and without each of preparation's rewrites`, () => {
    const dir = fixture(`stages-${basename(name, ".rs")}`);
    const native = runNative(file, dir, want.edition, want.native);
    if (typeof native === "string") throw new Error(native);
    expect(differences(file, dir, want.edition, native).join("\n\n")).toBe("");
  }, 600_000);
}

/** A generated program's first variant that differs from native Rust, if
 * any: `undefined` of one rustc rejects or rust-js refuses, which
 * `fuzz.test.ts` judges. */
function programDiffers(program: Program, seed: number, only?: string): string | undefined {
  const dir = fixture(`stages-seed-${seed}`);
  const text = print(program, seed);
  const file = join(contentDirectory(text), "program.rs");
  writeWhole(file, text);
  const native = runNative(file, dir);
  if (typeof native === "string") return undefined;
  const [problem] = differences(file, dir, "2024", native, only);
  return problem?.startsWith("rust-js failed") ? undefined : problem;
}

for (const seed of seeds) {
  test(`generated program ${seed} after each of tidy's stages and without each of preparation's rewrites`, async () => {
    const program = generate(seed);
    const problem = programDiffers(program, seed);
    if (problem === undefined) return;
    // As small as the same variant still differs, kept as it's found.
    const variant = problem.slice(0, problem.indexOf(":"));
    const out = join(target, "fuzz");
    mkdirSync(out, { recursive: true });
    const repro = join(out, `seed-${seed}.stages.rs`);
    const keep = (p: Program) => writeFileSync(repro, print(p, seed));
    keep(program);
    const { program: smallest } = await reduce(program, async (candidate) => programDiffers(candidate, seed, variant) !== undefined, {
      until: Date.now() + 300_000,
      kept: keep,
    });
    expect(`${problem}\n\nreduced from ${size(program)} statements to ${size(smallest)} in ${repro}:\n\n${print(smallest, seed)}`).toBe("");
  }, 900_000);
}
