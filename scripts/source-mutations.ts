// Tracked non-crate inputs of the existing mutation runner (#1848). A
// workspace owns every source it may write; installed third-party packages
// are read-only inputs, while workspace package links point inside the copy.
import {existsSync,mkdirSync,readFileSync,rmSync,symlinkSync,writeFileSync} from "node:fs";
import {dirname,join,resolve} from "node:path";
import {Worker} from "node:worker_threads";
import {createHash} from "node:crypto";
import {dlopen,FFIType,ptr} from "bun:ffi";
import {bytesOf,freeBytes} from "../tooling/bounded.js";
import { run, type Exit } from "../test/child";
import type { Mutation } from "./mutations";
import { underLoad } from "./timing";
import { wasmBuild } from "../wasm/web/site.ts";

const checkout = join(import.meta.dir, "..");

/** Preparation excludes workspace copying. The real Intl trace takes 21–25 ms
 * from launch to exit; retain the existing 1 s scheduling floor, scaled by
 * load. Worker copying and the shared child verdict remove the false timeout
 * rather than retaining the diagnostic interim 20 s allowance (#1861). */
const prepareTimeout = underLoad(1_000);

/** What a workspace made of `files` is marked by, beside them, and its size. */
const madeMarker = ".source-workspace.json";
const filesKey = (files: string[]) => createHash("sha256").update(files.join("\0")).digest("hex");

/** clonefile(2) of the tree `from` as `to`, which mustn't be there: APFS's
 * copy of a whole tree in one call, each file shared until it's written.
 * Whether it cloned: not where the system has no clonefile, or the volume
 * can't, as Linux's runners can't. */
let clonefile: ((from: Uint8Array, to: Uint8Array) => number) | undefined;
export function systemClone(from: string, to: string): boolean {
  if (process.platform !== "darwin") return false;
  try {
    if (clonefile === undefined) {
      const lib = dlopen("libSystem.B.dylib", { clonefile: { args: [FFIType.ptr, FFIType.ptr, FFIType.u32], returns: FFIType.i32 } });
      clonefile = (a, b) => lib.symbols.clonefile(ptr(a), ptr(b), 0);
    }
    return clonefile(Buffer.from(`${from}\0`), Buffer.from(`${to}\0`)) === 0;
  } catch {
    return false;
  }
}

/** The tree `from` made again as `to` by `clone`, the system's
 * (`systemClone`), or a test's where the system can't, so what's made of
 * it, its links' targets and its writes kept apart, is tested on every
 * machine. Whether it was made. */
export function cloneTree(from: string, to: string, clone: (from: string, to: string) => boolean = systemClone): boolean {
  mkdirSync(dirname(to), { recursive: true });
  return clone(from, to);
}

/** The files a source workspace of `root` is made of: the working tree's,
 * what git tracks and what it doesn't that it doesn't ignore, as a change's
 * own files are read (`since`, scripts/mutations.ts), and none the working
 * tree deleted. Of git's tracked files alone, a test the change adds,
 * uncommitted, wasn't in the copy, so its mutation ran nothing (#2485). */
export function sourceFilesOf(root: string): string[] {
  const listed = Bun.spawnSync(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], { cwd: root, stdout: "pipe", stderr: "pipe", timeout: 10_000 });
  if (listed.exitCode !== 0) throw new Error(`git ls-files in ${root} failed: ${listed.stderr.toString()}`);
  // Not a nested checkout git lists whole, `dir/`: another worktree's, under
  // the main checkout's `.claude/worktrees/` where nothing ignores it.
  return [...new Set(listed.stdout.toString().split("\0"))].filter((file) => file && !file.endsWith("/") && existsSync(join(root, file)));
}

/** A source workspace of `files` at `workspace`, and its size. Of a `root`
 * that's itself a workspace made of the same `files`, as each mutant's is
 * of the run's template, a clone of it whole (`cloneTree`, #2503): 0.05 s
 * for 7,362 files where each file's own clone took 0.55 s, and no data
 * written, so no floor asked. Else copied a file at a time on a worker so
 * one mutant's files cannot block another child's exit. Existing build/test
 * slots bound how many copies may be active. `free`, the disk's free bytes
 * it's made by, above the floor, a test's own (#2505). */
