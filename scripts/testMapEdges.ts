// rustc's resolved references, recorded with the map (ADR 1210's edges,
// #2608): what scripts/edges, the rustc driver the counting build compiles
// each of the compiler's crates with (scripts/counting-rustc.sh), wrote of
// each crate, its definitions and each pair of a definition and one it
// names, read into the map's `defs` and `edges`. A crate's file is its last
// compile's, Cargo compiling only what changed, and is read only where each
// source it names is as the tree has it now. A crate with no file, one
// whose walk failed, or one whose sources moved since, and there are no
// edges, said why: never part of a graph.
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

/** The form of a crate's file the map reads, the driver's `VERSION`
 * (scripts/edges/src/main.rs): one of another is no file. */
export const edgesVersion = 1;

/** A crate's file, as the driver writes it. */
type CrateEdges = {
  version: number;
  crate: string;
  sources: { file: string; hash: string }[];
  defs: { id: string; file: string; lines: [number, number]; kind: string; name: string }[];
  edges: [string, string][];
};

/** A definition of the map's: its file, its lines, and the map's functions
 * whose code is in it, a function's own and its closures'. */
export type Def = { file: string; lines: [start: number, end: number]; fns?: number[] };

/** The edges' driver (scripts/edges of `root`), built for the checkout's
 * toolchain into `target`; or why it isn't, and the map has no edges. */
export function edgesDriver(root: string, target: string): { path: string } | string {
  const built = Bun.spawnSync(["cargo", "build", "--locked", "--quiet", "--manifest-path", join(root, "scripts", "edges", "Cargo.toml"), "--target-dir", target], { cwd: root, stdout: "inherit", stderr: "inherit" });
  return built.exitCode === 0 ? { path: join(target, "debug", "rust-js-edges") } : `its driver didn't build (exit ${built.exitCode})`;
}

/** What the driver wrote of `crates` in `dir`, each crate's file named by
 * the crate: each definition of a file under `root`, and each edge between
 * two, as pairs of indices of `defs`, namer then named, an `Int32Array`'s
 * bytes, base64; or why there are none. `functions` are the map's, which a
 * definition of a function holds. */
export function recordedEdges(
  dir: string,
  crates: string[],
  root: string,
  functions: { file: string; spans: [number, number][] }[],
  read: (file: string) => Buffer | undefined = (file) => (existsSync(file) ? readFileSync(file) : undefined),
): { defs: Def[]; edges: string } | string {
  const said: CrateEdges[] = [];
  for (const crate of crates) {
    if (existsSync(join(dir, `${crate}.failed`))) return `the walk of ${crate} failed`;
    const at = join(dir, `${crate}.json`);
    if (!existsSync(at)) return `no record of ${crate}: it wasn't compiled with the driver`;
    const one: CrateEdges = JSON.parse(readFileSync(at, "utf8"));
    if (one.version !== edgesVersion) return `${crate}'s record is version ${one.version}, not ${edgesVersion}`;
    for (const { file, hash } of one.sources) {
      if (!within(root, file)) continue;
      const [kind, value] = hash.split("=");
      if (kind !== "md5" && kind !== "sha1" && kind !== "sha256") return `${crate}'s record hashes ${file} by ${kind}, which can't be checked`;
      const bytes = read(file);
      if (bytes === undefined || createHash(kind).update(bytes).digest("hex") !== value) return `${crate}'s record is of ${relative(root, file)} as it was, not as the tree has it`;
    }
    // A record that holds none of the checkout's is none: its paths were
    // read from elsewhere than the checkout, and its sources never checked.
    if (!one.defs.some((def) => within(root, def.file))) return `${crate}'s record holds no definition under ${root}`;
    said.push(one);
  }
  // Each definition of the checkout's once, by its id.
  const index = new Map<string, number>();
  const defs: Def[] = [];
  for (const { defs: of } of said) {
    for (const def of of) {
      if (index.has(def.id) || !within(root, def.file)) continue;
      index.set(def.id, defs.length);
      defs.push({ file: relative(root, def.file), lines: def.lines, ...(def.kind === "fn" || def.kind === "assoc fn" ? { fns: [] } : {}) });
    }
  }
  // Each function of the map, in the innermost function definition holding
  // its code: a closure's is its function's.
  const byFile = new Map<string, number[]>();
  defs.forEach((def, at) => def.fns && (byFile.get(def.file) ?? byFile.set(def.file, []).get(def.file)!).push(at));
  functions.forEach(({ file, spans }, fn) => {
    if (spans.length === 0) return;
    const [from, to] = [spans[0]![0], spans.at(-1)![1]];
    let inner: number | undefined;
    for (const at of byFile.get(file) ?? []) {
      const [start, end] = defs[at]!.lines;
      if (start <= from && to <= end && (inner === undefined || start >= defs[inner]!.lines[0])) inner = at;
    }
    if (inner !== undefined) defs[inner]!.fns!.push(fn);
  });
  // Each edge between two of the checkout's definitions, once.
  const pairs = new Set<string>();
  const flat: number[] = [];
  for (const { edges } of said) {
    for (const [namer, named] of edges) {
      const [from, to] = [index.get(namer), index.get(named)];
      if (from === undefined || to === undefined || from === to || pairs.has(`${from} ${to}`)) continue;
      pairs.add(`${from} ${to}`);
      flat.push(from, to);
    }
  }
  return { defs, edges: Buffer.from(new Int32Array(flat).buffer).toString("base64") };
}

