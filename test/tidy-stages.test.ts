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
// And each fact an expression carries that only permits a plainer form, or
// says how the program wrote it (`js::Known`'s roles, ADR 1127): a program
// compiled with one forgotten, `--without-known`, does the same, as a
// rewrite that drops a fact must leave it. One whose JS does otherwise
// names a fact its reader needs, a form's, not a permission.
//
// And each std function a recogniser took (ADR 1128): a program compiled
// with that recogniser left out, `--without-recognizer`, is refused, or
// does the same. What rust-js does of a call no recogniser takes is then
// never a wrong answer: a missing case of a recogniser is a refusal.
//
//   case.rs ─┬─ rustc ──► native ──────────────────────────────────────────────────────────┐
//            └─ rust-js --tidy-until <stage> | --prepare-without <r> | --without-known <k> ──► node ─┴─► the same?
//            └─ rust-js --without-recognizer <std fn> ──► refused, or node ──────────────────────────┴─► the same?
//
//   bun test test/tidy-stages.test.ts                                        # a few cases, every variant
//   TIDY_STAGES=corpus RUST_JS_CASES=1/6 bun test test/tidy-stages.test.ts   # the corpus, a sixth of it
//   TIDY_STAGES=programs FUZZ_START=1000 FUZZ_SEEDS=100 bun test test/tidy-stages.test.ts   # generated programs (ADR 0092)

import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { generate, print, type Program } from "./generate";
import { agree, compileJs, compileJsAsync, jsLimit, runJs, runNative, runtimes, show, type CompileError, type Run } from "./programs";
import { reduce, size } from "./reduce";
import { buildCompiler, caseShard, compiler, contentDirectory, fixture, run, target, writeWhole } from "./support";
import { machine } from "../scripts/shard";
import { record, runKey, shared, verified } from "./verified-runs";
import { partSetting } from "./part-settings";

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
  // A closure's parameter of a temporary's name, `(_2) => _2[0]`, beside
  // main's `_2` that a match binds: named temporaries leave the closure's
  // reads its own (#1105).
  "closure_param_shadows_a_named_temporary.rs",
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
  // `?` of a write's result whose `io::Error` holds what drops: its
  // `ControlFlow` dropped by its variants holds nothing, never a test of
  // `undefined.TAG` that tidy's first stage left (nightly 43133510260).
  "io_error_drops_what_it_holds.rs",
  "labeled_blocks.rs",
  // A value cell's clones, no JS, in a tuple: of them alone, none; beside
  // what isn't one, `undefined` in their place, never an undeclared
  // temporary's read (#641).
  "value_cell_set_after_its_call.rs",
  // A drop that panics as a panic unwinds aborts at every stage end: tidy
  // guards its cleanups wherever it stops (#847).
  "unwinding_drop_aborts_locals.rs",
  // A `while let` of a `by_ref()`'s steps: what isn't known to be a JS
  // iterator is taken for an array, so `Known::Iterator` is a form, never
  // forgotten (ADR 1127).
  "by_ref_steps.rs",
  // A `finally` the lowering wrote of what never throws, which tidy's
  // contract takes for guarded by `Known::NeverThrows`: forgotten, the
  // contract still reads it (ADR 1127).
  "flattened_state_got_mut.rs",
];
const which = partSetting("TIDY_STAGES");
if (which !== undefined && which !== "corpus" && which !== "programs") {
  throw new Error(`TIDY_STAGES=${which}: \`corpus\`, \`programs\`, or unset for a few cases`);
}

/** A setting that's a whole number, or `otherwise`: a part's, read as one
 * (test/part-settings.ts), so a run that sets it knows it was read. */
