// The corpus harness's own mutants (#918): compilers that get one thing
// wrong in what they give the corpus, each a wrapper of the compiler as it
// is, which its harness, `test/corpus.test.ts` and what it runs, must
// report. A change to the harness runs these, not every mutation of the
// compiler that names a corpus case, which needed each built again (34 min
// for 5,805), to show the harness still catches each kind of wrong JS: no
// build, each mutant on one case of each set of directives the corpus has,
// in each mode a mutation's run is (ADR 0093).
//
//   bun test/corpus-mutants.ts <mutant> <compiler> <its arguments>   as the compiler

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { cases, corpus, directives, type Directives } from "./corpus-cases";

const root = join(import.meta.dir, "..");

/** Which of the corpus's runs a JS edit is for: the binary's under Node,
 * the binary shipped, bundled and minified, the program's or the
 * library's (#2447). Each is told apart by where its JS is, so a binary's
 * edit goes in every JS the compiler writes, and the program's and the
 * library's JS are what they were beside it. A program's or a library's
 * goes in its JS alone: the harness runs a program's JS only where it
 * isn't its binary's, and a library's where it isn't its program's, and
 * says where that's unsaid. */
const runs = {
  node: `!import.meta.url.endsWith(".min.js") && !import.meta.url.endsWith("/program/case.js") && !import.meta.url.endsWith("/library/case.js")`,
  minified: `import.meta.url.endsWith(".min.js")`,
  program: `import.meta.url.endsWith("/program/case.js")`,
  library: `import.meta.url.endsWith("/library/case.js")`,
};

/** A build a mutant's edit is of alone: the program's or the library's. */
type Alone = "program" | "library";

/** What a JS edit gets wrong, in the run it's for: what it prints to
 * either stream, or how it ends. */
const wrongs = {
  stdout: `console.log("a mutant's line");`,
  stderr: `console.error("a mutant's line");`,
  outcome: `process.on("exit", () => { process.exitCode = 3; });`,
};

/** How a mutant's compiler differs from the compiler as it is. */
type Edit =
  /** The JS it writes, with `js` at its start: of one build alone, `alone`. */
  | { js: string; alone?: Alone }
  /** It crashes, or refuses, as a binary, or as one build alone. */
  | { exit: "crashes" | "refuses"; alone?: Alone }
  /** It compiles what rustc or rust-js refuse, writing an empty module. */
  | { exit: "compiles" }
  /** It refuses what it refuses with another error. */
  | { exit: "refuses-otherwise" };

/** A mutation's run of the corpus: its snapshots ignored, as most
 * mutations' (`false`), checked (`true`), or alone (`"alone"`, of one its
 * tests miss that is of how the JS reads, #864). As `Snapshots` in
 * scripts/mutations.ts. */
export type Mode = boolean | "alone";

export type CorpusMutant = {
  name: string;
  breaks: string;
  edit: Edit;
  /** The kinds of case it's a wrong answer of. */
  of: "runs" | "compile-fail";
  mode: Mode;
  /** Whether the harness must catch it, or must not: a mutant of only how
   * the JS reads is no wrong answer where a mutation's run ignores the
   * snapshots, and caught there it would say every mutation is. */
  caught: boolean;
};

const reads = { js: "// a mutant's comment" };
/** A program's JS that reads otherwise past what the wrapper adds: a
 * comment would be its binary's still (`asBinary`). */
const programReads = { js: `void "a mutant's statement";`, alone: "program" as const };

