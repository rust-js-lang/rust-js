// Tracked non-crate inputs of the existing mutation runner (#1848). A
// workspace owns every source it may write; installed third-party packages
// are read-only inputs, while workspace package links point inside the copy.
import {existsSync,readFileSync,rmSync,writeFileSync} from "node:fs";
import {join,resolve} from "node:path";
import {Worker} from "node:worker_threads";
import {bytesOf} from "../tooling/bounded.js";
import { run, type Exit } from "../test/child";
import type { Mutation } from "./mutations";
import { underLoad } from "./timing";

/** Preparation excludes workspace copying. The real Intl trace takes 21–25 ms
 * from launch to exit; retain the existing 1 s scheduling floor, scaled by
 * load. Worker copying and the shared child verdict remove the false timeout
 * rather than retaining the diagnostic interim 20 s allowance (#1861). */
const prepareTimeout = underLoad(1_000);

/** Copy on a worker so one mutant's files cannot block another child's exit.
 * Existing build/test slots bound how many copies may be active. */
export function makeSourceWorkspace(root: string, workspace: string, files: string[]): Promise<number> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./source-workspace.ts", import.meta.url), {workerData:{root,workspace,files}});
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
  test: (workspace: string, env: Record<string, string>) => Promise<SourceRun>,
  judge: (exit: Exit, output: string) => "caught" | "survived" | "inconclusive",
  mutate: (source: string, mutation: Mutation) => string | { problem: string },
): Promise<{
  result: string;
  output: string;
  build: number;
  tests: number;
  bytes: number;
}> {
  if (!/\.(m?[jt]s|[jt]sx)$/.test(mutation.file))
    throw new Error(`source load proof is not supported for ${mutation.file}`);
  let build = 0,
    tests = 0,
    peak = 0;
  let output = "";
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
      const owner = join(workspace, mutation.file),
        marker = join(workspace, ".mutation-loaded");
      let source = readFileSync(owner, "utf8");
      if (changed) {
        const mutated = mutate(source, mutation);
        if (typeof mutated !== "string") throw new Error(mutated.problem);
        source = mutated;
      }
      writeFileSync(owner, markSourceLoad(source, marker, mutation.name));
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
      if (!sourceWasLoaded(marker, mutation.name))
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
        const verdict = judge(tested.exit, tested.output);
        return {
          result:
            verdict === "caught"
              ? "caught"
              : verdict === "survived"
                ? "SURVIVED: its tests passed"
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