function whole(name: string, otherwise: number): number {
  const value = partSetting(name);
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

/** Each fact the JS is right without, as `--without-known` names it: one
 * that permits, or says what was written; never one that warns or is a
 * form (`rust-js --knowns`, a fact and its role a line). */
let forgotten: string[] | undefined;
function forgettable(): string[] {
  return (forgotten ??= run([compiler, "--knowns"])
    .trim()
    .split("\n")
    .map((line) => line.split(" "))
    .filter(([, ...role]) => ["permits", "as written"].includes(role.join(" ")))
    .map(([name]) => name));
}

/** The std functions whose route without their recogniser is neither a
 * refusal nor right, with the issue each waits on: a trait's method of a
 * concrete receiver handed to generic code's route (#1477). No variant is
 * made of one. A list that only shrinks: a name leaves it with its fix. */
const unrecognizedDiffers: Record<string, string> = {
  // A counted `Rc`'s `as_ref()` through generic code, `Box<Rc<str>>`'s.
  "std::convert::AsRef::as_ref": "#1518",
};

/** A way to compile a program: its name, and rust-js's flags for it. */
type Variant = { name: string; flags: string[] };

/** A program's variants, of those of preparation's rewrites that changed
 * it, `fired`, the facts a reader found in it, `found`, and the std
 * functions a recogniser took in it, `recognized`: tidy stopped after each
 * stage; preparation without each of those, as one that changed nothing
 * leaves the JS as it is; each of those facts the JS is right without
 * forgotten, as one no reader found changes nothing; each of those
 * functions' recognisers left out; and tidy and preparation both at their
 * least, the last. */
function variants(fired: string[], found: string[], recognized: string[] = []): Variant[] {
  const all = preparePasses();
  return [
    ...stageEnds().map((end) => ({ name: `after ${end}`, flags: ["--tidy-until", end] })),
    ...fired.map((pass) => ({ name: `without preparation's ${pass}`, flags: ["--prepare-without", pass] })),
    ...(forgettable().filter((known) => found.includes(known))).map((known) => ({ name: `without knowing ${known}`, flags: ["--without-known", known] })),
    ...(recognized.filter((path) => unrecognizedDiffers[path] === undefined)).map((path) => ({ name: `without the recogniser of ${path}`, flags: ["--without-recognizer", path] })),
    {
      name: `after ${stageEnds()[0]}, without any of preparation's rewrites`,
      flags: ["--tidy-until", stageEnds()[0], ...all.flatMap((pass) => ["--prepare-without", pass])],
    },
  ];
}

// The cases whose least variant doesn't run as native Rust does, each
// with its issue: a list that only shrinks. Each case's JS as built is
// right; its least variant runs what a later pass takes away.
const leastDiffers: Record<string, string> = {};

/** A compile's JS, its map's line left out: what's run, and kept by. */
const code = (js: string) => readFileSync(js, "utf8").replace(/^\/\/# sourceMappingURL=.*$/m, "");

type Compiled = { js: string } | CompileError;

/** A case's compiles in this run, by its file, edition and flags: each
 * made once, the control test's of the cases' (#808). */
const compiles = new Map<string, Promise<Compiled>>();

/** `file` compiled into `dir` with `flags`, or as it already was in this
 * run, by the compiler this run built. */
function compile(file: string, dir: string, edition: string, flags: string[]): Promise<Compiled> {
  const key = [file, edition, ...flags].join("\0");
  let made = compiles.get(key);
  if (made === undefined) compiles.set(key, (made = compileJsAsync(file, dir, edition, false, {}, flags)));
  return made;
}

/** `file`'s finished JS, the rewrites of preparation's that changed it,
 * as `--prepare-fired` writes them into `dir`, the facts a reader found,
 * as `--knowns-found` does, and the std functions a recogniser took, as
 * `--recognizers-found` does. */
type Full = { compiled: Compiled; fired: string[]; found: string[]; recognized: string[] };
const fullCompiles = new Map<string, Promise<Full>>();
function compileFull(file: string, dir: string, edition: string): Promise<Full> {
  const key = [file, edition].join("\0");
  let made = fullCompiles.get(key);
  if (made === undefined) {
    const firedFile = join(dir, "fired.txt");
    const foundFile = join(dir, "found.txt");
    const recognizedFile = join(dir, "recognized.txt");
    const lines = (path: string) => readFileSync(path, "utf8").split("\n").filter(Boolean);
    const flags = ["--prepare-fired", firedFile, "--knowns-found", foundFile, "--recognizers-found", recognizedFile];
    made = compileJsAsync(file, dir, edition, false, {}, flags).then((compiled) => ({
      compiled,
      fired: "js" in compiled ? lines(firedFile) : [],
      found: "js" in compiled ? lines(foundFile) : [],
      recognized: "js" in compiled ? lines(recognizedFile) : [],
    }));
    fullCompiles.set(key, made);
  }
  return made;
}

/** A case's folder in this run, one for its compiles, which the control
 * test's are of: each JS named for its flags, so two differ by them alone. */
const caseDirs = new Map<string, string>();
function caseDir(name: string): string {
  const stem = basename(name, ".rs");
  let dir = caseDirs.get(stem);
  if (dir === undefined) caseDirs.set(stem, (dir = fixture(`stages-${stem}`)));
  return dir;
}

/** Each of `items` given to `f`, as many at once as there are cores. */
async function eachAtOnce<T, U>(items: T[], f: (item: T) => Promise<U>): Promise<U[]> {
  const results: U[] = new Array(items.length);
  let next = 0;
  const worker = async () => {
    for (let at = next++; at < items.length; at = next++) results[at] = await f(items[at]);
  };
  await Promise.all(Array.from({ length: Math.min(machine().cores, items.length) }, worker));
  return results;
}

/** How `file`'s variants differ from `native`, each a line or more, or
 * none: `final` is its finished JS's error, a variant's refusal its own.
 * Its variants are compiled at once, then run one after another. */
async function differences(file: string, dir: string, edition: string, native: Run, only?: string): Promise<string[]> {
  const problems: string[] = [];
  const { compiled: full, fired, found, recognized } = await compileFull(file, dir, edition);
  if (!("js" in full)) return [`rust-js failed: ${full.error.split("\n").slice(0, 6).join("\n")}`];
  const finalEnd = stageEnds().at(-1);
  const least = variants([], []).at(-1)!.name;
  const listed = leastDiffers[basename(file)];
  const chosen = variants(fired, found, recognized).filter((variant) => only === undefined || variant.name === only);
  const made = await eachAtOnce(chosen, ({ flags }) => compile(file, dir, edition, flags));
  for (const [at, { name, flags }] of chosen.entries()) {
    const compiled = made[at];
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
      // And a std function whose recogniser is left out is refused, as
      // rust-js has no other model of it: what's wanted of one (ADR 1128).
      const unrecognized = flags[0] === "--without-recognizer";
      if (compiled.kind === "rejected" && (tidied || unrecognized) && !unreadable) continue;
      problems.push(`${name}: rust-js failed: ${compiled.error.split("\n").slice(0, 6).join("\n")}`);
      continue;
    }
    // The same JS, of any variant, runs the same: it's run once.
    const key = runKey(["readability variant", code(compiled.js), native.bytes.stdout, native.bytes.stderr, show(native.outcome), shared()]);
    if (verified(key)) continue;
    const [runtime, cmd] = runtimes[0];
    const ran = runJs(cmd, compiled.js, dir, `${runtime}-${flags.join("-").replaceAll(/[^A-Za-z0-9]+/g, "_").slice(0, 80)}`, jsLimit(native));
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
  expect(ends).toContain("finish/guarded cleanups");
  expect(ends.some((end) => end.startsWith("again/"))).toBe(true);
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
    // All of them left out at once, as each that changed nothing leaves
    // every later one the same tree: one compile, not one each.
    const without = compileJs(file, dir, "2024", false, {}, unfired.flatMap((pass) => ["--prepare-without", pass]));
    if (!("js" in without)) throw new Error(without.error);
    expect([name, code(without.js) === full]).toEqual([name, true]);
  }
}, 300_000);

