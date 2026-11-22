// The test map's item layer (#2288): which of the map's functions reach each
// item of the compiler's source outside any function, an enum and its
// variants, a struct and its fields, a constant, a type, a trait, and each
// `impl`'s header, so a change there selects their tests, not every test.
// Recorded with the map, at its commit, from rust-analyzer's SCIP index of
// the crate (`rust-analyzer scip`, the pinned toolchain's component): its
// definitions, with their extents, and its references. The rustc crates the
// compiler is built on resolve to nothing there, so what's reached through
// their types isn't a reference SCIP gives; each item's references are also
// every function whose counted code names it or a member of it, its
// variants', fields' and methods' identifiers, the textual belt (the lead's
// rule: under-selection is the costly way to be wrong). Any failure to make
// the index gives no item layer, said, never part of one.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { isBinarySource, isCompilerSource } from "./compiler-crates";

/** An item outside any function, by the lines its definition spans. */
export type ItemRecord = {
  file: string;
  /** Its first and last line, 1-based, as its definition encloses them. */
  span: [number, number];
  symbol: string;
  /** The map's functions, by index, that reference it or a member of it, by
   * SCIP or by name in their counted code. */
  refs: number[];
};

/** An `impl` block's header, whose change, its bounds, its trait or its
 * `Self`, can change which method a call dispatches to: its methods, and
 * every function that references its trait (the lead's rule). */
export type ImplRecord = {
  file: string;
  /** The header's lines, from `impl` to its opening brace, and the block's. */
  header: [number, number];
  span: [number, number];
  refs: number[];
  /** Whether it implements a trait, whose default a method added to it
   * overrides; an inherent one's added method only shadows one by name. */
  trait?: boolean;
};

export type ItemLayer = {
  items: ItemRecord[];
  impls: ImplRecord[];
  /** Of the map's functions, how many hold an identifier SCIP resolves to
   * nothing, a measure of what the textual belt carries. */
  unresolved: number;
  /** Each of the map's functions' own name, by index, the innermost function
   * the index defines around its first line, a closure its function's: what
   * a function moved to another file, a new one, is found by. */
  names: (string | null)[];
  /** Each `#[cfg(test)]` module's lines, the unit tests' code, which no
   * test of the map runs and which changes nothing the compiler does. */
  tests?: { file: string; span: [number, number] }[];
};

type Occurrence = { line: number; col: number; end: number; symbol: string; definition: boolean; enclosing?: [number, number] };
type Document = { path: string; occurrences: Occurrence[] };

/** Each document of a SCIP index, `index`, its occurrences: lines 0-based. */
export function readScip(index: Uint8Array): Document[] {
  const varint = (b: Uint8Array, p: { i: number }) => {
    let value = 0;
    let shift = 0;
    let byte: number;
    do {
      byte = b[p.i++]!;
      value += (byte & 0x7f) * 2 ** shift;
      shift += 7;
    } while (byte & 0x80);
    return value;
  };
  const fields = (b: Uint8Array, each: (field: number, wire: number, value: number | Uint8Array) => void) => {
    const p = { i: 0 };
    while (p.i < b.length) {
      const key = varint(b, p);
      const field = key >>> 3;
      const wire = key & 7;
      if (wire === 0) each(field, wire, varint(b, p));
      else if (wire === 2) {
        const length = varint(b, p);
        each(field, wire, b.subarray(p.i, p.i + length));
        p.i += length;
      } else if (wire === 1) p.i += 8;
      else if (wire === 5) p.i += 4;
      else throw new Error(`a SCIP index with a field of wire type ${wire}`);
    }
  };
  const packed = (value: number | Uint8Array) => {
    if (typeof value === "number") return [value];
    const out: number[] = [];
    const p = { i: 0 };
    while (p.i < value.length) out.push(varint(value, p));
    return out;
  };
  const text = new TextDecoder();
  const documents: Document[] = [];
  fields(index, (field, wire, value) => {
    if (field !== 2 || wire !== 2 || typeof value === "number") return;
    const document: Document = { path: "", occurrences: [] };
    fields(value, (f, w, v) => {
      if (f === 1 && typeof v !== "number") document.path = text.decode(v);
      if (f !== 2 || w !== 2 || typeof v === "number") return;
      let range: number[] = [];
      let symbol = "";
      let roles = 0;
      let enclosing: number[] | undefined;
      fields(v, (g, _, x) => {
        if (g === 1) range = range.concat(packed(x));
        if (g === 2 && typeof x !== "number") symbol = text.decode(x);
        if (g === 3 && typeof x === "number") roles = x;
        if (g === 7) enclosing = packed(x);
      });
      // [line, start, end] or [line, start, endLine, end].
      const end = range.length === 3 ? range[2]! : range[3]!;
      document.occurrences.push({
        line: range[0]!,
        col: range[1]!,
        end,
        symbol,
        definition: (roles & 1) !== 0,
        ...(enclosing === undefined ? {} : { enclosing: [enclosing[0]!, enclosing.length === 3 ? enclosing[0]! : enclosing[2]!] as [number, number] }),
      });
    });
    documents.push(document);
  });
  return documents;
}

