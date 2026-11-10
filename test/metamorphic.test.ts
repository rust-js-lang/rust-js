// The metamorphic net (#939, `test/metamorphic.ts`): each corpus case, each
// rewrite `scripts/metamorph` makes of it, run natively and as JS in both
// builds. A rewrite rustc rejects of every place it finds is tried of its
// first alone; one rustc rejects even so is no variant. What runs otherwise
// than native, or crashes rust-js, is listed under its issue in
// test/metamorphic-known.txt; each listed one runs each night, so the list
// only shrinks.
//
//   bun test test/metamorphic.test.ts                                      # a push's sample
//   METAMORPHIC=all RUST_JS_CASES=1/6 bun test test/metamorphic.test.ts    # every case, a sixth
//   target/metamorph/debug/metamorph <rewrite> test/corpus/<case>.rs       # a rewrite, by hand

import { beforeAll, expect, test } from "bun:test";
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { corpus } from "./corpus-cases";
import { runSync } from "./child";
import { buildRewriter, libraryCrate, overlapping, readKnown, rewritable, rewrites, rewritten, sampled, type Rewrite } from "./metamorphic";
import { reducedRepro } from "./reduced";
import { agree, compileJs, compileJsLibrary, nativeFlags, runJs, runNative, runtimes, show, type Run } from "./programs";
import { buildCompiler, caseShard, fixture, root } from "./support";
import { generatedTexts, record, runKey, shared, verified } from "./verified-runs";
import { shard } from "../scripts/shard";
import { partSetting } from "./part-settings";

beforeAll(() => {
  buildCompiler();
  buildRewriter();
}, 900_000);

const which = partSetting("METAMORPHIC");
// `METAMORPHIC_REWRITES=emi-prune,emi-insert`: those rewrites alone, of a
// round of new ones; every one unless it's given.
const named = process.env.METAMORPHIC_REWRITES?.split(",").filter((name) => name !== "");
const rewritesRun = rewrites.filter((rewrite) => named === undefined || named.includes(rewrite));
if (named !== undefined && rewritesRun.length !== named.length) throw new Error(`METAMORPHIC_REWRITES=${named.join(",")}: of ${rewrites.join(", ")}`);
if (which !== undefined && which !== "all") throw new Error(`METAMORPHIC=${which}: \`all\`, or unset for a push's sample`);
const known = readKnown();
const all = rewritable();
// A push's run: the cases a hash of their name samples, one in 128; the
// nightly's, every case, each listed one's too. Of a list,
// `RUST_JS_CASES=i/n:<file>`, the cases it names, as the corpus's does:
// those a change adds or edits, each of their rewrites (`bun
// scripts/iteratorSlice.ts run metamorphic`, #1165).
const listed = (process.env.RUST_JS_CASES ?? "").includes(":");
const picked = which === "all" || listed ? all : all.filter(({ name }) => sampled(name));
const chosen = new Set(caseShard(picked.map(({ name }) => name)));

/** The net's own harness, its text, which a kept run's key holds as
 * `shared()` holds the corpus's. */
const netHarness = ["metamorphic.test.ts", "metamorphic.ts"].map((file) => readFileSync(join(import.meta.dir, file), "utf8"));

