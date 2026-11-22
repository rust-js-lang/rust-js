// src/lower/'s internal dependency graph (#2295, #2294's step 1a): which of
// its top-level submodules (mir, analysis, recognition, ...) name which
// items of which others, by rust-analyzer's resolution of every name, its
// SCIP index, never by matching text. Says the cycles between submodules,
// how many items cross each seam and which way, and the candidate crate
// cuts: a set of submodules nothing below it names back, with the items
// the rest would need it to expose.
//
//   bun scripts/lower-graph.ts                    # index the crate, then report
//   bun scripts/lower-graph.ts --index <file>     # an index already made
//   bun scripts/lower-graph.ts --json <file>      # and write the graph as JSON
//
// The index is `rust-analyzer scip .` of the checkout (about 50 s on the
// M1), kept in target/scip/; rerun after each cut to see what's left.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const root = join(import.meta.dir, "..");
const args = process.argv.slice(2);
const flag = (name: string) => {
  const at = args.indexOf(name);
  return at < 0 ? undefined : args[at + 1];
};

/** SCIP's protobuf, read as far as the graph needs: each document's path
 * and occurrences, each occurrence's symbol and whether it defines it. */
class Reader {
  at = 0;
  constructor(readonly bytes: Uint8Array, readonly end = bytes.length) {}
  varint(): number {
    let result = 0;
    let shift = 0;
    for (;;) {
      const byte = this.bytes[this.at++];
      result += (byte & 0x7f) * 2 ** shift;
      if (byte < 0x80) return result;
      shift += 7;
    }
  }
  /** Each field of the message, its number, wire type and its bytes' span. */
  *fields(): Generator<{ field: number; wire: number; start: number; end: number; value: number }> {
    while (this.at < this.end) {
      const key = this.varint();
      const field = Math.floor(key / 8);
      const wire = key % 8;
      if (wire === 0) {
        const value = this.varint();
        yield { field, wire, start: this.at, end: this.at, value };
      } else if (wire === 2) {
        const length = this.varint();
        const start = this.at;
        this.at += length;
        yield { field, wire, start, end: this.at, value: length };
      } else if (wire === 5) {
        this.at += 4;
      } else if (wire === 1) {
        this.at += 8;
      } else {
        throw new Error(`a wire type SCIP doesn't use: ${wire}`);
      }
    }
  }
}

const text = new TextDecoder();

type Occurrence = { symbol: string; defines: boolean; line: number };
type Document = { path: string; occurrences: Occurrence[] };

function documents(bytes: Uint8Array): Document[] {
  const found: Document[] = [];
  for (const top of new Reader(bytes).fields()) {
    if (top.field !== 2 || top.wire !== 2) continue;
    const doc: Document = { path: "", occurrences: [] };
    const reader = new Reader(bytes, top.end);
    reader.at = top.start;
    for (const field of reader.fields()) {
      if (field.field === 1 && field.wire === 2) doc.path = text.decode(bytes.subarray(field.start, field.end));
      if (field.field === 2 && field.wire === 2) {
        const occ = new Reader(bytes, field.end);
        occ.at = field.start;
        let symbol = "";
        let roles = 0;
        let line = -1;
        for (const part of occ.fields()) {
          if (part.field === 2 && part.wire === 2) symbol = text.decode(bytes.subarray(part.start, part.end));
          if (part.field === 3 && part.wire === 0) roles = part.value;
          if (part.field === 1 && part.wire === 2 && line < 0) {
            const range = new Reader(bytes, part.end);
            range.at = part.start;
            line = range.varint();
          }
        }
        // A module's path, `use crate::lower::mir::..`'s `mir`, names no
        // item: a `mod` declaration's isn't a dependency.
        if (symbol !== "" && !symbol.startsWith("local ") && !symbol.endsWith("/")) {
          doc.occurrences.push({ symbol, defines: (roles & 1) === 1, line });
        }
      }
    }
    found.push(doc);
  }
  return found;
}