// A fact forgotten is one no reader finds, wherever it was said, so the
// variants above try what they say (ADR 1127): a case's JS without a fact
// that changes its form isn't its JS with it; what `--knowns-found` says is
// each fact a reader found, a form's too, and none that's forgotten; and a
// name that's no fact is refused.
test("a case's JS without a fact a reader found isn't its JS with it", async () => {
  buildCompiler();
  const name = "calculator.rs";
  const dir = caseDir(name);
  const { compiled, found } = await compileFull(join(corpus, name), dir, "2024");
  if (!("js" in compiled)) throw new Error(compiled.error);
  expect(found).toEqual(expect.arrayContaining(["ArrayItems", "Else", "Literal"]));
  expect(forgettable()).toEqual(expect.arrayContaining(["ArrayItems", "Else", "NeverThrows"]));
  // What warns, or is a form, is never forgotten by a variant.
  for (const fact of ["Getter", "Live", "Iterator", "InPlace", "IterLent"]) expect(forgettable()).not.toContain(fact);
  const foundFile = join(dir, "found-without.txt");
  const without = await compile(join(corpus, name), dir, "2024", ["--without-known", "ArrayItems", "--knowns-found", foundFile]);
  if (!("js" in without)) throw new Error(without.error);
  expect(code(without.js)).not.toBe(code(compiled.js));
  expect(readFileSync(foundFile, "utf8").split("\n")).not.toContain("ArrayItems");
  // Into the case's folder: one that compiled would write beside its source,
  // over the case's snapshot, as a mutant that takes any name did.
  const unknown = Bun.spawnSync([compiler, join(corpus, name), "-o", join(dir, "unknown.js"), "--without-known", "Nothing"]);
  expect(unknown.exitCode).not.toBe(0);
  expect(unknown.stderr.toString()).toContain("--without-known: no fact `Nothing`; `rust-js --knowns` lists them");
});