/** The two builds: the default, and std's iterator structs (ADR 0908). */
const builds: [string, string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** `METAMORPHIC_TALLY=<file>`: a line of what each rewrite of each case
 * made, `<rewrite> <case> <what>`, `unchanged`, `all` or `first` tried,
 * and `variant` where rustc compiled it, for a run's counts (ADR 1104). */
const tallied = process.env.METAMORPHIC_TALLY;
const tally = (rewrite: Rewrite, name: string, what: string) => {
  if (tallied) appendFileSync(tallied, `${rewrite} ${name} ${what}\n`);
};

/** What went wrong of `rewrite` of a case in each build, or nothing: its
 * JS run otherwise than native, or rust-js's crash. A refusal is safe, and
 * no wrong answer. */
function wrong(rewrite: Rewrite, name: string, edition: string, target: "host" | "wasm32"): { problems: Map<string, string>; program?: string } | undefined {
  const file = join(corpus, name);
  const dir = fixture(`metamorphic-${rewrite}-${name.replace(/\.rs$/, "")}`);
  const variant = join(dir, "variant.rs");
  const library = join(dir, "lib.rs");
  let native: Run | undefined;
  let made: ReturnType<typeof rewritten>;
  // Of every place, then of its first alone.
  for (const limit of [undefined, 1]) {
    made = rewritten(rewrite, file, edition, limit);
    tally(rewrite, name, made === undefined ? "unchanged" : limit === undefined ? "all" : "first");
    if (made === undefined) return undefined;
    writeFileSync(variant, made.program);
    // `library`'s program uses its library, built first.
    const rustc: string[] = [];
    if (made.library !== undefined) {
      writeFileSync(library, made.library);
      const rlib = join(dir, `lib${libraryCrate}.rlib`);
      // Of a case run natively on wasm32, its library built for it too.
      const triple = target === "wasm32" ? ["--target=wasm32-wasip1"] : [];
      const lib = runSync(["rustc", ...nativeFlags(edition), ...triple, "--crate-type=rlib", `--crate-name=${libraryCrate}`, library, "-o", rlib], root, 120_000);
      if (lib.code !== 0) continue;
      rustc.push("--extern", `${libraryCrate}=${rlib}`);
    }
    const ran = runNative(variant, dir, edition, target, rustc);
    // On wasm32 a panic aborts, with no outcome to judge the JS's by: no
    // variant, as one rustc rejects is.
    const unjudged = typeof ran !== "string" && target === "wasm32" && typeof ran.outcome === "string";
    if (typeof ran !== "string" && !unjudged && !overlapping(rewrite, ran.outcome)) {
      native = ran;
      tally(rewrite, name, "variant");
      break;
    }
  }
  if (native === undefined || made === undefined) return undefined;
  const problems = new Map<string, string>();
  for (const [build, flags] of builds) {
    const own = join(dir, build);
    mkdirSync(own, { recursive: true });
    let compiled: ReturnType<typeof compileJs>;
    if (made.library === undefined) {
      compiled = compileJs(variant, own, edition, false, {}, flags);
    } else {
      const lib = compileJsLibrary(library, own, edition, libraryCrate, flags);
      compiled = "rmeta" in lib
        ? compileJs(variant, own, edition, false, {}, [...flags, "--dependency", lib.manifest], ["--extern", `${libraryCrate}=${lib.rmeta}`, "-L", own])
        : lib;
    }
    if (!("js" in compiled)) {
      if (compiled.kind === "crashed") problems.set(build, `rust-js crashed: ${compiled.reason}`);
      continue;
    }
    // Its run, kept by what it ran (ADR 0567): the rewrite, the JS, what
    // native Rust printed and how it ended, what every case's runs share
    // and the net's harness. The same as a run that agreed before isn't run
    // again; the compile is, each time, as it's what's checked (#1047).
    const key = runKey([
      `metamorphic ${build}`,
      made.program,
      made.library ?? "",
      ...generatedTexts(own),
      native.bytes.stdout,
      native.bytes.stderr,
      show(native.outcome),
      shared(),
      ...netHarness,
    ]);
    if (verified(key)) continue;
    const [runtime, cmd] = runtimes[0];
    const ran = runJs(cmd, compiled.js, own, runtime);
    if (agree(ran, native)) {
      record(key, `${rewrite} ${name} ${build}`);
      continue;
    }
    const [a, b] = [native.stdout.split("\n"), ran.stdout.split("\n")];
    const at = a.findIndex((line, i) => line !== b[i]);
    problems.set(
      build,
      `ended ${show(ran.outcome)} where native ended ${show(native.outcome)}; native ${JSON.stringify(a.slice(at, at + 3))}, JS ${JSON.stringify(b.slice(at, at + 3))}${ran.stderr ? `; stderr ${JSON.stringify(ran.stderr.slice(0, 300))}` : ""}`,
    );
  }
  // The program that goes wrong, which a failure's reduced from (#1088):
  // not a `library` variant's, two crates.
  return { problems, program: made.library === undefined ? variant : undefined };
}

for (const { name, edition, native } of all.filter(({ name }) => chosen.has(name))) {
  test(`${name}, rewritten`, () => {
    const problems: string[] = [];
    for (const rewrite of rewritesRun) {
      const found = wrong(rewrite, name, edition, native);
      for (const [build] of builds) {
        const listedAs = known.find((k) => k.name === name && k.rewrite === rewrite && k.builds.includes(build));
        const problem = found?.problems.get(build);
        if (listedAs !== undefined && problem === undefined) {
          problems.push(`${build} ${rewrite} ${name} runs as Rust does now: take it off test/metamorphic-known.txt (#${listedAs.issue})`);
        } else if (listedAs === undefined && problem !== undefined) {
          // With its program reduced (#1088), where it's one crate.
          const repro = found?.program === undefined ? "" : reducedRepro(found.program, build, edition);
          problems.push(`${build} ${rewrite} ${name}: ${problem}${repro}`);
        }
      }
    }
    // What isn't listed: an issue to file, and `<builds> <rewrite> <case>`
    // to list under it.
    expect(problems.join("\n\n")).toBe("");
  }, 900_000);
}

// The harness's own parts.
test("a push's sample is stable: a case added changes only whether it's sampled", () => {
  const names = Array.from({ length: 2000 }, (_, i) => `c${i}.rs`);
  const sample = names.filter((name) => sampled(name));
  expect(sample.length).toBeGreaterThan(5);
  expect(sample.length).toBeLessThan(40);
  expect([...names, "added.rs"].filter((name) => sampled(name) && name !== "added.rs")).toEqual(sample);
});

// Equivalence modulo inputs (#1086): a block no run reaches made
// `unreachable!()`, what's reached kept; and code never run added.
const emiCase = `fn pick(n: u32) -> u32 {
    if n > 10 { n * 2 } else { n + 1 }
}

fn unused() -> u32 {
    7
}

fn main() {
    let mut total = pick(3);
    total += 1;
    println!("{total}");
}
`;

test("emi-prune makes each block no run reaches `unreachable!()`, and keeps what one reaches", () => {
  const dir = fixture("metamorphic-emi-prune-harness");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "case.rs");
  writeFileSync(file, emiCase);
  const made = rewritten("emi-prune", file, "2024");
  expect(made?.program.match(/unreachable\s*!/g)?.length).toBe(2);
  expect(made?.program).toContain("n + 1");
  expect(made?.program).not.toContain("n * 2");
  expect(made?.program).not.toMatch(/\b7\b/);
});