export function makeSourceWorkspace(root: string, workspace: string, files: string[], free = freeBytes(root)): Promise<number> {
  const key = filesKey(files);
  let made: { key?: string; bytes?: number } = {};
  try {
    made = JSON.parse(readFileSync(join(root, madeMarker), "utf8"));
  } catch {}
  if (made.key === key && typeof made.bytes === "number" && cloneTree(root, workspace)) return Promise.resolve(made.bytes);
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./source-workspace.ts", import.meta.url), {workerData:{root,workspace,files,free,key,marker:madeMarker}});
    let bytes: number | undefined;
    worker.once("message", (value: number) => { bytes = value; });
    worker.once("error", reject);
    // Wait for module completion too: appended actual-load markers must run.
    worker.once("exit", code => {
      if (code === 0 && bytes !== undefined) resolve(bytes);
      else reject(new Error("source workspace worker exited " + code + " without a result"));
    });
  });
}

/** Record execution, not merely resolution or a successful test process.
 * The marker is also used for unchanged controls. */
export function markSourceLoad(
  source: string,
  marker: string,
  name: string,
): string {
  let binding = "$rustJsMutationLoaded";
  while (source.includes(binding)) binding += "_";
  const mark = `import {appendFileSync as ${binding}} from "node:fs";\n${binding}(${JSON.stringify(marker)}, ${JSON.stringify(name + "\n")});\n`;
  // Keep shebangs, directive prologues and existing source line numbers.
  return source + "\n" + mark;
}

export function sourceWasLoaded(marker: string, name: string): boolean {
  return (
    existsSync(marker) &&
    readFileSync(marker, "utf8").split("\n").includes(name)
  );
}

/** The bindings crates' handwritten Rust, the js, webapi and react crates'
 * sources, which a test compiles into its own target as it starts, of its
 * workspace's files, by `buildReact` (test/support.ts), whose inputs they
 * are (#2105). */
const bindingsSources = ["builtins/src/", "webapi/src/", "react/src/"];

/** Whether `file` is the bindings crates' Rust (`bindingsSources`): a
 * mutant of it is compiled, not loaded, so no mark of its load is written
 * in it; it's proved compiled as each of its tests builds the crates. */
export function ofBindingsCrate(file: string): boolean {
  return file.endsWith(".rs") && bindingsSources.some((dir) => file.startsWith(dir));
}

/** Of `tests`, a mutation's, as `root` holds them, each test file that
 * doesn't build the bindings crates (`buildReact`), where a mutant of their
 * Rust would never be compiled, its tests passing whatever it is. */
export function unbuiltBy(root: string, tests: string[]): string[] {
  return tests.filter((arg) => /^test\/.*\.test\.tsx?$/.test(arg) && !readFileSync(join(root, arg), "utf8").includes("buildReact"));
}

/** What the wasm build made, which git doesn't track, so no workspace
 * copies (`wasmBuild`): `from`'s, the checkout's, linked into `workspace`,
 * for a mutation whose tests read it (`needs: "wasm"`, #2346). */
export function linkWasmBuild(workspace: string, from = checkout) {
  for (const path of wasmBuild) {
    mkdirSync(dirname(join(workspace, path)), { recursive: true });
    symlinkSync(join(from, path), join(workspace, path));
  }
}

export type SourceRun = {
  passed: boolean;
  ran: number;
  output: string;
  exit: Exit;
};

/** Run a source control and mutant through the existing tests/verdict path.
 * Preparation is a generator/build chain, charged to a build slot by the
 * caller. Pure module mutations have no preparation or compiler rebuild. */