/** Each mutant, by its name. */
export const corpusMutants: CorpusMutant[] = [
  ...Object.entries(runs).flatMap(([run, when]) =>
    Object.entries(wrongs).map(([wrong, js]): CorpusMutant => ({
      name: `${run}-${wrong}`,
      breaks: `the ${run} run's ${wrong === "outcome" ? "exit status" : wrong} isn't native Rust's`,
      edit: { js: `if (${when}) { ${js} }`, alone: run === "program" || run === "library" ? run : undefined },
      of: "runs",
      mode: false,
      caught: true,
    })),
  ),
  { name: "crashes", breaks: "rust-js crashes on a program it compiles", edit: { exit: "crashes" }, of: "runs", mode: false, caught: true },
  { name: "refuses", breaks: "rust-js refuses a program it compiles", edit: { exit: "refuses" }, of: "runs", mode: false, caught: true },
  { name: "refuses-a-program", breaks: "rust-js refuses as a program a binary it compiles", edit: { exit: "refuses", alone: "program" }, of: "runs", mode: false, caught: true },
  { name: "refuses-a-library", breaks: "rust-js refuses as a library a program it compiles as one", edit: { exit: "refuses", alone: "library" }, of: "runs", mode: false, caught: true },
  { name: "reads-otherwise-ignored", breaks: "the JS reads otherwise, which a mutation's run ignores", edit: reads, of: "runs", mode: false, caught: false },
  { name: "reads-otherwise", breaks: "the JS reads otherwise, and isn't its snapshot", edit: reads, of: "runs", mode: true, caught: true },
  { name: "reads-otherwise-alone", breaks: "the JS reads otherwise, and isn't its snapshot, judged alone", edit: reads, of: "runs", mode: "alone", caught: true },
  { name: "program-reads-otherwise", breaks: "a program's JS reads otherwise, and isn't its snapshot (ADR 1198, amended)", edit: programReads, of: "runs", mode: true, caught: true },
  { name: "program-reads-otherwise-alone", breaks: "a program's JS reads otherwise, judged by snapshots alone", edit: programReads, of: "runs", mode: "alone", caught: true },
  { name: "crashes-alone", breaks: "rust-js crashes on a program, judged by snapshots alone", edit: { exit: "crashes" }, of: "runs", mode: "alone", caught: true },
  { name: "compiles", breaks: "rust-js compiles a program it can't", edit: { exit: "compiles" }, of: "compile-fail", mode: false, caught: true },
  { name: "crashes-refusing", breaks: "rust-js crashes where it refuses", edit: { exit: "crashes" }, of: "compile-fail", mode: false, caught: true },
  { name: "refuses-otherwise", breaks: "rust-js refuses with another error", edit: { exit: "refuses-otherwise" }, of: "compile-fail", mode: false, caught: true },
];

/** The cases each mutant runs on: the first of each set of directives the
 * corpus has, its kind, edition, native target and library's, as each is
 * a way the harness runs a case. */
export function corpusMutantCases(): { name: string; want: Directives }[] {
  const firsts = new Map<string, { name: string; want: Directives }>();
  for (const name of cases) {
    const want = directives(readFileSync(join(corpus, name), "utf8"));
    if (typeof want === "string") continue;
    const key = JSON.stringify([want.kind, want.edition, want.native, want.libraryDiffers, want.libraryRefused !== undefined]);
    if (!firsts.has(key)) firsts.set(key, { name, want });
  }
  return [...firsts.values()];
}

/** Whether `mutant` is a wrong answer of a case that `want`s so: not one
 * of the library's run of a case whose library run is refused or differs,
 * as its directives say, where no library's output is compared (#1058). */
export const mutantOf = (mutant: CorpusMutant, want: Directives) =>
  !("alone" in mutant.edit && mutant.edit.alone === "library" && (want.libraryRefused !== undefined || want.libraryDiffers)) &&
  (mutant.of === "compile-fail" ? want.kind === "compile-fail" : want.kind === "run-pass" || want.kind === "run-fail" || want.kind === "run-abort");

/** The corpus harness's files: `test/corpus.test.ts` and what it imports,
 * each file of the repository's it imports in turn, and each of `test/`
 * one of them runs by its path, as `test/corpus-run.ts`; and these mutants'
 * own. A change to any runs the mutants. */