// A std function whose recogniser is left out is one no recogniser takes,
// in every body and analysis alike, so the variants above try what they
// say (ADR 1128): its call is refused, as rust-js has no other model of
// it; what `--recognizers-found` says is each function a recogniser took,
// and none that's left out; and one the list of known differences names
// has no variant.
test("a std function's call without its recogniser is refused, and what a compile recognised is listed", async () => {
  buildCompiler();
  const name = "calculator.rs";
  const dir = caseDir(name);
  const { compiled, recognized } = await compileFull(join(corpus, name), dir, "2024");
  if (!("js" in compiled)) throw new Error(compiled.error);
  const sqrt = "std::f64::<impl f64>::sqrt";
  expect(recognized).toEqual(expect.arrayContaining([sqrt, "std::string::String::push"]));
  const listed = join(dir, "recognized-without.txt");
  const without = await compile(join(corpus, name), dir, "2024", ["--without-recognizer", sqrt, "--recognizers-found", listed]);
  if ("js" in without) throw new Error(`${sqrt} left out, the case still compiled`);
  expect(without.kind).toBe("rejected");
  expect(without.error).toContain(`rust-js does not support calling \`${sqrt}\` yet`);
  expect(readFileSync(listed, "utf8").split("\n")).not.toContain(sqrt);
  const named = variants([], [], ["std::convert::AsRef::as_ref", sqrt]).map((variant) => variant.name);
  expect(named).toContain(`without the recogniser of ${sqrt}`);
  expect(named).not.toContain("without the recogniser of std::convert::AsRef::as_ref");
});

// A trait's method of std's impl, of a type known where it's called, that
// takes a `&mut` is refused without its recogniser too: it went by the trait
// call generic code makes, to a dictionary that reads a `&mut` as generic
// code's box, where the type known here gives its object, and ran otherwise:
// a `Hasher`'s writes, a `BorrowMut`'s and a reader's (#1487, #1477).
test("a known type's call that gives std's dictionary a &mut is refused without its recogniser", async () => {
  buildCompiler();
  for (const [name, path] of [
    ["hash_hashers.rs", "std::hash::Hasher::write"],
    ["borrow_mut_of_a_vec_as_a_slice.rs", "std::borrow::BorrowMut::borrow_mut"],
    ["io_buf_reader.rs", "std::io::Read::read"],
  ]) {
    const without = await compile(join(corpus, name), caseDir(name), "2024", ["--without-recognizer", path]);
    if ("js" in without) throw new Error(`${path} left out, ${name} still compiled`);
    expect(without.kind).toBe("rejected");
    expect(without.error).toContain(`rust-js does not support calling \`${path}\``);
  }
});

// A formatting trait's `fmt` of std's impl is never the trait call: its
// dictionary's entry gives the value's text, no `Formatter`, so what no
// recogniser takes is refused, never that call, which wrote into the held
// formatter's place what it gave back (#1477).
test("a formatting trait's fmt is refused without its recogniser, never the trait call", async () => {
  buildCompiler();
  const name = "fmt_to_held_formatter.rs";
  const path = "std::fmt::Debug::fmt";
  const without = await compile(join(corpus, name), caseDir(name), "2024", ["--without-recognizer", path]);
  if ("js" in without) throw new Error(`${path} left out, ${name} still compiled`);
  expect(without.kind).toBe("rejected");
  expect(without.error).toContain(`rust-js does not support calling \`${path}\``);
});

// What generic code bounded by `IntoIterator` steps is adapted by what it's
// given, its type, not by the recogniser of `into_iter`: a `&mut` to a
// sequence a handle on each item, `$mutItems(v)`, a `&mut Result`
// `iter_mut()`'s handle, each without that recogniser too (#1477).
test("a &mut given to an IntoIterator bound is adapted by its type, without into_iter's recogniser", async () => {
  buildCompiler();
  for (const name of ["iterables_given_to_generics.rs", "result_given_as_items.rs"]) {
    const file = join(corpus, name);
    const want = directives(readFileSync(file, "utf8"));
    if (typeof want === "string" || want.kind === "compile-fail" || want.kind === "ignore-rust-js") throw new Error(`${name}: no run`);
    const dir = caseDir(name);
    const native = runNative(file, dir, want.edition, want.native);
    if (typeof native === "string") throw new Error(native);
    const only = "without the recogniser of std::iter::IntoIterator::into_iter";
    expect((await differences(file, dir, want.edition, native, only)).join("\n\n")).toBe("");
  }
});

