// The object model's net (#891): each corpus case's items moved into a
// library (`scripts/metamorph`'s `library`), and the program run with
// each value the library gives it read through a counting `Proxy`
// (`object-model-run.ts`). A library's result is a value JS may have
// made, whose properties a getter may answer (ADR 1022), so the program
// reads it, writes it, tests its keys and calls its methods, with the
// `this` each is given, as Rust does whatever its readability passes do:
// its log, built with tidy stopped after its first stage and none of
// preparation's rewrites (the reads closest to MIR's, which are Rust's),
// is the oracle of its log as built, as ADR 1087's read cases are of
// theirs. Each run is native Rust's too, so a proxy JS can't see through
// shows. Where the logs differ, each of tidy's stages and each rewrite
// that fired is tried, to name what moved the read.
//
//   lib.rs + program.rs ── rust-js (least, as built) ──► node, the library's values proxied ──► the same log? native's output?
//
//   bun test test/object-model.test.ts                                     # a push's sample
//   OBJECT_MODEL=all RUST_JS_CASES=1/12 bun test test/object-model.test.ts  # every case, a twelfth

import { beforeAll, expect, test } from "bun:test";
import { appendFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { buildRewriter, libraryCrate, rewritable, rewritten, sampled } from "./metamorphic";
import { agree, compileJs, compileJsLibrary, execute, nativeFlags, node, runJs, runNative, runtimes, show, type Run } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";
import { shard } from "../scripts/shard";

beforeAll(() => {
  buildCompiler();
  buildRewriter();
}, 900_000);

const which = process.env.OBJECT_MODEL;
if (which !== undefined && which !== "all") throw new Error(`OBJECT_MODEL=${which}: \`all\`, or unset for a push's sample`);

/** What runs otherwise than it should, under its issue: `<issue> <case>`
 * a line, which only shrinks. */
const knownFile = join(import.meta.dir, "object-model-known.txt");
export function readKnown(text = existsSync(knownFile) ? readFileSync(knownFile, "utf8") : ""): { issue: string; name: string }[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("//"))
    .map((line) => {
      const [issue, name] = line.split(/\s+/);
      return { issue: issue.replace(/^#/, ""), name };
    });
}
const known = readKnown();

const all = rewritable();
// A push's run: one case in 64 by a hash of its name, and each listed one;
// the nightly's, every case.
const picked = which === "all" ? all : all.filter(({ name }) => sampled(name, 64) || known.some((k) => k.name === name));
const [part, parts] = (process.env.RUST_JS_CASES ?? "1/1").split("/").map(Number);
const chosen = new Set(shard(picked.map(({ name }) => name), part, parts));

/** The last pass of each of tidy's stages, `--tidy-until`'s. */
let ends: string[] | undefined;
function stageEnds(): string[] {
  if (ends) return ends;
  const last = new Map<string, string>();
  for (const pass of run([compiler, "--tidy-passes"]).trim().split("\n")) last.set(pass.slice(0, pass.lastIndexOf("/")), pass);
  ends = [...last.values()];
  return ends;
}
let rewrites: string[] | undefined;
const everyRewrite = () => (rewrites ??= run([compiler, "--prepare-passes"]).trim().split("\n"));

type Logged = { run: Run; log: string };

/** `OBJECT_MODEL_TALLY=<file>`: a line of what became of each case, for a
 * run's counts: `<case> <what>`, `unchanged`, `refused`, `ran <lines>`. */
const tallied = process.env.OBJECT_MODEL_TALLY;
const tally = (name: string, what: string) => {
  if (tallied) appendFileSync(tallied, `${name} ${what}\n`);
};

/** `js`, the program, run with the library's values proxied. */
function membraned(js: string, libraryJs: string, dir: string, name: string): Logged {
  const outcomeFile = join(dir, `${name}.outcome.json`);
  const logFile = join(dir, `${name}.log`);
  const ran = execute([node ?? "node", join(root, "test", "object-model-run.ts"), js, libraryJs, outcomeFile, logFile], outcomeFile);
  return { run: ran, log: existsSync(logFile) ? readFileSync(logFile, "utf8") : "" };
}

/** What's wrong of a case, or nothing: a variant run otherwise than
 * native, or reading what the library gave otherwise than the least. */
function wrong(name: string, edition: string): string | undefined {
  const file = join(root, "test", "corpus", name);
  const dir = fixture(`object-model-${name.replace(/\.rs$/, "")}`);
  const programFile = join(dir, "program.rs");
  // Not `lib.rs`, the name of the wrapper `compileJs` writes.
  const libraryFile = join(dir, "library.rs");
  let native: Run | undefined;
  // Of every place, then of its first alone, as the metamorphic net tries.
  for (const limit of [undefined, 1]) {
    const made = rewritten("library", file, edition, limit);
    if (made?.library === undefined) return void tally(name, "unchanged");
    writeFileSync(programFile, made.program);
    writeFileSync(libraryFile, made.library);
    const rlib = join(dir, `lib${libraryCrate}.rlib`);
    const lib = runSync(["rustc", ...nativeFlags(edition), "--crate-type=rlib", `--crate-name=${libraryCrate}`, libraryFile, "-o", rlib], root, 120_000);
    if (lib.code !== 0) continue;
    const ran = runNative(programFile, dir, edition, "host", ["--extern", `${libraryCrate}=${rlib}`]);
    if (typeof ran !== "string") {
      native = ran;
      break;
    }
  }
  if (native === undefined) return void tally(name, "no native run");
  const lib = compileJsLibrary(libraryFile, dir, edition, libraryCrate);
  // A refusal is safe; the metamorphic net holds a crash.
  if (!("rmeta" in lib)) return void tally(name, "library refused");
  const libraryJs = join(dir, libraryCrate, "lib.js");
  const program = (flags: string[]) =>
    compileJs(programFile, dir, edition, false, {}, [...flags, "--dependency", lib.manifest], ["--extern", `${libraryCrate}=${lib.rmeta}`, "-L", dir]);
  const firedFile = join(dir, "fired.txt");
  const built = program(["--prepare-fired", firedFile]);
  if (!("js" in built)) return void tally(name, "program refused");
  const ends = stageEnds();
  const least = program(["--tidy-until", ends[0], ...everyRewrite().flatMap((pass) => ["--prepare-without", pass])]);
  if (!("js" in least)) return `the least JS doesn't compile: ${least.error.split("\n").slice(0, 4).join("\n")}`;
  const want = membraned(least.js, libraryJs, dir, "least");
  const got = membraned(built.js, libraryJs, dir, "built");
  tally(name, `ran ${want.log === "" ? 0 : want.log.split("\n").length}`);
  const problems: string[] = [];
  for (const [variant, ran, js] of [
    ["least", want, least.js],
    ["as built", got, built.js],
  ] as const) {
    if (agree(ran.run, native)) continue;
    // Run as it is, without the proxies: one that's wrong so too is the
    // metamorphic net's to hold (its `library` rewrite), not this one's.
    const plain = runJs(runtimes[0][1], js, dir, `${variant.replace(" ", "-")}-plain`);
    if (!agree(plain, native)) {
      tally(name, `${variant} wrong unproxied`);
      continue;
    }
    const [a, b] = [native.stdout.split("\n"), ran.run.stdout.split("\n")];
    const at = a.findIndex((line, i) => line !== b[i]);
    problems.push(
      `${variant}, its library's values proxied, ended ${show(ran.run.outcome)} where native ended ${show(native.outcome)}; native ${JSON.stringify(a.slice(at, at + 3))}, JS ${JSON.stringify(b.slice(at, at + 3))}${ran.run.stderr ? `; stderr ${JSON.stringify(ran.run.stderr.slice(0, 400))}` : ""}`,
    );
  }
  if (problems.length > 0) return problems.join("\n");
  if (!agree(want.run, native) || !agree(got.run, native)) return undefined;
  if (got.log === want.log) return undefined;
  // The same reads in another order: the least is no oracle of order, as
  // it reads a value `format_args!` shows before the arguments after it are
  // made, where Rust and the JS as built make them all first; tallied.
  const sorted = (log: string) => log.split("\n").sort().join("\n");
  if (sorted(got.log) === sorted(want.log)) return void tally(name, "reordered");
  // Which of tidy's stages, and which of preparation's rewrites, read
  // otherwise than the least.
  const fired = existsSync(firedFile) ? readFileSync(firedFile, "utf8").split("\n").filter(Boolean) : [];
  // The first of tidy's stages after which it reads otherwise, and each
  // of preparation's rewrites without which it reads as the least.
  const read = (flags: string[], as: string): string | undefined => {
    const js = program(flags);
    return "js" in js ? membraned(js.js, libraryJs, dir, as.replaceAll(/[^A-Za-z0-9]+/g, "-")).log : undefined;
  };
  const stage = ends.slice(1).find((end) => end === ends.at(-1) || read(["--tidy-until", end], `after ${end}`) !== want.log);
  const rewrites = fired.filter((pass) => read(["--prepare-without", pass], `without ${pass}`) === want.log);
  const moved = [
    `first otherwise after tidy's ${stage}`,
    ...(rewrites.length > 0 ? [`as the least without preparation's ${rewrites.join(", ")}`] : []),
  ];
  const [a, b] = [want.log.split("\n"), got.log.split("\n")];
  const first = a.findIndex((line, i) => line !== b[i]);
  return `read otherwise than the least, from line ${first + 1}: least ${JSON.stringify(a.slice(first, first + 4))}, as built ${JSON.stringify(b.slice(first, first + 4))}\n${moved.join("\n")}`;
}

for (const { name, edition } of all.filter(({ name }) => chosen.has(name))) {
  test(`${name}, its library's values proxied`, () => {
    const problem = wrong(name, edition);
    const listed = known.find((k) => k.name === name);
    if (listed !== undefined && problem === undefined) {
      throw new Error(`${name} runs as Rust does now: take it off test/object-model-known.txt (#${listed.issue})`);
    }
    if (listed === undefined && problem !== undefined) throw new Error(`${name}: ${problem}`);
  }, 900_000);
}

test("a push's sample is some cases, a nightly's all", () => {
  expect(all.length).toBeGreaterThan(1000);
  expect(all.filter(({ name }) => sampled(name, 64)).length).toBeGreaterThan(10);
});

test("the known list reads its lines", () => {
  expect(readKnown("// a comment\n#1 a.rs\n\n#22  b.rs\n")).toEqual([
    { issue: "1", name: "a.rs" },
    { issue: "22", name: "b.rs" },
  ]);
});