export function corpusHarness(): string[] {
  const found = new Set<string>();
  const transpiler = new Bun.Transpiler({ loader: "ts" });
  const visit = (file: string) => {
    if (found.has(file)) return;
    let text: string;
    try {
      text = readFileSync(join(root, file), "utf8");
    } catch {
      return;
    }
    found.add(file);
    for (const { path } of transpiler.scanImports(text)) {
      if (!path.startsWith(".")) continue;
      const to = resolve(dirname(join(root, file)), path).slice(root.length + 1);
      visit(/\.[cm]?[jt]s$/.test(to) ? to : `${to}.ts`);
    }
    for (const [, path] of text.matchAll(/["'`](test\/[\w./-]+\.[jt]s)["'`]/g)) if (!path.endsWith(".test.ts")) visit(path);
  };
  visit("test/corpus.test.ts");
  visit("test/corpus-mutants.ts");
  return [...found].sort();
}

/** What a compile printed and how it ended. */
type Wrapped = { code: number; stdout: string; stderr: string };

/** The wrapper's run: the compiler, `compiler`, given `args`, as `mutant`. */
function wrap(mutant: CorpusMutant, compiler: string, args: string[]): Wrapped {
  const out = args.indexOf("-o") >= 0 ? args[args.indexOf("-o") + 1] : undefined;
  // Which build it is: a library's, `--library`, a program's, in its
  // folder, or a binary's.
  const build = args.includes("--library") ? "library" : out?.includes("/program/") ? "program" : "binary";
  const { edit } = mutant;
  const ofIt = !("alone" in edit) || edit.alone === undefined || edit.alone === build;
  // What isn't a compile, as `--runtime-module`, is the compiler's own.
  if (out !== undefined && "exit" in edit && ofIt) {
    if (edit.exit === "crashes") return { code: 101, stdout: "", stderr: "thread 'rustc' panicked at src/main.rs:1:1:\na mutant's crash\n" };
    if (edit.exit === "refuses") return { code: 1, stdout: "", stderr: "error: a mutant's refusal\n" };
    if (edit.exit === "compiles") {
      writeFileSync(out, "export function entry() {}\n");
      return { code: 0, stdout: "", stderr: "" };
    }
  }
  const p = Bun.spawnSync([compiler, ...args], { stdout: "pipe", stderr: "pipe", stdin: "inherit" });
  const [stdout, stderr, code] = [p.stdout.toString(), p.stderr.toString(), p.exitCode ?? 1];
  if (out !== undefined && "exit" in edit && edit.exit === "refuses-otherwise" && code !== 0) {
    return { code, stdout, stderr: stderr.replace(/^error(\[E\d+\])?:.*$/gm, "error: a mutant's other error") };
  }
  // First in the module, run before anything of its own: a binary's JS
  // calls `$main(main)` near its top, which ends the process at once where
  // `main` fails or aborts, so an edit at its end never ran there (#2447's
  // first CI run, worker-02 agent's read).
  if (out !== undefined && "js" in edit && ofIt && code === 0) writeFileSync(out, `${edit.js}\n${readFileSync(out, "utf8")}`);
  return { code, stdout, stderr };
}

/** A worker's job, a line of `--worker`'s (ADR 1182), each build compiled
 * alone as `mutant`, so a mutant's wrong answer reaches the harness by its
 * jobs as by its compiles. */
function wrapJob(mutant: CorpusMutant, compiler: string, line: string): string {
  const job = JSON.parse(line) as { builds: string[][] };
  return `${JSON.stringify({ builds: job.builds.map((args) => wrap(mutant, compiler, args)) })}\n`;
}

if (import.meta.main) {
  const [name, compiler, ...args] = process.argv.slice(2);
  const mutant = corpusMutants.find((m) => m.name === name);
  if (!mutant) throw new Error(`no corpus mutant ${name}`);
  // Written whole before it exits: a pipe's `write` may not be, and the
  // runtime's module, `--runtime-module`, is long.
  // Each job answered as it comes: the harness keeps its worker, whose
  // stdin ends only with the test process.
  if (args.length === 1 && args[0] === "--worker") {
    for await (const line of console) {
      if (line.trim() !== "") await Bun.write(Bun.stdout, wrapJob(mutant, compiler, line));
    }
    process.exit(0);
  }
  const wrapped = wrap(mutant, compiler, args);
  await Bun.write(Bun.stdout, wrapped.stdout);
  await Bun.write(Bun.stderr, wrapped.stderr);
  process.exit(wrapped.code);
}
