// A changed file's dependants: the test files that can see it change, by what
// the test map recorded reaching it (ADR 1210). The one lookup the selector
// (scripts/select.ts) asks of a file a test builds an app from, where it ran
// everything (ADR 1101, amended). Its sources:
//
//   readers  each test that imports the file, transitively, or reads it as
//            text, itself or through a module it imports (#2480's, ADR 1101's
//            amendment of 2026-11-09): what finds an example's sources, which
//            the tests copy to their target before a bundler reads the copy
//   loads    each test the map saw load it, in its own process or in any
//            child it started (scripts/testMapLoads.ts, #2482): a Vite
//            config by what its bundle was made of; a file no test loads,
//            not JS or TS (a lock, a list), the tests that loaded anything
//            under its nearest folder that has loads
//   edges    rustc's resolved references, recorded with the map (#2608,
//            worker-02's): reserved, `TestMap.defs` and `edges`; until then a
//            Rust source of an app's folder runs everything
//
// Unsure means everything, as the selector's every rule: no map, a map
// without loads, a file Cargo builds as a test's dependency, or one no test
// loads, imports or reads, until a map has seen it.

import { dirname } from "node:path";

import { caseFiles, type Change, type TestMap } from "./testMap";

/** Of each test file, the modules it imports, transitively, the preload's
 * too, and what each of those reads as text (`readsOf`, a path, a folder or
 * a glob). */
export type Reach = Map<string, { modules: Set<string>; reads: Set<string> }>;

/** What a lookup is of: the push suite's test files, the map or why there's
 * none, and what each test file reaches besides compiles. */
export type Lookup = { files: string[]; map: TestMap | string; reach: Reach };

/** A change's dependants: the test files it runs, the corpus's file meaning
 * every case, and why; or everything, and why. */
export type Dependants = { all: true; why: string } | { all: false; tests: string[]; why: string };

/** The baselines a test reads by a path its source doesn't spell, each with
 * the tests that read it: what `readsOf` can't see. */
export const baselines: { path: string; owners: string[] }[] = [
  // Each case's own JS, `${name}.js`, beside its source.
  { path: "test/corpus/*.js", owners: ["test/readability.test.ts"] },
  // Each snapshot's files, by the snapshot's name.
  { path: "test/snapshots/**", owners: ["test/snapshots.test.ts", "test/snapshots-serde.test.ts", "test/readability.test.ts"] },
  // The playground's committed JS, its snapshot (test/snapshots.test.ts).
  { path: "wasm/web/rust/**/*.{js,jsx}", owners: ["test/snapshots.test.ts", "test/playground.test.ts", "test/readability.test.ts"] },
];

const baselineGlobs = baselines.map(({ path, owners }) => ({ glob: new Bun.Glob(path), owners }));

/** The tests `file` reaches as a module or a read. */
export function readersOf(file: string, { files, reach }: Lookup): string[] {
  const found: string[] = [];
  for (const [test, { modules, reads }] of reach) {
    if (test === file) continue;
    if (modules.has(file) || [...reads].some((read) => file === read || file.startsWith(`${read}/`) || (read.includes("*") && new Bun.Glob(read).match(file)))) found.push(test);
  }
  for (const { glob, owners } of baselineGlobs) if (glob.match(file)) found.push(...owners.filter((owner) => files.includes(owner)));
  return [...new Set(found)];
}

/** A file Cargo builds as a test's dependency, which no process loads:
 * rustc's references will say what reaches it (#2608). */
const cargoBuilds = /(?:^|\/)(?:Cargo\.toml|Cargo\.lock|build\.rs)$|\.rs$|^\.cargo\//;

/** What a JS runtime loads by its name: a module, never a folder's. */
const script = /\.(?:[cm]?[jt]sx?)$/;

/** The test files `changes` reach (the file's comment says how). */
export function dependantsOf(changes: Change[], lookup: Lookup): Dependants {
  const { files, map } = lookup;
  const tests = new Set<string>();
  const why: string[] = [];
  const of = typeof map === "string" ? undefined : map.loads?.of;
  const paths = typeof map === "string" ? [] : (map.paths ?? []);
  // Each path's loaders, and the tests whose loads are unknown.
  const loaders = new Map<number, string[]>();
  const unknown: string[] = [];
  for (const [test, loaded] of Object.entries(of ?? {})) {
    if (!files.includes(test) && !(caseFiles as readonly string[]).includes(test)) continue;
    if (loaded === null) unknown.push(test);
    else for (const at of loaded) (loaders.get(at) ?? loaders.set(at, []).get(at)!).push(test);
  }
  const at = new Map(paths.map((path, index) => [path, index]));
  for (const { file } of changes) {
    const top = file.split("/")[0];
    const app = `${file} is of ${top}/, which tests build as an app's dependency`;
    if (typeof map === "string") return { all: true, why: `${app}, and ${map}` };
    if (cargoBuilds.test(file)) return { all: true, why: `${app}, which Cargo builds, and no process loads (rustc's references will say, #2608)` };
    if (of === undefined) return { all: true, why: `${app}, and the map at ${map.commit.slice(0, 9)} recorded no loads` };
    // Its own loaders; of a file that's no module, those of its folder's.
    let loading: string[] = [];
    let folder: string | undefined;
    const own = at.get(file);
    if (own !== undefined) loading = loaders.get(own) ?? [];
    else if (!script.test(file)) {
      for (let dir = dirname(file); dir !== "." && loading.length === 0; dir = dirname(dir)) {
        const under = paths.flatMap((path, index) => (path.startsWith(`${dir}/`) ? (loaders.get(index) ?? []) : []));
        if (under.length > 0) [loading, folder] = [[...new Set(under)], dir];
      }
    }
    const readers = readersOf(file, lookup);
    if (loading.length === 0 && readers.length === 0) return { all: true, why: `${file} is of ${top}/, which no test the map at ${map.commit.slice(0, 9)} saw load, import or read` };
    for (const test of [...loading, ...readers, ...unknown]) tests.add(test);
    const count = (n: number, what: string) => `${n} test${n === 1 ? "" : "s"} ${what}`;
    const said = [count(loading.length, folder === undefined ? "load it" : `load what's of ${folder}/`), count(readers.length, "import or read it")];
    if (unknown.length > 0) said.push(`${unknown.length} whose loads are unknown`);
    why.push(`${file}: ${said.join(", ")}`);
  }
  return { all: false, tests: [...tests], why: why.join("; ") };
}