export async function sourceMutation(
  mutation: Mutation,
  root: string,
  work: string,
  files: string[],
  test: (workspace: string, env: Record<string, string>, tests?: string[]) => Promise<SourceRun>,
  judge: (
    exit: Exit,
    output: string,
    again: (tests: string[]) => Promise<SourceRun>,
  ) => Promise<{ verdict: "caught" | "survived" | "inconclusive"; again: string; flake?: string }>,
  mutate: (source: string, mutation: Mutation) => string | { problem: string },
): Promise<{
  result: string;
  output: string;
  build: number;
  tests: number;
  bytes: number;
}> {
  // The bindings crates' Rust, compiled where a test builds them (#2105).
  const compiled = ofBindingsCrate(mutation.file);
  if (!compiled && !/\.(m?[jt]s|[jt]sx)$/.test(mutation.file))
    throw new Error(`source load proof is not supported for ${mutation.file}`);
  let build = 0,
    tests = 0,
    peak = 0;
  let output = "";
  const unbuilt = compiled ? unbuiltBy(root, mutation.tests ?? []) : [];
  if (compiled && unbuilt.length > 0)
    return {
      result: `INCONCLUSIVE: ${unbuilt.join(", ")} doesn't build the bindings crates (\`buildReact\`), so the mutant of ${mutation.file} is never compiled`,
      output,
      build,
      tests,
      bytes: 0,
    };
  for (const changed of [false, true]) {
    const workspace = join(
      work,
      "sources",
      mutation.name + (changed ? "-mutant" : "-control"),
    );
    rmSync(workspace, { recursive: true, force: true });
    const began = performance.now();
    try {
      peak = Math.max(peak, await makeSourceWorkspace(root, workspace, files));
      if (mutation.needs === "wasm") linkWasmBuild(workspace);
      const owner = join(workspace, mutation.file),
        marker = join(workspace, ".mutation-loaded");
      let source = readFileSync(owner, "utf8");
      if (changed) {
        const mutated = mutate(source, mutation);
        if (typeof mutated !== "string") throw new Error(mutated.problem);
        source = mutated;
      }
      writeFileSync(owner, compiled ? source : markSourceLoad(source, marker, mutation.name));
      const env = {
        RUST_JS_TYPESCRIPT: resolve(workspace, "typescript/index.js"),
      };
      if (mutation.prepare) {
        const prepared = await run(mutation.prepare, workspace, prepareTimeout, env);
        output += prepared.stdout + prepared.stderr;
        if (prepared.code !== 0 || prepared.signal || prepared.timedOut)
          return {
            result: prepared.timedOut
              ? `INCONCLUSIVE: source preparation timed out after ${prepareTimeout / 1000} s`
              : "INCONCLUSIVE: source preparation failed",
            output,
            build,
            tests,
            bytes: Math.max(peak, bytesOf(workspace)),
          };
      }
      build += (performance.now() - began) / 1000;
      const testing = performance.now();
      const tested = await test(workspace, env);
      tests += (performance.now() - testing) / 1000;
      output += tested.output;
      if (!compiled && !sourceWasLoaded(marker, mutation.name))
        return {
          result: "INCONCLUSIVE: mutated source was not executed",
          output,
          build,
          tests,
          bytes: Math.max(peak, bytesOf(workspace)),
        };
      if (!changed && (!tested.passed || tested.ran === 0))
        return {
          result: "INCONCLUSIVE: unchanged source control did not pass",
          output,
          build,
          tests,
          bytes: Math.max(peak, bytesOf(workspace)),
        };
      if (changed) {
        // A test that failed is run again in the mutant's workspace: a
        // flake is no catch (`judgeTwice`).
        const { verdict, again, flake } = await judge(tested.exit, tested.output, (tests) => test(workspace, env, tests));
        output += again;
        return {
          result:
            verdict === "caught"
              ? "caught"
              : verdict === "survived"
                ? "SURVIVED: its tests passed"
                : flake
                  ? `INCONCLUSIVE: ${flake}`
                  : "INCONCLUSIVE: no test failed or runner did not finish",
          output,
          build,
          tests,
          bytes: Math.max(peak, bytesOf(workspace)),
        };
      }
    } finally {
      peak = Math.max(peak, bytesOf(workspace));
      rmSync(workspace, { recursive: true, force: true });
    }
  }
  throw new Error("source mutation did not run");
}
