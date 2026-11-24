// Where each mutation reaches the corpus (#1979's sweep): its mutant's JS
// beside the compiler's as it is, case by case, in the main build and, with
// `--std-iterators`, std's iterator structs' too; each case whose JS
// differs then run, both, to say whether it behaves otherwise. Its answer,
// of each mutation: the cases it reaches, and a target its `tests` can
// name, so a survivor whose own case no longer reaches it is retargeted,
// and one that reaches none is a retirement to ask `--retire` of. Of the
// nightly's 17 survivors on 2026-11-25, seven were caught by retargeting
// alone, each found by this sweep run by hand.
//
//   bun scripts/mutation-reach.ts <name>.. [--names=<file>] [--cases=<file>]
//       [--std-iterators] [--limit=<s>]
//
// Its mutants are built as a mutation run builds them (`site`, `batches`,
// `schema`), in a workspace of its own, `target/mutation-reach`, never the
// checkout's sources. A mutation of the runtime changes no compiled JS: its
// reach is what runs, which `mutations.ts` judges, so this says so of it
// and sweeps none. Heavy: it takes the machine's turn (`holdLease`), and
// `--limit` bounds its run once it has it.
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { batches, crateFiles, mutations, schema, site, syncTree, type Mutation, type Sited } from "./mutations";
import { holdLease } from "./leases";
import { machine } from "./shard";
import { root } from "../test/support";
import { run, type Exit } from "../test/child";
import { compileCommand, node } from "../test/programs";
import { directives } from "../test/corpus-cases";

/** How a case answers to a mutant, of one build: it compiles otherwise
 * (`compile`), a refusal or a crash where the compiler as it is compiled
 * it, or the other way; its JS differs and it runs otherwise (`behaves`);
 * or its JS alone differs (`form`), which a snapshot holds. */
export type Reach = "compile" | "behaves" | "form";

/** Of a mutation, each case it reaches, by build, and how. */
export type Reached = { mutation: string; cases: { name: string; build: string; reach: Reach }[] };

/** The JS as it's compared: its source map's line, which names its file,
 * left out. */