/** The top-level submodule of src/lower/ a file is of: `src/lower/mir.rs`
 * and `src/lower/mir/..` are `mir`; `src/lower.rs` is `lower` itself. */
function moduleOf(path: string): string | undefined {
  if (path === "src/lower.rs") return "lower";
  const m = /^src\/lower\/([^/]+?)(?:\.rs|\/.*)$/.exec(path);
  return m?.[1];
}

const indexPath = flag("--index") ?? join(root, "target", "scip", "index.scip");
if (flag("--index") === undefined) {
  mkdirSync(join(root, "target", "scip"), { recursive: true });
  const made = spawnSync("rust-analyzer", ["scip", ".", "--output", indexPath], { cwd: root, stdio: ["ignore", "ignore", "inherit"] });
  if (made.status !== 0) throw new Error("rust-analyzer scip failed: is the rust-analyzer component installed (rustup component add rust-analyzer)?");
}
if (!existsSync(indexPath)) throw new Error(`no index at ${indexPath}`);
const docs = documents(new Uint8Array(readFileSync(indexPath)));

// Where each of the crate's symbols is defined, of src/lower/'s alone.
const definedIn = new Map<string, string>();
const linesOf = new Map<string, number>();
for (const doc of docs) {
  const module = moduleOf(doc.path);
  if (module === undefined) continue;
  linesOf.set(module, (linesOf.get(module) ?? 0) + readFileSync(join(root, doc.path), "utf8").split("\n").length);
  for (const occ of doc.occurrences) if (occ.defines && !definedIn.has(occ.symbol)) definedIn.set(occ.symbol, module);
}

// Each reference from one submodule to an item another defines: the edge
// `from -> to`, its distinct items and its references.
type Edge = { items: Map<string, number>; refs: number };
const edges = new Map<string, Map<string, Edge>>();
for (const doc of docs) {
  const from = moduleOf(doc.path);
  if (from === undefined) continue;
  for (const occ of doc.occurrences) {
    if (occ.defines) continue;
    const to = definedIn.get(occ.symbol);
    if (to === undefined || to === from) continue;
    const out = edges.get(from) ?? new Map<string, Edge>();
    edges.set(from, out);
    const edge = out.get(to) ?? { items: new Map(), refs: 0 };
    out.set(to, edge);
    edge.items.set(occ.symbol, (edge.items.get(occ.symbol) ?? 0) + 1);
    edge.refs++;
  }
}
const modules = [...linesOf.keys()].sort();

/** Whether a symbol is an inherent method of `FnCx`, the lowering's one
 * context, whose methods each submodule writes in its own file and every
 * other calls: the coupling a cut has to turn into traits or move first. */
const ofFnCx = (symbol: string) => /impl#\[`?FnCx[<`\]]/.test(symbol);

/** `edges` without what `skip` says, and without the submodules `drop`. */
function filtered(skip: (symbol: string) => boolean, drop: Set<string>): Map<string, Map<string, Edge>> {
  const kept = new Map<string, Map<string, Edge>>();
  for (const [from, out] of edges) {
    if (drop.has(from)) continue;
    for (const [to, e] of out) {
      if (drop.has(to)) continue;
      const items = new Map([...e.items].filter(([s]) => !skip(s)));
      if (items.size === 0) continue;
      const map = kept.get(from) ?? new Map<string, Edge>();
      kept.set(from, map);
      map.set(to, { items, refs: [...items.values()].reduce((a, b) => a + b, 0) });
    }
  }
  return kept;
}