/** Whether `file`, an absolute path, is under `root`. */
const within = (root: string, file: string) => file.startsWith(`${root}/`);

/** A map's edges, `defs` indices, read from their base64. */
export function edgePairs(edges: string): Int32Array {
  return new Int32Array(new Uint8Array(Buffer.from(edges, "base64")).buffer);
}

/** Of a map with edges, what reaches each line outside any function's
 * counted code: the innermost definition holding the line, its functions,
 * and backwards along the edges each definition that names it, in turn,
 * the functions of each; a function reached is where its namers' tests
 * already run it, so the walk goes on past a function only from the line's
 * own, whose signature its callers' calls may change. Of lines `from` to
 * `to`, the innermost definition holding both, as of a line added between
 * two. `undefined` of a line no definition holds; none where the map has
 * no edges. */
export function edgeReach(map: { paths?: string[]; defs?: { file: number; lines: [number, number]; fns?: number[] }[]; edges?: string | null }): ((file: string, from: number, to?: number) => number[] | undefined) | undefined {
  const { paths, defs, edges } = map;
  if (paths === undefined || defs === undefined || typeof edges !== "string") return undefined;
  const byFile = new Map<string, number[]>();
  defs.forEach((def, at) => {
    const file = paths[def.file]!;
    (byFile.get(file) ?? byFile.set(file, []).get(file)!).push(at);
  });
  const namers: number[][] = defs.map(() => []);
  const pairs = edgePairs(edges);
  for (let at = 0; at < pairs.length; at += 2) namers[pairs[at + 1]!]!.push(pairs[at]!);
  return (file, from, to = from) => {
    let inner: number | undefined;
    for (const at of byFile.get(file) ?? []) {
      const [start, end] = defs[at]!.lines;
      if (start <= from && to <= end && (inner === undefined || end - start < defs[inner]!.lines[1] - defs[inner]!.lines[0])) inner = at;
    }
    if (inner === undefined) return undefined;
    const reached = new Set<number>();
    const seen = new Set([inner]);
    const queue = [inner];
    while (queue.length > 0) {
      const at = queue.shift()!;
      const fns = defs[at]!.fns ?? [];
      for (const fn of fns) reached.add(fn);
      if (at !== inner && fns.length > 0) continue;
      for (const namer of namers[at]!) if (!seen.has(namer)) (seen.add(namer), queue.push(namer));
    }
    return [...reached];
  };
}