/** The SCIP index of the checkout at `root`, by the pinned toolchain's
 * rust-analyzer, in `dir`: its documents, or why there are none. */
export function scipIndex(root: string, dir: string, channel: string, timeout = 15 * 60_000): Document[] | string {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const output = join(dir, "index.scip");
  const ran = spawnSync("rustup", ["run", channel, "rust-analyzer", "scip", ".", "--output", output], { cwd: root, encoding: "utf8", timeout, maxBuffer: 64 * 1024 * 1024 });
  if (ran.error !== undefined) return `rust-analyzer couldn't run: ${ran.error.message}`;
  if (ran.status !== 0 || !existsSync(output)) return `rust-analyzer scip exited ${ran.status ?? ran.signal}: ${(ran.stderr ?? "").trim().split("\n").slice(-3).join(" ")}`;
  try {
    const documents = readScip(new Uint8Array(readFileSync(output)));
    if (!documents.some((d) => isBinarySource(d.path))) return "its index holds none of the binary's sources";
    return documents;
  } catch (error) {
    return `its index couldn't be read: ${error instanceof Error ? error.message : error}`;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const keywords = new Set(
  "as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while".split(" "),
);

/** A symbol's last name: `InPlaceForm` of `.. in_place_forms/InPlaceForm#`,
 * `Copied` of `..InPlaceForm#Copied#`, `held` of `..Form#held.`. */
export function symbolName(symbol: string): string | undefined {
  const tail = symbol.split(" ").at(-1) ?? "";
  return tail.match(/([A-Za-z_][A-Za-z0-9_]*)[#.]$/)?.[1] ?? tail.match(/`([^`]+)`[#.]$/)?.[1];
}

/** Whether a SCIP symbol is an item outside any function the layer keeps:
 * a type, a constant, a static, a field, a variant, a trait or its item;
 * not a function, a module, a parameter, a type parameter, a local. */
function isItem(symbol: string): boolean {
  if (symbol.startsWith("local ")) return false;
  const tail = symbol.split(" ").at(-1) ?? "";
  return !tail.endsWith("/") && !tail.endsWith(").") && !tail.includes("(") && !tail.includes("[") && /[#.]$/.test(tail);
}

/** The item layer of the map's `functions` at the checkout `root`, of the
 * SCIP index's `documents`, each file's lines read by `lines`. */
export function itemLayer(documents: Document[], functions: { file: string; spans: [number, number][] }[], lines: (file: string) => string[]): ItemLayer {
  // Each function's spans, by file; and the identifiers of its counted code.
  const byFile = new Map<string, { at: number; from: number; to: number }[]>();
  functions.forEach(({ file, spans }, at) => {
    const list = byFile.get(file) ?? [];
    for (const [from, to] of spans) list.push({ at, from, to });
    byFile.set(file, list);
  });
  const holding = (file: string, line: number) => (byFile.get(file) ?? []).filter(({ from, to }) => from <= line && line <= to).map(({ at }) => at);
  const naming = new Map<string, Set<number>>();
  functions.forEach(({ file, spans }, at) => {
    const text = lines(file);
    for (const [from, to] of spans) {
      for (let line = from; line <= to; line++) {
        const code = (text[line - 1] ?? "").replace(/\/\/.*$/, "");
        for (const [word] of code.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) {
          if (keywords.has(word)) continue;
          const set = naming.get(word) ?? new Set<number>();
          set.add(at);
          naming.set(word, set);
        }
      }
    }
  });
  // The compiler's sources, each of its crates' (ADR 1181).
  const src = documents.filter((d) => isCompilerSource(d.path) && d.path.endsWith(".rs"));
  // Each referenced symbol, the functions referencing it.
  const referencing = new Map<string, Set<number>>();
  for (const document of src) {
    for (const o of document.occurrences) {
      if (o.definition || o.symbol.startsWith("local ")) continue;
      for (const at of holding(document.path, o.line + 1)) {
        const set = referencing.get(o.symbol) ?? new Set<number>();
        set.add(at);
        referencing.set(o.symbol, set);
      }
    }
  }
  // Each item's definition, and its members' names, which are items too:
  // a member's symbol is its owner's with more after a `#` or a `.`.
  const definitions = src.flatMap((d) => d.occurrences.filter((o) => o.definition && o.enclosing !== undefined && isItem(o.symbol)).map((o) => ({ file: d.path, o })));
  const itemSymbols = new Set(definitions.map(({ o }) => o.symbol));
  const owners = (symbol: string) => {
    const out: string[] = [];
    for (let i = 0; i < symbol.length; i++) {
      if (symbol[i] === "#" || symbol[i] === ".") {
        const prefix = symbol.slice(0, i + 1);
        if (itemSymbols.has(prefix)) out.push(prefix);
      }
    }
    return out;
  };
  const members = new Map<string, string[]>();
  for (const { o } of definitions) for (const owner of owners(o.symbol)) if (owner !== o.symbol) members.set(owner, [...(members.get(owner) ?? []), o.symbol]);
  // Each item, the functions referencing it or a member of it.
  const reaching = new Map<string, Set<number>>();
  for (const [symbol, at] of referencing) {
    for (const owner of owners(symbol)) {
      const set = reaching.get(owner) ?? new Set<number>();
      for (const f of at) set.add(f);
      reaching.set(owner, set);
    }
  }
  // An item's own methods, `Type#method().`, are among its members by name.
  const methodNames = new Map<string, string[]>();
  for (const document of src) {
    for (const o of document.occurrences) {
      if (!o.definition || !o.symbol.endsWith(").")) continue;
      const owner = o.symbol.replace(/[A-Za-z_][A-Za-z0-9_]*\(\)\.$/, "");
      const name = o.symbol.match(/([A-Za-z_][A-Za-z0-9_]*)\(\)\.$/)?.[1];
      if (name !== undefined) methodNames.set(owner, [...(methodNames.get(owner) ?? []), name]);
    }
  }
  const items: ItemRecord[] = definitions.map(({ file, o }) => {
    const refs = new Set<number>(reaching.get(o.symbol) ?? []);
    const names = [o.symbol, ...(members.get(o.symbol) ?? [])].map(symbolName).filter((name): name is string => name !== undefined);
    names.push(...(methodNames.get(o.symbol) ?? []));
    for (const name of names) for (const at of naming.get(name) ?? []) refs.add(at);
    return { file, span: [o.enclosing![0] + 1, o.enclosing![1] + 1], symbol: o.symbol, refs: [...refs].sort((a, b) => a - b) };
  });
  // An item named by another outside any function, `passes: JUMPS` in a
  // const `STAGES` a function iterates: what reaches that one reaches it,
  // by SCIP and by name, to a fixed point.
  const outermost = (file: string, line: number) => {
    const holding = items.filter((i) => i.file === file && i.span[0] <= line && line <= i.span[1]);
    return holding.length === 0 ? undefined : holding.reduce((a, b) => (b.span[1] - b.span[0] > a.span[1] - a.span[0] ? b : a));
  };
  const index = new Map(items.map((item, at) => [item, at] as const));
  const bySymbol = new Map(items.map((item, at) => [item.symbol, at] as const));
  const namedIn = new Map<number, Set<number>>();
  const edge = (named: number, by: number) => {
    if (named === by) return;
    const set = namedIn.get(named) ?? new Set<number>();
    set.add(by);
    namedIn.set(named, set);
  };
  for (const document of src) {
    for (const o of document.occurrences) {
      if (o.definition || o.symbol.startsWith("local ") || holding(document.path, o.line + 1).length > 0) continue;
      const by = outermost(document.path, o.line + 1);
      if (by === undefined) continue;
      for (const owner of owners(o.symbol)) {
        const named = bySymbol.get(owner);
        if (named !== undefined) edge(named, index.get(by)!);
      }
    }
  }
  // By name, only an item of its module, a type, a constant, a static or a
  // trait, not a member, whose names, `name`, `kind`, are every struct's.
  const byName = new Map<string, number[]>();
  items.forEach((item, at) => {
    const name = symbolName(item.symbol);
    if (name !== undefined && owners(item.symbol).length === 1) byName.set(name, [...(byName.get(name) ?? []), at]);
  });
  items.forEach((item, by) => {
    const text = lines(item.file);
    for (let line = item.span[0]; line <= item.span[1]; line++) {
      if (holding(item.file, line).length > 0) continue;
      const code = (text[line - 1] ?? "").replace(/\/\/.*$/, "");
      for (const [word] of code.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) for (const named of byName.get(word) ?? []) edge(named, by);
    }
  });
  const reached = items.map((item) => new Set(item.refs));
  for (let changed = true; changed; ) {
    changed = false;
    for (const [named, by] of namedIn) {
      for (const b of by) {
        for (const f of reached[b]!) {
          if (!reached[named]!.has(f)) {
            reached[named]!.add(f);
            changed = true;
          }
        }
      }
    }
  }
  items.forEach((item, at) => {
    item.refs = [...reached[at]!].sort((a, b) => a - b);
  });
  // Each `impl` header: its trait's references, and its methods.
  const impls: ImplRecord[] = [];
  for (const document of src) {
    const text = lines(document.path);
    for (let at = 0; at < text.length; at++) {
      if (!/^\s*(?:#\[[^\]]*\]\s*)*(?:unsafe\s+)?impl\b/.test(text[at]!)) continue;
      let depth = 0;
      let open = -1;
      let close = -1;
      for (let line = at; line < text.length && close < 0; line++) {
        for (const c of text[line]!.replace(/\/\/.*$/, "")) {
          if (c === "{") {
            if (depth === 0 && open < 0) open = line;
            depth++;
          } else if (c === "}") {
            depth--;
            if (depth === 0 && open >= 0) close = line;
          }
        }
      }
      if (open < 0 || close < 0) continue;
      const header = text.slice(at, open + 1).join(" ");
      const methods = (byFile.get(document.path) ?? []).filter(({ from, to }) => from > at + 1 && to <= close + 1).map(({ at: f }) => f);
      const refs = new Set<number>(methods);
      // Its trait, what precedes ` for `: by SCIP where its header's
      // occurrences say, and by name.
      const traitPart = header.includes(" for ") ? header.slice(0, header.indexOf(" for ")) : "";
      for (const o of document.occurrences) {
        if (o.line < at || o.line > open || o.definition) continue;
        const name = symbolName(o.symbol);
        if (name === undefined || !traitPart.includes(name)) continue;
        // The trait's, or any of its methods', `k.m()` of `Tr#m().`; and its
        // methods by name.
        for (const f of reaching.get(o.symbol) ?? referencing.get(o.symbol) ?? []) refs.add(f);
        for (const member of members.get(o.symbol) ?? []) for (const f of naming.get(symbolName(member) ?? "") ?? []) refs.add(f);
        for (const method of methodNames.get(o.symbol) ?? []) for (const f of naming.get(method) ?? []) refs.add(f);
      }
      const traitName = traitPart.match(/([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*$/)?.[1];
      if (traitName !== undefined) for (const f of naming.get(traitName) ?? []) refs.add(f);
      impls.push({ file: document.path, header: [at + 1, open + 1], span: [at + 1, close + 1], refs: [...refs].sort((a, b) => a - b), trait: traitPart !== "" });
      at = open;
    }
  }
  // How many functions hold an identifier SCIP resolved to nothing.
  const starts = new Map<string, Set<string>>();
  for (const document of src) starts.set(document.path, new Set(document.occurrences.map((o) => `${o.line}:${o.col}`)));
  let unresolved = 0;
  functions.forEach(({ file, spans }) => {
    const text = lines(file);
    const known = starts.get(file) ?? new Set<string>();
    const lacks = spans.some(([from, to]) => {
      for (let line = from; line <= to; line++) {
        const code = (text[line - 1] ?? "").replace(/\/\/.*$/, "");
        for (const m of code.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) {
          if (keywords.has(m[0]) || code[m.index! - 1] === "'" || /^\d/.test(m[0])) continue;
          if (!known.has(`${line - 1}:${m.index}`)) return true;
        }
      }
      return false;
    });
    if (lacks) unresolved++;
  });
  // Each function's own name, of the innermost function defined around it.
  const fnDefs = new Map<string, { name: string; from: number; to: number }[]>();
  for (const document of src) {
    fnDefs.set(document.path, document.occurrences.flatMap((o) => {
      const name = o.symbol.match(/([A-Za-z_][A-Za-z0-9_]*)\(\)\.$/)?.[1];
      return o.definition && o.enclosing !== undefined && name !== undefined ? [{ name, from: o.enclosing[0] + 1, to: o.enclosing[1] + 1 }] : [];
    }));
  }
  const names = functions.map(({ file, spans }) => {
    const first = spans[0]?.[0];
    if (first === undefined) return null;
    const around = (fnDefs.get(file) ?? []).filter(({ from, to }) => from <= first && first <= to);
    return around.length === 0 ? null : around.reduce((a, b) => (b.to - b.from < a.to - a.from ? b : a)).name;
  });
  // Each `#[cfg(test)]` module's lines.
  const tests: { file: string; span: [number, number] }[] = [];
  for (const document of src) {
    const text = lines(document.path);
    for (let at = 0; at < text.length; at++) {
      if (!/^\s*#\[cfg\(test\)\]\s*$/.test(text[at]!) || !/^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{/.test(text[at + 1] ?? "")) continue;
      let depth = 0;
      for (let line = at + 1; line < text.length; line++) {
        for (const c of text[line]!.replace(/\/\/.*$/, "")) depth += c === "{" ? 1 : c === "}" ? -1 : 0;
        if (depth === 0) {
          tests.push({ file: document.path, span: [at + 1, line + 1] });
          at = line;
          break;
        }
      }
    }
  }
  return { items, impls, unresolved, names, tests };
}

/** Whether `line` of `file` is in a `#[cfg(test)]` module of `layer`. */
export function inUnitTests(layer: ItemLayer, file: string, line: number): boolean {
  return (layer.tests ?? []).some((t) => t.file === file && t.span[0] <= line && line <= t.span[1]);
}

/** Of `layer`, what a new file of `text` touches: the map's functions of
 * each function it defines, by name, where one moved there, and the
 * functions that reach each item it defines, by name, or each trait it
 * implements; a function new everywhere is reached only by the functions
 * changed to call it, which their own changes select. A file that defines a
 * macro, whose expansions may be anywhere: why everything runs. */
export function touchedByNewFile(layer: ItemLayer, text: string): number[] | string {
  const code = text.split("\n").map((line) => line.replace(/\/\/.*$/, "")).join("\n");
  if (/\bmacro_rules!|#\[proc_macro/.test(code)) return "it defines a macro, whose expansions may be anywhere";
  const touched = new Set<number>();
  const fns = new Set([...code.matchAll(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]!));
  layer.names.forEach((name, at) => {
    if (name !== null && fns.has(name)) touched.add(at);
  });
  const defined = new Set([
    ...[...code.matchAll(/\b(?:struct|enum|union|trait|type|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)/g)].map((m) => m[1]!),
    ...[...code.matchAll(/\bimpl\b[^{;]*?\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s+for\b/g)].map((m) => m[1]!),
  ]);
  for (const item of layer.items) {
    const name = symbolName(item.symbol);
    if (name !== undefined && defined.has(name)) for (const at of item.refs) touched.add(at);
  }
  return [...touched];
}

/** Of `layer`, what a change at `line` of `file` outside any function's
 * counted code touches: an `impl` header's methods and its trait's
 * referencers, or the outermost item holding the line's referencers; none
 * where neither holds it. */
/** The `impl` block of `layer` whose body, not its header, holds `line`. */
export function implBodyAt(layer: ItemLayer, file: string, line: number): ImplRecord | undefined {
  return layer.impls.filter((i) => i.file === file && i.header[1] < line && line <= i.span[1]).sort((a, b) => a.span[1] - a.span[0] - (b.span[1] - b.span[0]))[0];
}

export function touchedByItem(layer: ItemLayer, file: string, line: number): number[] | undefined {
  // An `impl` block's header, or its body between its methods, a method
  // added there say, which may override its trait's default: its methods
  // and its trait's referencers.
  const impl = layer.impls.filter((i) => i.file === file && i.span[0] <= line && line <= i.span[1]).sort((a, b) => a.span[1] - a.span[0] - (b.span[1] - b.span[0]))[0];
  const holding = layer.items.filter((i) => i.file === file && i.span[0] <= line && line <= i.span[1]);
  if (holding.length === 0) return impl?.refs;
  const outermost = holding.reduce((a, b) => (b.span[1] - b.span[0] > a.span[1] - a.span[0] ? b : a));
  return impl === undefined ? outermost.refs : [...new Set([...outermost.refs, ...impl.refs])];
}
