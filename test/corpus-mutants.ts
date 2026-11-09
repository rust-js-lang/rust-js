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

/** Which of the corpus's runs a JS edit is for: the program's under Node,
 * the program shipped, bundled and minified, or the library's. Each is told
 * apart by where its JS is, so a program's edit goes in every JS the
 * compiler writes, and the library's JS is its program's where it was. A
 * library's goes in its JS alone: the harness runs a library's JS only
 * where it isn't its program's, and says where that's unsaid. */
const runs = {
  node: `!import.meta.url.endsWith(".min.js") && !import.meta.url.endsWith("/library/case.js")`,
  minified: `import.meta.url.endsWith(".min.js")`,
  library: `import.meta.url.endsWith("/library/case.js")`,
};

/** What a JS edit gets wrong, in the run it's for: what it prints to
 * either stream, or how it ends. */
const wrongs = {
  stdout: `console.log("a mutant's line");`,
  stderr: `console.error("a mutant's line");`,
  outcome: `process.on("exit", () => { process.exitCode = 3; });`,
};

/** How a mutant's compiler differs from the compiler as it is. */
type Edit =
  /** The JS it writes, with `js` at its end: as a library alone, `library`. */
  | { js: string; library?: boolean }
  /** It crashes, or refuses, as a program, or as a library alone. */
  | { exit: "crashes" | "refuses"; library?: boolean }
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

/** Each mutant, by its name. */
export const corpusMutants: CorpusMutant[] = [
  ...Object.entries(runs).flatMap(([run, when]) =>
    Object.entries(wrongs).map(([wrong, js]): CorpusMutant => ({
      name: `${run}-${wrong}`,
      breaks: `the ${run} run's ${wrong === "outcome" ? "exit status" : wrong} isn't native Rust's`,
      edit: { js: `if (${when}) { ${js} }`, library: run === "library" },
      of: "runs",
      mode: false,
      caught: true,
    })),
  ),
  { name: "crashes", breaks: "rust-js crashes on a program it compiles", edit: { exit: "crashes" }, of: "runs", mode: false, caught: true },
  { name: "refuses", breaks: "rust-js refuses a program it compiles", edit: { exit: "refuses" }, of: "runs", mode: false, caught: true },
  { name: "refuses-a-library", breaks: "rust-js refuses as a library a program it compiles as one", edit: { exit: "refuses", library: true }, of: "runs", mode: false, caught: true },
  { name: "reads-otherwise-ignored", breaks: "the JS reads otherwise, which a mutation's run ignores", edit: reads, of: "runs", mode: false, caught: false },
  { name: "reads-otherwise", breaks: "the JS reads otherwise, and isn't its snapshot", edit: reads, of: "runs", mode: true, caught: true },
  { name: "reads-otherwise-alone", breaks: "the JS reads otherwise, and isn't its snapshot, judged alone", edit: reads, of: "runs", mode: "alone", caught: true },
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
  !("library" in mutant.edit && mutant.edit.library && (want.libraryRefused !== undefined || want.libraryDiffers)) &&
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

/** The wrapper's run: the compiler, `compiler`, given `args`, as `mutant`. */
async function wrap(mutant: CorpusMutant, compiler: string, args: string[]): Promise<number> {
  const out = args.indexOf("-o") >= 0 ? args[args.indexOf("-o") + 1] : undefined;
  const library = args.includes("--library");
  const { edit } = mutant;
  // What isn't a compile, as `--runtime-module`, is the compiler's own.
  if (out !== undefined && "exit" in edit && (!("library" in edit && edit.library) || library)) {
    if (edit.exit === "crashes") {
      process.stderr.write("thread 'rustc' panicked at src/main.rs:1:1:\na mutant's crash\n");
      return 101;
    }
    if (edit.exit === "refuses") {
      process.stderr.write("error: a mutant's refusal\n");
      return 1;
    }
    if (edit.exit === "compiles") {
      writeFileSync(out, "export function entry() {}\n");
      return 0;
    }
  }
  const p = Bun.spawnSync([compiler, ...args], { stdout: "inherit", stderr: "pipe", stdin: "inherit" });
  const stderr = p.stderr.toString();
  if (out !== undefined && "exit" in edit && edit.exit === "refuses-otherwise" && p.exitCode !== 0) {
    process.stderr.write(stderr.replace(/^error(\[E\d+\])?:.*$/gm, "error: a mutant's other error"));
    return p.exitCode ?? 1;
  }
  process.stderr.write(stderr);
  if (out !== undefined && "js" in edit && (!edit.library || library) && p.exitCode === 0) writeFileSync(out, `${readFileSync(out, "utf8")}\n${edit.js}\n`);
  return p.exitCode ?? 1;
}

if (import.meta.main) {
  const [name, compiler, ...args] = process.argv.slice(2);
  const mutant = corpusMutants.find((m) => m.name === name);
  if (!mutant) throw new Error(`no corpus mutant ${name}`);
  process.exit(await wrap(mutant, compiler, args));
}