export const jsCompared = (js: string) => js.replace(/^\/\/# sourceMappingURL=.*$/m, "");

/** What `tests` runs of the corpus: its `-t` pattern, where it runs
 * test/corpus.test.ts; `undefined` where it runs no corpus case, `null`
 * where it runs every one. */
export function corpusPattern(tests: string[] | undefined): RegExp | null | undefined {
  if (!tests?.includes("test/corpus.test.ts")) return undefined;
  const at = tests.indexOf("-t");
  return at < 0 ? null : new RegExp(tests[at + 1]);
}

/** The case `tests` should name, of those `reached`: one it runs
 * otherwise first, then one it compiles otherwise, each the smallest
 * file's, as it runs fastest; else one whose JS alone differs, held by
 * its snapshot (`snapshots: true`). `undefined` where it reaches none. */
export function retarget(reached: Reached["cases"], size: (name: string) => number): { tests: string[]; snapshots: boolean } | undefined {
  for (const reach of ["behaves", "compile", "form"] as const) {
    const of = reached.filter((c) => c.reach === reach && c.build === "main");
    if (of.length === 0) continue;
    const [best] = [...new Set(of.map((c) => c.name))].sort((a, b) => size(a) - size(b) || a.localeCompare(b));
    return { tests: ["test/corpus.test.ts", "-t", `^${best.replaceAll(".", "\\.")}$`], snapshots: reach === "form" };
  }
  return undefined;
}

/** Each of `chosen`'s mutants and the compiler as it is, built in `work`,
 * by name: `unmutated`, and each mutation's own. A build of many chooses
 * its mutation by its file's name (`schema`'s chooser), so each is a copy
 * named for it. A runtime mutation's isn't built (`runtimeOnly`). */
async function buildAll(chosen: Mutation[], work: string): Promise<Map<string, string>> {
  const crate = join(work, "crate");
  const target = join(work, "target");
  const bin = join(work, "bin");
  mkdirSync(bin, { recursive: true });
  const read = (file: string) => readFileSync(join(root, file), "utf8");
  const built = new Map<string, string>();
  const cargo = async (files: Map<string, string>, names: string[]) => {
    syncTree(root, crate, crateFiles);
    for (const [path, text] of files) writeFileSync(join(crate, path), text);
    const p = await run(["cargo", "build", "--quiet", "--locked", "--target-dir", target], crate, 30 * 60_000);
    if (p.code !== 0) return `doesn't build: ${p.stderr.split("\n").find((line) => line.startsWith("error")) ?? p.stderr.slice(-400)}`;
    for (const name of names) {
      copyFileSync(join(target, "debug", "rust-js"), join(bin, name));
      built.set(name, join(bin, name));
    }
    return undefined;
  };
  const failed = await cargo(new Map(), ["unmutated"]);
  if (failed) throw new Error(`the compiler as it is ${failed}`);
  const sited: Sited[] = chosen.map((mutation) => ({ mutation, site: site(mutation, read) }));
  const groups = [...batches(sited, 16), ...sited.filter((s) => s.site === undefined).map((s) => [s])];
  for (const group of groups) {
    const files = schema(group, read);
    const problem = files instanceof Map ? await cargo(files, group.map((s) => s.mutation.name)) : files.problem;
    if (problem) for (const { mutation } of group) console.error(`${mutation.name}: ${problem}`);
  }
  syncTree(root, crate, crateFiles);
  return built;
}

/** A runtime mutation's: its file the runtime's JS, which the compiler
 * writes none of. */
export const runtimeOnly = (mutation: Mutation) => mutation.file.endsWith(".js");

/** `case` compiled by `compiler` into `dir`, of `build`'s flags: its exit
 * and its JS. A mutant's compile isn't kept (`RUST_JS_COMPILE_CACHE_MB`). */
async function compiled(compiler: string, file: string, dir: string, edition: string, flags: string[], kept: boolean) {
  mkdirSync(dir, { recursive: true });
  const { cmd, js } = compileCommand(file, dir, edition, "binary", flags);
  const p = await run([compiler, ...cmd.slice(1)], root, 10 * 60_000, kept ? {} : { RUST_JS_COMPILE_CACHE_MB: "0" });
  return { ok: p.code === 0, js: p.code === 0 && existsSync(js) ? readFileSync(js, "utf8") : "", path: js };
}

/** `js` run by Node, bounded. */
const ran = (js: string, dir: string): Promise<Exit> => run([node ?? "node", js], dir, 60_000);

const same = (a: Exit, b: Exit) => a.code === b.code && a.bytes.stdout.equals(b.bytes.stdout) && a.bytes.stderr.equals(b.bytes.stderr);

/** Each case of `cases` against each of `mutants`, `cores` at a time. */
async function sweep(cases: string[], mutants: Map<string, string>, builds: [string, string[]][], work: string, cores: number): Promise<Reached[]> {
  const reached = new Map<string, Reached["cases"]>([...mutants.keys()].filter((n) => n !== "unmutated").map((n) => [n, []]));
  let next = 0;
  let done = 0;
  const worker = async () => {
    for (let at = next++; at < cases.length; at = next++) {
      const file = cases[at];
      const name = basename(file);
      const wanted = directives(readFileSync(file, "utf8"));
      const edition = typeof wanted === "string" ? "2024" : wanted.edition;
      for (const [build, flags] of builds) {
        const dir = join(work, "cases", build, name.replace(/\.rs$/, ""));
        const base = await compiled(mutants.get("unmutated")!, file, join(dir, "unmutated"), edition, flags, true);
        let baseRun: Exit | undefined;
        for (const [mutation, compiler] of mutants) {
          if (mutation === "unmutated") continue;
          const mine = await compiled(compiler, file, join(dir, mutation), edition, flags, false);
          let reach: Reach | undefined;
          if (mine.ok !== base.ok) reach = "compile";
          else if (mine.ok && jsCompared(mine.js) !== jsCompared(base.js)) {
            baseRun ??= await ran(base.path, join(dir, "unmutated"));
            reach = same(baseRun, await ran(mine.path, join(dir, mutation))) ? "form" : "behaves";
          }
          if (reach) reached.get(mutation)!.push({ name, build, reach });
        }
      }
      if (++done % 250 === 0) console.error(`mutation-reach: ${done} of ${cases.length} cases`);
    }
  };
  await Promise.all(Array.from({ length: Math.max(1, cores) }, worker));
  return [...reached].map(([mutation, cases]) => ({ mutation, cases: cases.sort((a, b) => a.name.localeCompare(b.name)) }));
}

/** A mutation's line of the answer, and what to do of it. */
export function said(mutation: Mutation, reached: Reached["cases"], size: (name: string) => number): string {
  const count = (reach: Reach) => new Set(reached.filter((c) => c.reach === reach).map((c) => c.name)).size;
  const pattern = corpusPattern(mutation.tests);
  const own = pattern ? reached.filter((c) => pattern.test(c.name)) : [];
  const head = `${mutation.name}: reaches ${new Set(reached.map((c) => c.name)).size} cases (behaves ${count("behaves")}, compile ${count("compile")}, form ${count("form")})`;
  const tests = pattern === undefined ? "its tests run no corpus case" : own.length > 0 ? `its tests' cases reach it (${own.map((c) => `${c.name} ${c.reach}`).join(", ")})` : "its tests' cases don't reach it";
  const target = retarget(reached, size);
  const next = target
    ? own.length > 0 && own.some((c) => c.reach !== "form" || mutation.snapshots)
      ? "held as it is"
      : `retarget: tests: ${JSON.stringify(target.tests)}${target.snapshots ? ", snapshots: true" : ""}`
    : "reaches no case: a retirement to ask `--retire` of, or a case to write";
  return `${head}; ${tests}; ${next}`;
}

async function main() {
  const args = process.argv.slice(2);
  const list = (flag: string) => {
    const arg = args.find((a) => a.startsWith(`${flag}=`));
    return arg ? readFileSync(arg.slice(flag.length + 1), "utf8").split(/\s+/).filter(Boolean) : undefined;
  };
  const names = [...args.filter((a) => !a.startsWith("--")), ...(list("--names") ?? [])];
  if (names.length === 0) throw new Error("mutation-reach: name a mutation, or `--names=<file>` of them; none names every one");
  const chosen = names.map((name) => mutations.find((m) => m.name === name) ?? (() => { throw new Error(`no mutation ${name}`); })());
  for (const m of chosen.filter(runtimeOnly)) console.log(`${m.name}: of the runtime, which no compiled JS holds: its reach is what runs, \`bun scripts/mutations.ts ${m.name}\`'s`);
  const swept = chosen.filter((m) => !runtimeOnly(m));
  if (swept.length === 0) return;
  holdLease("mutation-reach");
  const work = join(root, "target", "mutation-reach");
  const cases = (list("--cases")?.map((c) => (c.includes("/") ? c : join(root, "test", "corpus", c))) ?? readdirSync(join(root, "test", "corpus")).filter((f) => f.endsWith(".rs")).map((f) => join(root, "test", "corpus", f))).sort();
  const builds: [string, string[]][] = [["main", []], ...(args.includes("--std-iterators") ? [["std-iterators", ["--std-iterators"]] as [string, string[]]] : [])];
  const mutants = await buildAll(swept, work);
  const answered = await sweep(cases, mutants, builds, work, machine().cores);
  const size = (name: string) => statSync(join(root, "test", "corpus", name)).size;
  for (const { mutation, cases } of answered) console.log(said(swept.find((m) => m.name === mutation)!, cases, size));
  for (const m of swept.filter((m) => !mutants.has(m.name))) console.log(`${m.name}: its mutant didn't build (above), so nothing's said of it`);
  writeFileSync(join(work, "reach.json"), `${JSON.stringify(answered, null, 2)}\n`);
  console.error(`mutation-reach: ${join(work, "reach.json")}`);
}

if (import.meta.main) await main();
