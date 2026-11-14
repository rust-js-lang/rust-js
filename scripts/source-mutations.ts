// Tracked non-crate inputs of the existing mutation runner (#1848). A
// workspace owns every source it may write; installed third-party packages
// are read-only inputs, while workspace package links point inside the copy.
import {
  constants,
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { bytesOf, freeBytes } from "../tooling/bounded.js";
import { floorBytes } from "../test/bounded";
import { run, type Exit } from "../test/child";
import type { Mutation } from "./mutations";
import { underLoad } from "./timing";

/** A generator's preparation: 18 ms alone in a quiet workspace, but past a
 * second for three of ten mutants run at once, which #1861 is finding the
 * cause of. Until it's found, twice the most the measured step took, 9.5 s
 * with the workspace's copy in a batch beside another checkout's check (ADR
 * 1154), as the load makes it (`underLoad`): a slow spot listed in
 * docs/dev-loop.md, not a bound the generator needs. */
const prepareTimeout = underLoad(20_000);

/** An independently writable source tree, without generated stores. The
 * caller supplies Git's tracked files, including current working-tree edits. */
export function makeSourceWorkspace(
  root: string,
  workspace: string,
  files: string[],
): number {
  const kept = files.filter(
    (file) =>
      !file
        .split("/")
        .some((part) => ["target", "node_modules", ".git"].includes(part)),
  );
  const needed = kept.reduce(
    (total, file) => total + lstatSync(join(root, file)).blocks * 512,
    0,
  );
  if (freeBytes(root) - needed < floorBytes)
    throw new Error(
      "a source mutation workspace would cross the development disk floor",
    );
  mkdirSync(workspace, { recursive: true });
  for (const file of kept) {
    const from = join(root, file),
      to = join(workspace, file);
    mkdirSync(dirname(to), { recursive: true });
    copyFileSync(from, to, constants.COPYFILE_FICLONE);
  }
  const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const packages = new Map<string, string>();
  for (const pattern of manifest.workspaces ?? []) {
    for (const directory of new Bun.Glob(pattern).scanSync({
      cwd: root,
      onlyFiles: false,
    })) {
      const file = join(root, directory, "package.json");
      if (existsSync(file))
        packages.set(JSON.parse(readFileSync(file, "utf8")).name, directory);
    }
  }
  const linkDependencies = (directory: string) => {
    const installed = join(root, directory, "node_modules");
    const local = join(workspace, directory, "node_modules");
    mkdirSync(local, { recursive: true });
    for (const name of existsSync(installed) ? readdirSync(installed) : []) {
      if (name === ".bin") continue;
      const source = join(installed, name);
      if (name.startsWith("@") && statSync(source).isDirectory()) {
        mkdirSync(join(local, name), { recursive: true });
        for (const child of readdirSync(source)) {
          if (!packages.has(name + "/" + child))
            symlinkSync(join(source, child), join(local, name, child));
        }
      } else if (!packages.has(name)) symlinkSync(source, join(local, name));
    }
    for (const [name, path] of packages) {
      const to = join(local, name);
      mkdirSync(dirname(to), { recursive: true });
      symlinkSync(join(workspace, path), to);
    }
  };
  linkDependencies("");
  for (const directory of packages.values()) linkDependencies(directory);
  return bytesOf(workspace);
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
      peak = Math.max(peak, makeSourceWorkspace(root, workspace, files));
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