// The strongly connected components, Tarjan's: each a set of submodules
// that name each other round a cycle, which no cut can put apart.
function components(edges: Map<string, Map<string, Edge>>, modules: string[]): string[][] {
  let index = 0;
  const order = new Map<string, number>();
  const low = new Map<string, number>();
  const stack: string[] = [];
  const on = new Set<string>();
  const found: string[][] = [];
  const visit = (v: string) => {
    order.set(v, index);
    low.set(v, index++);
    stack.push(v);
    on.add(v);
    for (const w of edges.get(v)?.keys() ?? []) {
      if (!order.has(w)) {
        visit(w);
        low.set(v, Math.min(low.get(v)!, low.get(w)!));
      } else if (on.has(w)) low.set(v, Math.min(low.get(v)!, order.get(w)!));
    }
    if (low.get(v) === order.get(v)) {
      const part: string[] = [];
      let w: string;
      do {
        w = stack.pop()!;
        on.delete(w);
        part.push(w);
      } while (w !== v);
      found.push(part.sort());
    }
  };
  for (const m of modules) if (!order.has(m)) visit(m);
  return found;
}
const sccs = components(edges, modules);
const sccOf = new Map<string, number>();
sccs.forEach((part, i) => part.forEach((m) => sccOf.set(m, i)));

const edge = (from: string, to: string) => edges.get(from)?.get(to);
const itemName = (symbol: string) => symbol.replace(/^rust-analyzer cargo rust-js [^ ]+ /, "");

// A candidate cut, of one submodule: what the rest names of it (its
// interface, if it were a crate), what it names of the rest (which must be
// below it, a crate of its own or the one it'd be cut from), and whether
// it's on a cycle, which a cut can't break without moving items first.
type Cut = { module: string; lines: number; exposes: number; exposedRefs: number; uses: number; usedBy: string[]; dependsOn: string[]; onCycle: boolean };
const cuts: Cut[] = modules.map((m) => {
  const exposedItems = new Set<string>();
  let exposedRefs = 0;
  const usedBy: string[] = [];
  for (const other of modules) {
    const e = edge(other, m);
    if (e === undefined) continue;
    usedBy.push(other);
    for (const s of e.items.keys()) exposedItems.add(s);
    exposedRefs += e.refs;
  }
  const usesItems = new Set<string>();
  for (const e of edges.get(m)?.values() ?? []) for (const s of e.items.keys()) usesItems.add(s);
  return {
    module: m,
    lines: linesOf.get(m) ?? 0,
    exposes: exposedItems.size,
    exposedRefs,
    uses: usesItems.size,
    usedBy: usedBy.sort(),
    dependsOn: [...(edges.get(m)?.keys() ?? [])].sort(),
    onCycle: (sccs[sccOf.get(m)!]?.length ?? 1) > 1,
  };
});