// A name that's no case is the test above's to report.
for (const name of chosen.filter((name) => cases.includes(name))) {
  const file = join(corpus, name);
  const want = directives(readFileSync(file, "utf8"));
  // What doesn't run as Rust does yet, or doesn't run, has no stage to
  // check; a malformed directive is the corpus's to report.
  if (typeof want === "string" || want.kind === "compile-fail" || want.kind === "ignore-rust-js") continue;
  test(`${basename(name, ".rs")} after each of tidy's stages and without each of preparation's rewrites`, async () => {
    const dir = caseDir(name);
    const native = runNative(file, dir, want.edition, want.native);
    if (typeof native === "string") throw new Error(native);
    expect((await differences(file, dir, want.edition, native)).join("\n\n")).toBe("");
  }, 600_000);
}

// What's checked is each stage's JS, not the finished JS each time: tidy
// stopped after its first stage leaves a body as no later stage does; and
// so is preparation's without a rewrite it made. After the cases, whose
// compiles of these it reads, made once (#808).
test("a case's JS after tidy's first stage isn't its JS after the last, nor without a rewrite that fired", async () => {
  buildCompiler();
  const [first, ...rest] = stageEnds();
  const at = async (name: string, flags: string[]) => {
    const compiled = await compile(join(corpus, name), caseDir(name), "2024", flags);
    if (!("js" in compiled)) throw new Error(compiled.error);
    // Its map is named for the stage, as its file is: the code's what's compared.
    return code(compiled.js);
  };
  const [firstStage, lastStage] = await Promise.all([
    at("conditional_regions.rs", ["--tidy-until", first]),
    at("conditional_regions.rs", ["--tidy-until", rest.at(-1)!]),
  ]);
  expect(firstStage).not.toBe(lastStage);
  // Of a case whose rewrites are made in place, a block's, and where
  // their conditions say, `undefined left out`'s.
  const { compiled, fired: each } = await compileFull(join(corpus, "interpreter.rs"), caseDir("interpreter.rs"), "2024");
  if (!("js" in compiled)) throw new Error(compiled.error);
  const full = code(compiled.js);
  expect(each).toEqual(expect.arrayContaining(["undefined left out", "elses after exits"]));
  const without = await eachAtOnce(each, (pass) => at("interpreter.rs", ["--prepare-without", pass]));
  for (const [i, pass] of each.entries()) expect([pass, without[i]]).not.toEqual([pass, full]);
});

/** A generated program's first variant that differs from native Rust, if
 * any: `undefined` of one rustc rejects or rust-js refuses, which
 * `fuzz.test.ts` judges. */
async function programDiffers(program: Program, seed: number, only?: string): Promise<string | undefined> {
  const dir = fixture(`stages-seed-${seed}`);
  const text = print(program, seed);
  const file = join(contentDirectory(text), "program.rs");
  writeWhole(file, text);
  const native = runNative(file, dir);
  if (typeof native === "string") return undefined;
  const [problem] = await differences(file, dir, "2024", native, only);
  return problem?.startsWith("rust-js failed") ? undefined : problem;
}

for (const seed of seeds) {
  test(`generated program ${seed} after each of tidy's stages and without each of preparation's rewrites`, async () => {
    const program = generate(seed);
    const problem = await programDiffers(program, seed);
    if (problem === undefined) return;
    // As small as the same variant still differs, kept as it's found.
    const variant = problem.slice(0, problem.indexOf(":"));
    const out = join(target, "fuzz");
    mkdirSync(out, { recursive: true });
    const repro = join(out, `seed-${seed}.stages.rs`);
    const keep = (p: Program) => writeFileSync(repro, print(p, seed));
    keep(program);
    const { program: smallest } = await reduce(program, async (candidate) => (await programDiffers(candidate, seed, variant)) !== undefined, {
      until: Date.now() + 300_000,
      kept: keep,
    });
    expect(`${problem}\n\nreduced from ${size(program)} statements to ${size(smallest)} in ${repro}:\n\n${print(smallest, seed)}`).toBe("");
  }, 900_000);
}