test("emi-insert adds code never run: a borrow after each `let`, and a call of each function that takes nothing", () => {
  const dir = fixture("metamorphic-emi-insert-harness");
  mkdirSync(dir, { recursive: true });
  const file = join(dir, "case.rs");
  writeFileSync(file, emiCase);
  const program = rewritten("emi-insert", file, "2024")?.program.replaceAll(/\s+/g, " ") ?? "";
  expect(program).toContain("if std :: hint :: black_box (false) { let _emi = & mut total ; }");
  expect(program).toContain("if std :: hint :: black_box (false) { let _emi = unused () ; }");
  expect(program).not.toContain("_emi = main ()");
});

// A case that compares functions is no `library` variant, as it's no
// `closure-fn` one: a generic function's instance is one function in no
// promise of Rust's across crates (ADR 0395, amended, #987, #1027). One that
// compares what Rust does promise, two `Rc`s, is.
test("a case that compares functions is moved into no library", () => {
  expect(rewritten("library", join(corpus, "fn_addr_eq_compares_functions.rs"), "2024")).toBeUndefined();
  expect(rewritten("library", join(corpus, "generic_fn_value_is_one_function.rs"), "2024")).toBeUndefined();
  expect(rewritten("library", join(corpus, "rc_fn_ptr_eq_of_cloned.rs"), "2024")).toBeDefined();
});

test("a listed rewrite is read under its issue or its decision, and a malformed line is refused", () => {
  expect(readKnown("#7 P0: a title\nplain std boxed a.rs\n")).toEqual([{ issue: "7", builds: ["plain", "std"], rewrite: "boxed", name: "a.rs" }]);
  // A difference the user decided, under its decision's id (ADR 0262).
  expect(readKnown("#rd360 a decision\nplain library a.rs\n")).toEqual([{ issue: "rd360", builds: ["plain"], rewrite: "library", name: "a.rs" }]);
  expect(() => readKnown("#rdx t\nplain boxed a.rs\n")).toThrow();
  expect(() => readKnown("plain boxed a.rs\n")).toThrow();
  expect(() => readKnown("#7 t\nplain inlined a.rs\n")).toThrow();
});