// Report.
const out: string[] = [];
out.push(`# src/lower/'s dependency graph, by rust-analyzer's SCIP index`);
out.push("");
out.push(`${modules.length} top-level submodules, ${[...linesOf.values()].reduce((a, b) => a + b, 0)} lines; ${definedIn.size} items defined; ${[...edges.values()].reduce((n, out) => n + out.size, 0)} edges between submodules.`);
out.push("");
out.push("## Cycles (strongly connected components of more than one submodule)");
for (const part of sccs.filter((p) => p.length > 1).sort((a, b) => b.length - a.length)) {
  const lines = part.reduce((n, m) => n + (linesOf.get(m) ?? 0), 0);
  out.push(`- ${part.length} submodules, ${lines} lines: ${part.join(", ")}`);
}
out.push("");
out.push("## The cycles without FnCx's methods, and without the root as a node");
out.push("A cut can't split a cycle; FnCx's inherent methods, written across the submodules, and the root's items (FnCx itself, R) tie them. Taken away, what's left:");
const fnCxRefs = [...edges.values()].flatMap((out) => [...out.values()]).reduce((n, e) => n + [...e.items].filter(([s]) => ofFnCx(s)).reduce((a, [, k]) => a + k, 0), 0);
const allRefs = [...edges.values()].flatMap((out) => [...out.values()]).reduce((n, e) => n + e.refs, 0);
out.push(`FnCx's methods are ${fnCxRefs} of the ${allRefs} references between submodules.`);
const variants: [string, Map<string, Map<string, Edge>>, string[]][] = [
  ["without FnCx's methods", filtered(ofFnCx, new Set()), modules],
  ["without FnCx's methods and the root", filtered(ofFnCx, new Set(["lower"])), modules.filter((m) => m !== "lower")],
];
const lastVariant = variants[variants.length - 1];
for (const [name, kept, mods] of variants) {
  const parts = components(kept, mods).filter((p) => p.length > 1).sort((a, b) => b.length - a.length);
  out.push(`- ${name}: ${parts.length} cycle(s); ${parts.map((p) => `${p.length} (${p.slice(0, 12).join(", ")}${p.length > 12 ? ", .." : ""})`).join("; ") || "none"}`);
}
out.push("");
out.push("## Layers below the root and FnCx's methods: each submodule's depth, what names nothing above it");
{
  const [, kept, mods] = lastVariant;
  const parts = components(kept, mods);
  const partOf = new Map<string, number>();
  parts.forEach((p, i) => p.forEach((m) => partOf.set(m, i)));
  const depth = new Map<number, number>();
  const depthOf = (i: number): number => {
    if (depth.has(i)) return depth.get(i)!;
    depth.set(i, 0);
    let d = 0;
    for (const m of parts[i]) for (const to of kept.get(m)?.keys() ?? []) if (partOf.get(to) !== i) d = Math.max(d, depthOf(partOf.get(to)!) + 1);
    depth.set(i, d);
    return d;
  };
  const rows = parts.map((p, i) => ({ part: p, depth: depthOf(i), lines: p.reduce((n, m) => n + (linesOf.get(m) ?? 0), 0) }));
  out.push("| depth | submodules | lines |");
  out.push("|---|---|---|");
  for (const r of rows.sort((a, b) => a.depth - b.depth || b.lines - a.lines)) out.push(`| ${r.depth} | ${r.part.join(", ")} | ${r.lines} |`);
}
out.push("");
out.push("## An order to cut by: the fewest items pointing up (a weighted feedback arc set, Eades-Lin-Smyth)");
out.push("Below the root and FnCx's methods, each submodule placed so that as few items as it can name a submodule placed above it. A prefix of this order is a candidate crate, cut from the bottom: `breaks` is the items it names above it, which must move down or turn into a trait before it can stand alone; `exposes` is what the rest names of it, its interface.");
{
  const [, kept, mods] = lastVariant;
  const weight = (from: string, to: string) => kept.get(from)?.get(to)?.items.size ?? 0;
  // Greedy: take sinks to the end, sources to the front, else the one
  // whose out-weight most exceeds its in-weight to the front.
  const left = new Set(mods);
  const front: string[] = [];
  const back: string[] = [];
  const outW = (v: string) => [...left].reduce((n, w) => n + (w === v ? 0 : weight(v, w)), 0);
  const inW = (v: string) => [...left].reduce((n, w) => n + (w === v ? 0 : weight(w, v)), 0);
  while (left.size > 0) {
    let moved = true;
    while (moved) {
      moved = false;
      for (const v of [...left].sort()) {
        if (outW(v) === 0) {
          back.unshift(v);
          left.delete(v);
          moved = true;
        } else if (inW(v) === 0) {
          front.push(v);
          left.delete(v);
          moved = true;
        }
      }
    }
    if (left.size === 0) break;
    const best = [...left].sort().reduce((a, b) => (outW(b) - inW(b) > outW(a) - inW(a) ? b : a));
    front.push(best);
    left.delete(best);
  }
  // Front first means "names others, named by few": a top. The order from
  // the bottom up is its reverse.
  const order = [...front, ...back].reverse();
  const position = new Map(order.map((m, i) => [m, i]));
  const breaksTotal = order.reduce((n, v) => n + order.reduce((k, w) => k + (position.get(w)! > position.get(v)! ? weight(v, w) : 0), 0), 0);
  out.push(`Items pointing up in this order, in all: ${breaksTotal}.`);
  out.push("");
  out.push("| from the bottom, through | lines | breaks (items it names above) | exposes (items above name of it) | FnCx methods it calls above | FnCx methods above call of it |");
  out.push("|---|---|---|---|---|---|");
  const fnCx = filtered((s) => !ofFnCx(s), new Set(["lower"]));
  let lines = 0;
  for (let k = 0; k < order.length; k++) {
    lines += linesOf.get(order[k]) ?? 0;
    const below = new Set(order.slice(0, k + 1));
    const breaks = new Set<string>();
    const exposes = new Set<string>();
    for (const v of below) {
      for (const [w, e] of kept.get(v) ?? []) if (!below.has(w)) e.items.forEach((_, s) => breaks.add(s));
    }
    for (const v of order.slice(k + 1)) {
      for (const [w, e] of kept.get(v) ?? []) if (below.has(w)) e.items.forEach((_, s) => exposes.add(s));
    }
    const up = new Set<string>();
    const down = new Set<string>();
    for (const [v, out] of fnCx) {
      for (const [w, e] of out) {
        if (below.has(v) && !below.has(w)) e.items.forEach((_, s) => up.add(s));
        if (!below.has(v) && below.has(w)) e.items.forEach((_, s) => down.add(s));
      }
    }
    out.push(`| ${k + 1}. ${order[k]} | ${lines} | ${breaks.size} | ${exposes.size} | ${up.size} | ${down.size} |`);
  }
}
out.push("");
out.push("## Submodules on no cycle (each a cut as it is), by the items the rest names of it");
out.push("| submodule | lines | items exposed (refs) | items it uses | named by | names |");
out.push("|---|---|---|---|---|---|");
for (const c of cuts.filter((c) => !c.onCycle).sort((a, b) => b.lines - a.lines)) {
  out.push(`| ${c.module} | ${c.lines} | ${c.exposes} (${c.exposedRefs}) | ${c.uses} | ${c.usedBy.length} | ${c.dependsOn.join(" ") || "-"} |`);
}
out.push("");
out.push("## The narrowest seams within the largest cycle: back-edges to break first");
const largest = [...sccs].sort((a, b) => b.length - a.length)[0] ?? [];
const inLargest = new Set(largest);
const seams: { from: string; to: string; items: number; refs: number; back: number }[] = [];
for (const from of largest) {
  for (const [to, e] of edges.get(from) ?? []) {
    if (!inLargest.has(to)) continue;
    seams.push({ from, to, items: e.items.size, refs: e.refs, back: edge(to, from)?.items.size ?? 0 });
  }
}
out.push("| from -> to | items | refs | items back (to -> from) |");
out.push("|---|---|---|---|");
for (const s of seams.filter((s) => s.back > 0).sort((a, b) => a.items - b.items || a.refs - b.refs).slice(0, 40)) {
  out.push(`| ${s.from} -> ${s.to} | ${s.items} | ${s.refs} | ${s.back} |`);
}
out.push("");
out.push("## Each submodule's interface if cut (largest cycle's members too), smallest exposure per line first");
out.push("| submodule | lines | on a cycle | items exposed | named by | names |");
out.push("|---|---|---|---|---|---|");
for (const c of [...cuts].sort((a, b) => a.exposes / Math.max(a.lines, 1) - b.exposes / Math.max(b.lines, 1))) {
  out.push(`| ${c.module} | ${c.lines} | ${c.onCycle ? "yes" : "no"} | ${c.exposes} | ${c.usedBy.length} | ${c.dependsOn.length} |`);
}
const report = out.join("\n");
console.log(report);

const json = flag("--json");
if (json !== undefined) {
  const graph = {
    modules: Object.fromEntries(modules.map((m) => [m, { lines: linesOf.get(m) ?? 0 }])),
    edges: Object.fromEntries(
      [...edges].map(([from, out]) => [
        from,
        Object.fromEntries([...out].map(([to, e]) => [to, { refs: e.refs, items: [...e.items].map(([s, n]) => [itemName(s), n]) }])),
      ]),
    ),
    cycles: sccs.filter((p) => p.length > 1),
  };
  writeFileSync(json, JSON.stringify(graph, null, 1));
}
