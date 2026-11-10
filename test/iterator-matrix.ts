// The iterator conformance matrix (ADR 1090): each source, chain of
// adapters, consumer and panic site, as a case of a program whose items
// and accumulators print as they're dropped, run under `catch_unwind`.
// test/iterator-matrix.test.ts compiles each batch of cases natively and
// as JS, in both builds, and compares each case's lines.
//
// A combination that isn't Rust isn't made: each adapter says what it
// takes and makes (an item, double-ended, exact-sized), each consumer what
// item it takes, and each panic site where it can be. Nothing is skipped
// by a list. After those, of chains of none or one adapter, two variants
// (#731): each closure capturing a `D` of its own by move, which drops with
// it, and a type stepped from its back by `rfold`, `nth_back` or `rfind`
// beside a forward consumer of the same type, as a crate-wide fact of it
// (#721) is decided by every use in the crate.
//
// It makes two tables of the same sources, adapters and consumers, which
// one harness runs (ITERATOR_TABLE): the matrix, every chain of none to two
// adapters through a few consumers (ADR 1090); and the conformance table,
// each adapter alone through every step of ADR 1092's contract, from either
// end, with closures that capture what has a destructor (#730). The same
// harness runs the panic points of the conformance table's cases (#817),
// and of operations beyond iterators, test/unwind-points.ts's (#854); and
// every std method that takes an iterator by value, given one that prints
// its steps, clones and drops, test/consumer-net.ts's.

import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { random } from "./generate";
import { pointsRunner } from "./panic-points";
import { casesOf, tableOf } from "./matrix-tables";
import { root, target } from "./support";
import { partSetting } from "./part-settings";

/** The builds each case is compiled in: the recognizers' rewrites, and
 * std's iterator structs alone (ADR 0908). */
export const builds: [name: string, flags: string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** Which table a run makes: the matrix (ADR 1090), the conformance
 * table (ADR 1092), the panic points of the conformance table's cases
 * (#817), the panic points of operations beyond iterators, the unwind
 * table (#854), every format spec of each value, the formatting grid
 * (#941), every number operation of each type's boundary values, the
 * numbers grid (#940), the panic points across `.await`, the async
 * table (#942), or every std method that takes an iterator by value, the
 * consumer net, `ITERATOR_TABLE`'s. */
export type Table = "matrix" | "conformance" | "points" | "unwind" | "format" | "numbers" | "async" | "consumers";
export const tables: Table[] = ["matrix", "conformance", "points", "unwind", "format", "numbers", "async", "consumers"];
export const table: Table = (() => {
  const which = partSetting("ITERATOR_TABLE") ?? "matrix";
  if (!(tables as string[]).includes(which)) throw new Error(`ITERATOR_TABLE=${which}: ${tables.map((t) => `\`${t}\``).join(", ")}`);
  return which as Table;
})();

/** Where each failure's program and lines are written, of either table. */
export const failures = join(target, "iterator-matrix");
/** The known failures a run found running as Rust does, one a line. */
export const fixedFile = join(failures, "fixed.txt");
const knownFile = join(root, "test", "iterator-matrix-known.txt");

/** How the known list names a build's cases of `of`: `plain` of the
 * matrix's, `conformance:plain` of the conformance table's. */
export const listedBuild = (build: string, of: Table = table) => (of === "matrix" ? build : `${of}:${build}`);
/** Whether a listed build's cases are `of`'s. */
export const ofTable = (listed: string, of: Table = table) => (of === "matrix" ? !listed.includes(":") : listed.startsWith(`${of}:`));

/** The known failures of every table, one list: by `<build> <n>`, a case's
 * build as `listedBuild` names it and its number in its table's cases, each
 * one's issue. Under a line `#<issue> <title>`, a line a build, `plain
 * 120-179,4410` or `conformance:std 12`, the numbers of its cases as sorted
 * ranges; `//` a comment. Its `// generator <version>` line is the version
 * of the matrix's cases its numbers are of, `// generator conformance
 * <version>` the conformance table's, `// generator points <version>` the
 * points table's, and so on. */
export type Known = { versions: Partial<Record<Table, string>>; issues: { heading: string; cases: Map<string, number[]> }[] };

export function parseKnown(text: string): Known {
  const known: Known = { versions: {}, issues: [] };
  for (const line of text.split("\n")) {
    const version = /^\/\/ generator (?:(\w+) )?(\w+)$/.exec(line);
    if (version && version[1] !== undefined && !(tables as string[]).includes(version[1])) {
      throw new Error(`test/iterator-matrix-known.txt: \`${line}\` is no table's generator`);
    }
    if (version) known.versions[(version[1] as Table | undefined) ?? "matrix"] = version[2];
    if (line === "" || line.startsWith("//")) continue;
    if (/^#\d+ /.test(line)) {
      known.issues.push({ heading: line, cases: new Map() });
      continue;
    }
    const at = known.issues.at(-1);
    const row = /^((?:(\w+):)?\w+) (\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*)$/.exec(line);
    if (!at || !row || (row[2] !== undefined && !(tables as string[]).includes(row[2]))) throw new Error(`test/iterator-matrix-known.txt: \`${line}\` is no build's cases under an issue`);
    const numbers = row[3].split(",").flatMap((range) => {
      const [from, to = from] = range.split("-").map(Number);
      return Array.from({ length: to - from + 1 }, (_, k) => from + k);
    });
    at.cases.set(row[1], [...(at.cases.get(row[1]) ?? []), ...numbers]);
  }
  return known;
}

/** Each known case, `<build> <n>`, and its issue. */
export function knownCases(known: Known): Map<string, number> {
  const out = new Map<string, number>();
  for (const { heading, cases } of known.issues) {
    const issue = Number(/^#(\d+)/.exec(heading)![1]);
    for (const [build, numbers] of cases) {
      for (const n of numbers) {
        if (out.has(`${build} ${n}`)) throw new Error(`test/iterator-matrix-known.txt: ${build} ${n} is listed twice`);
        out.set(`${build} ${n}`, issue);
      }
    }
  }
  return out;
}

/** Sorted numbers as ranges, `1-3,7`. */
export function ranges(numbers: number[]): string {
  const sorted = [...new Set(numbers)].sort((a, b) => a - b);
  const out: string[] = [];
  for (let i = 0; i < sorted.length; ) {
    let j = i;
    while (j + 1 < sorted.length && sorted[j + 1] === sorted[j] + 1) j++;
    out.push(i === j ? `${sorted[i]}` : `${sorted[i]}-${sorted[j]}`);
    i = j + 1;
  }
  return out.join(",");
}

/** The list's text: `header`, each table's version, each issue's builds'
 * ranges, the matrix's first. */
export function printKnown(known: Known, header: string): string {
  const order = (build: string) => `${build.includes(":") ? 1 : 0} ${build}`;
  const issues = known.issues
    .map(({ heading, cases }) => {
      const rows = [...cases].filter(([, numbers]) => numbers.length > 0).sort(([a], [b]) => order(a).localeCompare(order(b)));
      return rows.length === 0 ? "" : `\n${heading}\n${rows.map(([build, numbers]) => `${build} ${ranges(numbers)}`).join("\n")}\n`;
    })
    .join("");
  const versions = tables
    .filter((of) => known.versions[of] !== undefined)
    .map((of) => `// generator ${of === "matrix" ? "" : `${of} `}${known.versions[of]}\n`)
    .join("");
  return `${header}${versions}${issues}`;
}

export const readKnown = () => parseKnown(existsSync(knownFile) ? readFileSync(knownFile, "utf8") : "");

/** The cases' version: of their names, in their order, so a change to what
 * the generator makes, or in what order, is another. */
export function version(cases: Row[]): string {
  return new Bun.CryptoHasher("sha256").update(cases.map((c) => c.id).join("\n")).digest("hex").slice(0, 12);
}

/** What an iterator yields: an item that prints as it's dropped, `D`, a
 * number, a reference to either, or a pair of them. */
type Item = "D" | "N" | "&D" | "&N" | "pair";

/** An iterator so far: what it yields, whether a `D` it owns is in it
 * (whose `Drop` may panic), and the traits it has. */
type Shape = { item: Item; owns: boolean; back: boolean; exact: boolean };

/** Where a case panics: nowhere, in the source's `next`, in the chain's
 * first closure, in the consumer's closure, or in an item's `Drop`; or, of
 * the points table, at each of its points in turn (`run_points`), which no
 * other table's `sites` makes. */
export type Site = "none" | "next" | "stage" | "consumer" | "drop" | "drop+drop" | "points";
export const sites: Site[] = ["none", "next", "stage", "consumer", "drop"];

type Source = {
  name: string;
  shape: Shape;
  /** Its expression, given where it panics. */
  expr: (site: Site) => string;
  /** A statement before it, a `let` it borrows from. */
  before?: string;
  /** Whether its own `next` can panic: a crate's iterator. */
  own: boolean;
  /** Passed to a generic `fn` as `I`, so the case's chain is of `I`. */
  generic?: boolean;
};

/** The crate's own iterators, by which of `Iterator`'s steps each has of
 * its own: each prints as it runs one, so which step std's adapters call
 * is seen, and drops as `drop src`. */
type Own = { name: string; steps: ("fold" | "nth" | "last" | "count" | "size_hint" | "rfold" | "nth_back")[]; exact: boolean; back: boolean };
const owns: Own[] = [
  { name: "Plain", steps: [], exact: false, back: false },
  { name: "Folds", steps: ["fold"], exact: false, back: false },
  { name: "Nths", steps: ["nth"], exact: false, back: false },
  { name: "Lasts", steps: ["last"], exact: false, back: false },
  { name: "Counts", steps: ["count"], exact: false, back: false },
  { name: "Hints", steps: ["size_hint"], exact: false, back: false },
  { name: "Exact", steps: ["size_hint"], exact: true, back: false },
  { name: "Backs", steps: [], exact: false, back: true },
  { name: "Every", steps: ["fold", "nth", "last", "count", "size_hint", "rfold", "nth_back"], exact: true, back: true },
];

const ownSource = (own: Own, generic = false): Source => ({
  name: `${generic ? "generic " : ""}own ${own.name}`,
  shape: { item: "D", owns: true, back: own.back, exact: own.exact },
  expr: (site) => `${own.name}::new(${site === "next" ? 3 : 0})`,
  own: true,
  generic,
});

export const sources: Source[] = [
  { name: "vec", shape: { item: "D", owns: true, back: true, exact: true }, expr: () => "ds().into_iter()", own: false },
  { name: "rev vec", shape: { item: "D", owns: true, back: true, exact: true }, expr: () => "ds().into_iter().rev()", own: false },
  { name: "range", shape: { item: "N", owns: false, back: true, exact: true }, expr: () => "(1..6u32)", own: false },
  { name: "vec iter", shape: { item: "&D", owns: false, back: true, exact: true }, before: "let items = ds();", expr: () => "items.iter()", own: false },
  { name: "array iter", shape: { item: "&N", owns: false, back: true, exact: true }, before: "let nums = [1u32, 2, 3, 4, 5];", expr: () => "nums.iter()", own: false },
  ...owns.map((own) => ownSource(own)),
  { name: "generic vec", shape: { item: "D", owns: true, back: true, exact: true }, expr: () => "ds().into_iter()", own: false, generic: true },
  ownSource(owns.find((own) => own.name === "Every")!, true),
];

/** An adapter: the shape it makes of `from`, or none where it doesn't
 * apply; and its text, `panics` saying it's the stage that panics. */
type Adapter = { name: string; closure: boolean; shape: (from: Shape) => Shape | undefined; text: (from: Shape, panics: Panics) => string };

/** Whether a closure panics: at its second call, `true`; or, of the points
 * table, where the run's point is, `"point"`, each call a point (`point()`). */
type Panics = boolean | "point";

const base = (item: Item) => item !== "pair";
const owned = (item: Item) => item === "D" || item === "N";
/** A closure's body, after a counter of its calls panics at the second, if
 * it's the one that does; of the points table, after its point. */
const counted = (panics: Panics, what: string, body: string) =>
  panics === "point" ? `{ point(); ${body} }` : panics ? `{ calls += 1; if calls == 2 { panic!("${what}") } ${body} }` : body;

export const adapters: Adapter[] = [
  {
    name: "map",
    closure: true,
    shape: (from) => (from.item === "D" ? from : { item: "N", owns: false, back: from.back, exact: from.exact }),
    text: (from, panics) => {
      const body = { D: "D(x.0 + 100)", N: "x + 100", "&D": "x.0 + 100", "&N": "x + 100", pair: "x.v()" }[from.item];
      return `.map(|x| ${counted(panics, "map", body)})`;
    },
  },
  { name: "filter", closure: true, shape: (from) => ({ ...from, exact: false }), text: (_, panics) => `.filter(|x| ${counted(panics, "filter", "x.v() % 2 == 1")})` },
  { name: "skip", closure: false, shape: (from) => ({ ...from, back: from.back && from.exact }), text: () => ".skip(1)" },
  { name: "take", closure: false, shape: (from) => ({ ...from, back: from.back && from.exact }), text: () => ".take(3)" },
  { name: "fuse", closure: false, shape: (from) => from, text: () => ".fuse()" },
  {
    name: "peekable",
    closure: false,
    shape: (from) => from,
    // Its `let` and its peek are the case's (`caseFn`).
    text: () => "",
  },
  {
    name: "enumerate",
    closure: false,
    shape: (from) => (base(from.item) ? { item: "pair", owns: from.owns, back: from.back && from.exact, exact: from.exact } : undefined),
    text: () => ".enumerate()",
  },
  {
    name: "chain",
    closure: false,
    shape: (from) => (base(from.item) ? { ...from, exact: false } : undefined),
    text: (from) => `.chain(${{ D: "vec![D(6), D(7)]", N: "6..8u32", "&D": "tail.iter()", "&N": "[6u32, 7].iter()", pair: "" }[from.item]})`,
  },
  { name: "rev", closure: false, shape: (from) => (from.back ? from : undefined), text: () => ".rev()" },
  { name: "copied", closure: false, shape: (from) => (from.item === "&N" ? { ...from, item: "N" } : undefined), text: () => ".copied()" },
  {
    name: "cloned",
    closure: false,
    shape: (from) => (from.item === "&N" ? { ...from, item: "N" } : from.item === "&D" ? { ...from, item: "D", owns: true } : undefined),
    text: () => ".cloned()",
  },
  { name: "step_by", closure: false, shape: (from) => ({ ...from, back: from.back && from.exact }), text: () => ".step_by(2)" },
  {
    name: "flatten",
    closure: true,
    // Each item made two, by a closure of its own.
    shape: (from) => (owned(from.item) ? { ...from, exact: false } : undefined),
    text: (from, panics) => `.map(|x| ${counted(panics, "flatten", from.item === "D" ? "vec![D(x.0 + 10), x]" : "vec![x + 10, x]")}).flatten()`,
  },
  {
    name: "zip",
    closure: false,
    // With a shorter one, so an item it took of its own is dropped.
    shape: (from) => (base(from.item) ? { item: "pair", owns: from.owns, back: from.back && from.exact, exact: from.exact } : undefined),
    text: () => ".zip(10..13u32)",
  },
];

type Consumer = {
  name: string;
  /** Whether it has a closure of its own, which may panic. */
  closure: boolean;
  /** Whether it takes `shape`'s items. */
  takes: (shape: Shape) => boolean;
  /** Its statements, of the chain `it`, an expression, `panics` saying
   * its closure does. */
  text: (it: string, panics: Panics) => string;
  /** Whether it takes `it` by `&mut` where `it` is an expression, so a
   * generic case's parameter alone is `mut`. */
  lends?: boolean;
  /** Whether what it collects into asks the iterator's `size_hint`, which
   * only a source that prints its own shows: the conformance table runs it
   * of those alone (#820). */
  asks?: boolean;
};

/** A consumer's closure's start: its call counted, and a panic at the
 * second call if it's the one that panics; of the points table, its point
 * first. */
const call = (panics: Panics) =>
  `${panics === "point" ? "point(); " : ""}seen += 1;${panics === true ? ` if seen == 2 { panic!("consumer") }` : ""}`;
const any = () => true;

export const consumers: Consumer[] = [
  {
    name: "fold",
    closure: true,
    takes: any,
    text: (it, p) => `let acc = ${it}.fold(Acc(0), |mut a, x| { ${call(p)} a.0 += x.v(); a });\n    println!("fold {} after {seen}", acc.0);`,
  },
  {
    name: "rfold",
    closure: true,
    takes: (shape) => shape.back,
    text: (it, p) => `let acc = ${it}.rfold(Acc(0), |mut a, x| { ${call(p)} a.0 += x.v(); a });\n    println!("rfold {} after {seen}", acc.0);`,
  },
  {
    name: "try_fold",
    closure: true,
    takes: any,
    lends: true,
    // Broken off at the third item, the accumulator dropped where it is.
    text: (it, p) =>
      `let r = ${it}.try_fold(Acc(0), |mut a, x| { ${call(p)} a.0 += x.v(); if seen == 3 { Err(a.0) } else { Ok(a) } });\n    match &r { Ok(a) => println!("ok {}", a.0), Err(n) => println!("err {n}") }`,
  },
  { name: "any", closure: true, takes: any, lends: true, text: (it, p) => `let r = ${it}.any(|x| { ${call(p)} x.v() % 3 == 0 });\n    println!("any {r} after {seen}");` },
  { name: "all", closure: true, takes: any, lends: true, text: (it, p) => `let r = ${it}.all(|x| { ${call(p)} x.v() % 4 != 0 });\n    println!("all {r} after {seen}");` },
  { name: "count", closure: false, takes: any, text: (it) => `let n = ${it}.count();\n    println!("count {n}");` },
  { name: "last", closure: false, takes: any, text: (it) => `let l = ${it}.last();\n    println!("last {:?}", l.as_ref().map(V::v));` },
  {
    name: "nth",
    closure: false,
    takes: any,
    // The rest dropped after, as the iterator is.
    text: (it) => `let mut it = ${it};\n    let a = it.nth(1);\n    println!("nth {:?}", a.as_ref().map(V::v));\n    let b = it.next();\n    println!("then {:?}", b.as_ref().map(V::v));`,
  },
  {
    name: "sum",
    closure: false,
    takes: (shape) => shape.item !== "pair",
    text: (it) => `let s: Total = ${it}.sum();\n    println!("sum {}", s.0);`,
  },
  {
    name: "for",
    closure: true,
    takes: any,
    text: (it, p) => `for x in ${it} {\n        ${call(p)}\n        println!("item {}", x.v());\n        if seen == 3 {\n            break;\n        }\n    }\n    println!("for after {seen}");`,
  },
  { name: "vec", closure: false, takes: any, text: (it) => `let v: Vec<_> = ${it}.collect();\n    println!("vec {}", show(&v));` },
  {
    name: "string",
    closure: true,
    takes: any,
    text: (it, p) => `let s: String = ${it}.map(|x| { ${call(p)} char::from(b'a' + (x.v() % 26) as u8) }).collect();\n    println!("string {s:?}");`,
  },
  {
    name: "hash set",
    closure: true,
    takes: any,
    // What a hash collection holds is a number, which no drop prints: its
    // order is its hasher's, natively, and not JS's.
    text: (it, p) => `let s: HashSet<u32> = ${it}.map(|x| { ${call(p)} x.v() }).collect();\n    println!("set {:?}", sorted(s.into_iter().collect()));`,
  },
  {
    name: "hash map",
    closure: true,
    takes: any,
    text: (it, p) => `let m: HashMap<u32, u32> = ${it}.map(|x| { ${call(p)} (x.v() % 3, x.v()) }).collect();\n    println!("map {:?}", sorted(m.into_iter().collect()));`,
  },
];

/** What a crate steps from the back of a type, which may decide how the
 * crate makes it everywhere (ADR 0545): a paired case steps its type so
 * first. */
export type Setter = "rfold" | "nth_back" | "rfind";
export const setters: Setter[] = ["rfold", "nth_back", "rfind"];

/** A case as the matrix began, `plain`; with each closure of its chain and
 * consumer capturing a `D` of its own by move, so it drops with the
 * closure, `captures`; or with its iterator's type first stepped from its
 * back elsewhere in the case, by `setter`, a crate-wide fact beside the
 * forward consumer of the same type, `paired` (#731); or of its site
 * `drop+drop`, where the drop that panics is followed by another's, which
 * as the first unwinds aborts natively, `aborts`, a program of its own
 * (`batches`, #847). */
export type Variant = { kind: "plain" } | { kind: "captures" } | { kind: "paired"; setter: Setter } | { kind: "aborts" };

export type Case = { n: number; id: string; source: Source; chain: Adapter[]; consumer: Consumer; site: Site; variant: Variant };

/** The chains of none, one or two adapters that apply to `from`, and
 * the shape each makes. */
function chains(from: Shape): [Adapter[], Shape][] {
  const out: [Adapter[], Shape][] = [[[], from]];
  for (const a of adapters) {
    const one = a.shape(from);
    if (!one) continue;
    out.push([[a], one]);
    for (const b of adapters) {
      const two = b.shape(one);
      if (two) out.push([[a, b], two]);
    }
  }
  return out;
}

/** Whether `site` can panic in this case: a source of the crate's own, a
 * closure of the chain or the consumer, an owned `D`. */
function applies(site: Site, source: Source, chain: Adapter[], consumer: Consumer, shapes: Shape[]): boolean {
  switch (site) {
    case "none":
      return true;
    case "next":
      return source.own;
    case "stage":
      return chain.some((a) => a.closure);
    case "consumer":
      return consumer.closure;
    case "drop":
    case "drop+drop":
      return shapes.some((shape) => shape.owns);
    // Every case has points: `pointsCases()` makes them, not `sites`.
    case "points":
      return true;
  }
}

/** Each chain of `source`, as `each` takes it, with its shapes. */
function eachChain(source: Source, each: (chain: Adapter[], shape: Shape, shapes: Shape[]) => void) {
  for (const [chain, shape] of chains(source.shape)) {
    const shapes = [source.shape];
    for (const a of chain) shapes.push(a.shape(shapes.at(-1)!)!);
    each(chain, shape, shapes);
  }
}

/** Whether a case's chain or consumer has a closure to capture with. */
const hasClosure = (chain: Adapter[], consumer: Consumer) => chain.some((a) => a.closure) || (consumer.closure && consumer.name !== "for");

/** The forward consumers a paired case runs beside its setter: what read
 * a type's facts the crate decided by its back (#721). */
const forward = ["count", "last", "nth", "vec", "fold"];

/** Every case of the matrix, in one order, each id its own: the plain
 * cases first, as they were before the variants, so their numbers are
 * test/iterator-matrix-known.txt's still; then, of chains of none or one
 * adapter, the captures, and the pairs. */
export function allCases(): Case[] {
  const out: Case[] = [];
  const add = (source: Source, chain: Adapter[], consumer: Consumer, site: Site, variant: Variant) => {
    const tail =
      variant.kind === "plain" || variant.kind === "aborts" ? "" : variant.kind === "captures" ? " | captures" : ` | after ${variant.setter}`;
    const id = `${source.name} | ${chain.map((a) => a.name).join(".") || "-"} | ${consumer.name} | ${site}${tail}`;
    out.push({ n: out.length, id, source, chain, consumer, site, variant });
  };
  for (const source of sources) {
    eachChain(source, (chain, shape, shapes) => {
      for (const consumer of consumers) {
        if (!consumer.takes(shape)) continue;
        for (const site of sites) if (applies(site, source, chain, consumer, shapes)) add(source, chain, consumer, site, { kind: "plain" });
      }
    });
  }
  for (const source of sources) {
    eachChain(source, (chain, shape, shapes) => {
      if (chain.length > 1) return;
      for (const consumer of consumers) {
        if (!consumer.takes(shape) || !hasClosure(chain, consumer)) continue;
        for (const site of ["none", "stage", "consumer"] as Site[]) {
          if (applies(site, source, chain, consumer, shapes)) add(source, chain, consumer, site, { kind: "captures" });
        }
      }
    });
  }
  for (const source of sources) {
    if (source.generic) continue;
    eachChain(source, (chain, shape) => {
      if (chain.length > 1 || chain.some((a) => a.name === "peekable") || !shape.back) return;
      for (const consumer of consumers) {
        if (!forward.includes(consumer.name)) continue;
        for (const setter of setters) add(source, chain, consumer, "none", { kind: "paired", setter });
      }
    });
  }
  // Then, each a program of its own, a drop that panics and the next
  // drop after it, which natively aborts where it's as the first unwinds
  // (#847): of no adapter, or one, collected into a `Vec`.
  for (const source of sources) {
    eachChain(source, (chain, shape, shapes) => {
      if (chain.length > 1) return;
      for (const consumer of consumers) {
        if (!consumer.takes(shape) || (chain.length === 1 && consumer.name !== "vec")) continue;
        if (applies("drop+drop", source, chain, consumer, shapes)) add(source, chain, consumer, "drop+drop", { kind: "aborts" });
      }
    });
  }
  return out;
}

/** Of a case's closures, each an argument, `(|x| body)` or `(init, |a, x|
 * body)`, made to capture a guard of its own by move, `move |x| { let _g =
 * &g1; body }`, its body to the `,` or `)` that ends the argument; and how
 * many guards. */
function capturing(text: string): { text: string; guards: number } {
  let out = "";
  let guards = 0;
  let i = 0;
  const starts = /(?<=\(|, )\|[^|]*\| /g;
  for (let start = starts.exec(text); start; start = starts.exec(text)) {
    const body = start.index + start[0].length;
    let end = body;
    for (let depth = 0; depth > 0 || !(text[end] === ")" || text[end] === ","); end++) {
      if ("([{".includes(text[end])) depth++;
      if (")]}".includes(text[end])) depth--;
    }
    guards++;
    out += `${text.slice(i, start.index)}move ${start[0]}{ let _g = &g${guards}; ${text.slice(body, end)} }`;
    i = end;
    starts.lastIndex = end;
  }
  return { text: out + text.slice(i), guards };
}

/** What a paired case's setter does to `it`, of its type. */
function setterText(setter: Setter, it: string): string {
  switch (setter) {
    case "rfold":
      return `let r = ${it}.rfold(0, |a, x| a + x.v());\n    println!("rfold first {r}");`;
    case "nth_back":
      return `let r = ${it}.nth_back(1).map(|x| x.v());\n    println!("nth_back first {r:?}");`;
    case "rfind":
      return `let r = ${it}.rfind(|x| x.v() % 2 == 0).map(|x| x.v());\n    println!("rfind first {r:?}");`;
  }
}

// The conformance table (ADR 1092, #730): each adapter of std's alone,
// through each step of the contract, over the sources that tell its steps
// apart: a `Vec`'s, a reversed `Vec`'s, a range, a slice's, the crate's
// iterator that writes none of its steps, one that writes every one, one
// double-ended, and a type parameter of a `Vec`'s and of the crate's.

/** What a `map` makes of an item of `from`, its number past 100. */
const mapped = (from: Shape) => ({ D: "D(x.0 + 100)", N: "x + 100", "&D": "x.0 + 100", "&N": "x + 100", pair: "x.v()" })[from.item];

/** The adapters only the table runs, besides the matrix's. */
const stepAdapters: Adapter[] = [
  {
    name: "take_while",
    closure: true,
    shape: (from) => ({ ...from, back: false, exact: false }),
    text: (_, panics) => `.take_while(|x| ${counted(panics, "take_while", "x.v() % 10 < 4")})`,
  },
  {
    name: "skip_while",
    closure: true,
    shape: (from) => ({ ...from, back: false, exact: false }),
    text: (_, panics) => `.skip_while(|x| ${counted(panics, "skip_while", "x.v() % 10 < 2")})`,
  },
  {
    name: "map_while",
    closure: true,
    shape: (from) => ({ ...from, back: false, exact: false }),
    text: (_, panics) => `.map_while(|x| ${counted(panics, "map_while", "(x.v() % 10 < 4).then_some(x)")})`,
  },
  {
    name: "scan",
    closure: true,
    shape: (from) => ({ ...from, back: false, exact: false }),
    text: (_, panics) => `.scan(0u32, |s, x| ${counted(panics, "scan", "{ *s += 1; (*s < 4).then_some(x) }")})`,
  },
  { name: "inspect", closure: true, shape: (from) => from, text: (_, panics) => `.inspect(|x| ${counted(panics, "inspect", 'println!("saw {}", x.v())')})` },
  {
    name: "filter_map",
    closure: true,
    shape: (from) => ({ ...from, exact: false }),
    text: (_, panics) => `.filter_map(|x| ${counted(panics, "filter_map", "(x.v() % 2 == 1).then_some(x)")})`,
  },
  {
    name: "flat_map",
    closure: true,
    shape: (from) => (owned(from.item) ? { ...from, exact: false } : undefined),
    text: (from, panics) => `.flat_map(|x| ${counted(panics, "flat_map", from.item === "D" ? "vec![D(x.0 + 10), x]" : "vec![x + 10, x]")})`,
  },
  {
    // A closure that holds a `D` of its own, `drop 80`, `caseFn`'s `h`,
    // moved into the adapter, which its fold drops where std's does (ADR
    // 0991).
    name: "map holding",
    closure: true,
    shape: (from) => (from.item === "D" ? from : { item: "N", owns: false, back: from.back, exact: from.exact }),
    text: (from, panics) => `.map(move |x| { let _h = &h; ${counted(panics, "map", mapped(from))} })`,
  },
  {
    // The same made by a block given to the call, which runs as the
    // statement form does since #759's fix.
    name: "map holding block",
    closure: true,
    shape: (from) => (from.item === "D" ? from : { item: "N", owns: false, back: from.back, exact: from.exact }),
    text: (from, panics) => `.map({ let h = D(80); move |x| { let _h = &h; ${counted(panics, "map", mapped(from))} } })`,
  },
  // Lent, `caseFn`'s: its steps are `&mut I`'s.
  { name: "by_ref", closure: false, shape: (from) => from, text: () => "" },
];

/** A consumer's closure that holds a `D` of its own, `drop 90`, moved in,
 * and counts by the case's `seen`, which it borrows as `counter`: what
 * comes before the consumer's call, `holds`, and the closure, its start as
 * `call`'s. Each a statement of its own, as a block that makes the closure
 * is refused (`a closure that holds a value with a destructor, made here`). */
const holds = "let g = D(90);\n    let counter = &mut seen;\n    ";
const holding = (panics: Panics, params: string, body: string) =>
  `move |${params}| { let _g = &g; ${panics === "point" ? "point(); " : ""}*counter += 1;${panics === true ? ` if *counter == 2 { panic!("consumer") }` : ""} ${body} }`;
const back = (shape: Shape) => shape.back;

/** The steps only the table runs, besides the matrix's consumers: each
 * step of the contract that isn't one already, and the folds by value of
 * a closure that holds what has a destructor (#734). */
const stepConsumers: Consumer[] = [
  {
    name: "next",
    closure: false,
    takes: any,
    // What it leaves dropped where it's dropped, before what it gave.
    text: (it) => `let mut it = ${it};\n    let a = it.next();\n    drop(it);\n    println!("next {:?}", a.as_ref().map(V::v));`,
  },
  {
    name: "next_back",
    closure: false,
    takes: back,
    text: (it) => `let mut it = ${it};\n    let a = it.next_back();\n    println!("back {:?}", a.as_ref().map(V::v));\n    let b = it.next();\n    println!("then {:?}", b.as_ref().map(V::v));`,
  },
  {
    name: "nth_back",
    closure: false,
    takes: back,
    text: (it) => `let mut it = ${it};\n    let a = it.nth_back(1);\n    println!("nth_back {:?}", a.as_ref().map(V::v));\n    let b = it.next_back();\n    println!("then {:?}", b.as_ref().map(V::v));`,
  },
  {
    name: "rfind",
    closure: true,
    takes: back,
    lends: true,
    text: (it, p) => `let r = ${it}.rfind(|x| { ${call(p)} x.v() % 2 == 0 });\n    println!("rfind {:?} after {seen}", r.as_ref().map(V::v));`,
  },
  {
    name: "find",
    closure: true,
    takes: any,
    lends: true,
    text: (it, p) => `let r = ${it}.find(|x| { ${call(p)} x.v() % 2 == 0 });\n    println!("find {:?} after {seen}", r.as_ref().map(V::v));`,
  },
  {
    name: "position",
    closure: true,
    takes: any,
    lends: true,
    text: (it, p) => `let r = ${it}.position(|x| { ${call(p)} x.v() % 3 == 0 });\n    println!("position {r:?} after {seen}");`,
  },
  { name: "size_hint", closure: false, takes: any, text: (it) => `let it = ${it};\n    println!("hint {:?}", it.size_hint());` },
  {
    // What a collect into a `Vec` reserved, by the hints it asked as it grew
    // (#728, #733), which the crate's `Hints` and `Exact` print: its room
    // is what std promises of it, at least its length, not native Rust's
    // number, which ADR 0419 leaves to std's growth (case B).
    name: "capacity",
    closure: false,
    takes: any,
    text: (it) => `let v: Vec<_> = ${it}.collect();\n    println!("vec{} room {}", show(&v), v.capacity() >= v.len());`,
  },
  // Each other path std asks the hint by (#820, `iterators/hint_asks.rs`):
  // a collect into a `Result` or an `Option` of a `Vec`, through
  // `GenericShunt`'s hint; a `Vec`'s `extend` as it fills, from none and
  // from room of its own; a `VecDeque`'s, a `String`'s and a hash map's
  // `extend`, each reserving up front.
  {
    name: "result vec",
    closure: false,
    takes: any,
    asks: true,
    text: (it) =>
      `let r: Result<Vec<_>, u8> = ${it}.map(Ok).collect();\n    if let Ok(v) = r {\n        println!("result vec{} room {}", show(&v), v.capacity() >= v.len());\n    }`,
  },
  {
    name: "option vec",
    closure: false,
    takes: any,
    asks: true,
    text: (it) =>
      `let o: Option<Vec<_>> = ${it}.map(Some).collect();\n    if let Some(v) = o {\n        println!("option vec{} room {}", show(&v), v.capacity() >= v.len());\n    }`,
  },
  {
    name: "extend vec",
    closure: false,
    takes: any,
    asks: true,
    text: (it) => `let mut v = Vec::new();\n    v.extend(${it});\n    println!("extend vec{} room {}", show(&v), v.capacity() >= v.len());`,
  },
  {
    name: "extend vec with room",
    closure: false,
    takes: any,
    asks: true,
    text: (it) =>
      `let mut v = Vec::with_capacity(2);\n    v.extend(${it});\n    println!("extend room vec{} room {}", show(&v), v.capacity() >= v.len());`,
  },
  {
    name: "extend deque",
    closure: false,
    takes: any,
    asks: true,
    text: (it) => `let mut q = std::collections::VecDeque::new();\n    q.extend(${it});\n    println!("extend deque{}", show(q.make_contiguous()));`,
  },
  {
    name: "extend string",
    closure: false,
    takes: any,
    asks: true,
    text: (it) =>
      `let mut s = String::new();\n    s.extend(${it}.map(|x| char::from(b'a' + (x.v() % 26) as u8)));\n    println!("extend string {s:?}");`,
  },
  {
    name: "extend hash map",
    closure: false,
    takes: any,
    asks: true,
    text: (it) =>
      `let mut m = HashMap::new();\n    m.extend(${it}.map(|x| (x.v() % 3, x.v())));\n    println!("extend map {:?}", sorted(m.into_iter().collect()));`,
  },
  { name: "len", closure: false, takes: (shape) => shape.exact, text: (it) => `let it = ${it};\n    println!("len {}", it.len());` },
  {
    name: "for_each",
    closure: true,
    takes: any,
    text: (it, p) => `${it}.for_each(|x| { ${call(p)} println!("item {}", x.v()); });\n    println!("for_each after {seen}");`,
  },
  {
    name: "fold holding",
    closure: true,
    takes: any,
    text: (it, p) => `${holds}let acc = ${it}.fold(Acc(0), ${holding(p, "mut a, x", "a.0 += x.v(); a")});\n    println!("fold {} after {seen}", acc.0);`,
  },
  {
    // The same made by a block given to the call, which runs as the
    // statement form does since #759's fix.
    name: "fold holding block",
    closure: true,
    takes: any,
    text: (it, p) =>
      `let acc = ${it}.fold(Acc(0), { let g = D(90); let counter = &mut seen; ${holding(p, "mut a, x", "a.0 += x.v(); a")} });\n    println!("fold {} after {seen}", acc.0);`,
  },
  {
    name: "rfold holding",
    closure: true,
    takes: back,
    text: (it, p) => `${holds}let acc = ${it}.rfold(Acc(0), ${holding(p, "mut a, x", "a.0 += x.v(); a")});\n    println!("rfold {} after {seen}", acc.0);`,
  },
  {
    name: "for_each holding",
    closure: true,
    takes: any,
    text: (it, p) => `${holds}${it}.for_each(${holding(p, "x", 'println!("item {}", x.v());')});\n    println!("for_each after {seen}");`,
  },
  {
    name: "try_fold holding",
    closure: true,
    takes: any,
    lends: true,
    text: (it, p) =>
      `${holds}let r = ${it}.try_fold(Acc(0), ${holding(p, "mut a, x", "a.0 += x.v(); if *counter == 3 { Err(a.0) } else { Ok(a) }")});\n    match &r { Ok(a) => println!("ok {}", a.0), Err(n) => println!("err {n}") }`,
  },
  {
    name: "any holding",
    closure: true,
    takes: any,
    lends: true,
    text: (it, p) => `${holds}let r = ${it}.any(${holding(p, "x", "x.v() % 3 == 0")});\n    println!("any {r} after {seen}");`,
  },
];

// The crate's own `Hints` and `Exact` print as their `size_hint` runs, which
// a collect into a `Vec` reserves by (#733).
const tableSources = ["vec", "rev vec", "range", "vec iter", "own Plain", "own Hints", "own Exact", "own Every", "own Backs", "generic vec", "generic own Every"];
// The sources whose `size_hint` prints as it runs, of which alone a
// consumer that asks it shows when (#820).
const hinting = ["own Hints", "own Exact", "own Every", "generic own Every"];

/** Every case of the conformance table, in one order: each source alone and
 * through each adapter, each step that takes it, each panic site. */
export function conformanceCases(): Case[] {
  const out: Case[] = [];
  const steps = [...consumers, ...stepConsumers];
  for (const source of tableSources.map((name) => sources.find((s) => s.name === name)!)) {
    const chains: [Adapter[], Shape][] = [[[], source.shape]];
    for (const a of [...adapters, ...stepAdapters]) {
      const shape = a.shape(source.shape);
      if (shape) chains.push([[a], shape]);
    }
    for (const [chain, shape] of chains) {
      const shapes = [source.shape, shape];
      for (const consumer of steps) {
        if (!consumer.takes(shape)) continue;
        if (consumer.asks && !hinting.includes(source.name)) continue;
        // A closure made by a block given to the call (#759): a few of it,
        // of a `Vec`'s, the consumer's alone and the adapter's folded, the
        // statement form's other shapes being the same closure.
        const block = (name: string) => name.endsWith(" block");
        const blockCase =
          source.name === "vec" &&
          ((chain.length === 0 && block(consumer.name)) || (chain.length === 1 && block(chain[0].name) && consumer.name === "fold"));
        if ((block(consumer.name) || chain.some((a) => block(a.name))) && !blockCase) continue;
        for (const site of sites) {
          if (!applies(site, source, chain, consumer, shapes)) continue;
          const id = `${source.name} | ${chain.map((a) => a.name).join(".") || "-"} | ${consumer.name} | ${site}`;
          out.push({ n: out.length, id, source, chain, consumer, site, variant: { kind: "plain" } });
        }
      }
    }
  }
  return out;
}

/** Every case of the points table (#817): each of the conformance table's
 * that panics nowhere, run again at each of its points in turn, each a
 * closure's call, a value's drop, a clone, or a step or the drop of the
 * crate's iterator (`point()`), so each step of each adapter is seen
 * unwinding from every point it has, in both builds. */
export function pointsCases(): Case[] {
  return conformanceCases()
    .filter((c) => c.site === "none")
    .map((c, n) => ({ ...c, n, id: c.id.replace(/ \| none$/, " | points"), site: "points" as const }));
}

/** What test/iterator-matrix.test.ts judges of a case of any table: its
 * number in its table's cases, and its id, which its program prints as
 * `case <id>` as it begins. */
export type Row = { n: number; id: string };


/** The cases in programs of `perBatch`, in their order, but each that
 * `alone` says ends its program, a program of its own. */
export function batches<C extends Row>(cases: C[], perBatch = 60, alone: (c: C) => boolean = () => false): C[][] {
  const out: C[][] = [];
  // One that aborts ends its program, so it's a program of its own, after
  // the others, whose batches are as before (#847).
  const others = cases.filter((c) => !alone(c));
  for (let i = 0; i < others.length; i += perBatch) out.push(others.slice(i, i + perBatch));
  for (const c of cases) if (alone(c)) out.push([c]);
  return out;
}

/** Of the iterator tables, a case that aborts natively (#847). */
export const aborts = (c: Case) => c.variant.kind === "aborts";

/** What a change to what every adapter shares runs, of `cases` (#818): of
 * each pairing of an adapter, or of none, with a consumer, `perPair` of its
 * cases, `sample`'s, in their order. */
export function pairs(cases: Case[], perPair: number): Case[] {
  const of = new Map<string, Case[]>();
  for (const c of cases) {
    for (const adapter of c.chain.length === 0 ? ["-"] : c.chain.map((a) => a.name)) {
      const pairing = `${adapter} | ${c.consumer.name}`;
      of.set(pairing, [...(of.get(pairing) ?? []), c]);
    }
  }
  const picked = new Set<Case>();
  [...of.keys()].sort().forEach((pairing, seed) => {
    for (const c of sample(of.get(pairing)!, seed + 1, perPair)) picked.add(c);
  });
  return cases.filter((c) => picked.has(c));
}

/** `count` of `items`, the seed saying which, in their order. */
export function sample<T>(items: T[], seed: number, count: number): T[] {
  const { int } = random(seed);
  const pool = items.map((_, i) => i);
  const picked: number[] = [];
  while (picked.length < count && pool.length > 0) picked.push(pool.splice(int(pool.length), 1)[0]);
  return picked.sort((a, b) => a - b).map((i) => items[i]);
}

/** What a slice of the matrix covers, of a case, each a pairing: of a
 * plain case, its source, each adapter of its chain and where it panics,
 * each with its consumer, and its source and consumer with where it
 * panics; of a capturing one, each adapter with its consumer, and its
 * source and consumer with where it panics; of a paired one, its setter
 * with its source and chain, and with its consumer. */
export function strata(c: Case): string[] {
  const { source, chain, consumer, site, variant } = c;
  const kind = variant.kind === "paired" ? `after ${variant.setter}` : variant.kind;
  // Each a program of its own: the slice runs one of each consumer's.
  if (variant.kind === "aborts") return [`${kind} | consumer ${consumer.name}`];
  if (variant.kind === "paired") return [`${kind} | source ${source.name} | chain ${chain.map((a) => a.name).join(".")}`, `${kind} | consumer ${consumer.name}`];
  return [
    `${kind} | source ${source.name} | consumer ${consumer.name}`,
    `${kind} | source ${source.name} | site ${site}`,
    `${kind} | consumer ${consumer.name} | site ${site}`,
    ...chain.flatMap((a) => [`${kind} | adapter ${a.name} | consumer ${consumer.name}`, ...(variant.kind === "plain" ? [`plain | adapter ${a.name} | site ${site}`] : [])]),
  ];
}

/** The matrix's slice, which a push that changes the iterators runs
 * (scripts/iterators.ts): the fewest whole batches, picked greedily, the
 * first of equals, whose cases between them have every stratum any case
 * has, each pairing of a source, an adapter, a consumer and a panic site
 * (`strata`), so every adapter and consumer runs in both builds; of
 * another table, its strata. Fixed by the cases alone, so a failure is
 * found again by running it again. */
export function slice<C extends Row>(all: C[][], strataOf: (c: C) => string[]): number[] {
  const covers = all.map((batch) => new Set(batch.flatMap(strataOf)));
  const left = new Set(covers.flatMap((strata) => [...strata]));
  const picked: number[] = [];
  while (left.size > 0) {
    let best = 0;
    let most = 0;
    covers.forEach((strata, i) => {
      const count = [...strata].filter((s) => left.has(s)).length;
      if (count > most) [best, most] = [i, count];
    });
    picked.push(best);
    for (const s of covers[best]) left.delete(s);
  }
  return picked.sort((a, b) => a - b);
}

/** A case's function, `case_<n>`, and for a generic source the generic
 * one its chain is in. */
function caseFn(c: Case, n: number): string {
  const lines: string[] = [];
  const name = `case_${n}`;
  const { source, chain, consumer, site } = c;
  const body: string[] = [];
  if (site === "drop" || site === "drop+drop") body.push("BOOM.store(3, Ordering::Relaxed);");
  if (site === "drop+drop") body.push("TWICE.store(1, Ordering::Relaxed);");
  if (source.before && !source.generic) body.push(source.before);
  if (chain.some((a) => a.name === "chain") && source.shape.item === "&D") body.push("let tail = vec![D(6), D(7)];");
  if (site === "stage") body.push("let mut calls = 0;");
  if (consumer.closure) body.push("let mut seen = 0;");
  let shape = source.shape;
  let expr = source.generic ? "src" : source.expr(site);
  let panicked = false;
  for (const a of chain) {
    const panics: Panics = site === "points" ? "point" : site === "stage" && a.closure && !panicked;
    if (panics) panicked = true;
    if (a.name === "peekable") {
      // Peeked first, so what it holds is in each step after.
      body.push(`let mut it = ${expr}.peekable();`, `println!("peeked {:?}", it.peek().map(|x| x.v()));`);
      expr = "it";
    } else if (a.name === "map holding") {
      // What its closure holds, moved in.
      body.push("let h = D(80);");
      expr += a.text(shape, panics);
    } else if (a.name === "by_ref") {
      // Lent: what the consumer leaves is dropped as `held` is, after it.
      body.push(`let mut held = ${expr};`);
      expr = "held.by_ref()";
    } else {
      expr += a.text(shape, panics);
    }
    shape = a.shape(shape)!;
  }
  const { variant } = c;
  if (variant.kind === "paired") {
    // Of the same type: of a chain with a closure, each closure a type of
    // its own, by one closure that makes it; else the chain written again.
    if (chain.some((a) => a.closure)) {
      body.push(`let make = || ${expr};`);
      expr = "make()";
    }
    body.push(setterText(variant.setter, expr));
  }
  body.push(consumer.text(expr, site === "points" ? "point" : site === "consumer"));
  if (variant.kind === "captures") {
    const captured = capturing(body.join("\n    "));
    body.splice(0, body.length, ...Array.from({ length: captured.guards }, (_, k) => `let g${k + 1} = D(${91 + k});`), captured.text);
  }
  const inner = body.map((line) => `    ${line}`).join("\n");
  if (source.generic) {
    lines.push(`fn ${name}() {\n    ${name}_of(${source.expr(site)});\n}`);
    // `try_fold`, `any` and `all` take `&mut self`: of the parameter
    // itself, it's `mut`.
    const held = chain.length === 0 && consumer.lends ? "mut src" : "src";
    lines.push(`fn ${name}_of<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(${held}: I) {\n${inner}\n}`);
  } else {
    lines.push(`fn ${name}() {\n${inner}\n}`);
  }
  return lines.join("\n\n");
}

/** The crate's iterator `own`, of `D(1)` to `D(5)`, its `next` panicking
 * as it would yield `D(boom)`; its `size_hint` printing as it runs where
 * `hints` says, the conformance table's, so who asks it shows (#733). */
function ownIterator(own: Own, hints: boolean): string {
  const has = (step: Own["steps"][number]) => own.steps.includes(step);
  const steps: string[] = [];
  const said = hints ? `println!("own size_hint {n}");\n        ` : "";
  if (has("size_hint")) steps.push(`    fn size_hint(&self) -> (usize, Option<usize>) {\n        let n = (self.end - self.at) as usize;\n        ${said}(n, Some(n))\n    }`);
  if (has("fold"))
    steps.push(
      `    fn fold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {\n        println!("own fold");\n        let mut acc = init;\n        while let Some(x) = self.next() {\n            acc = f(acc, x);\n        }\n        acc\n    }`,
    );
  if (has("nth"))
    steps.push(
      `    fn nth(&mut self, n: usize) -> Option<D> {\n        println!("own nth {n}");\n        for _ in 0..n {\n            self.next()?;\n        }\n        self.next()\n    }`,
    );
  if (has("last"))
    steps.push(
      `    fn last(mut self) -> Option<D> {\n        println!("own last");\n        let mut last = None;\n        while let Some(x) = self.next() {\n            last = Some(x);\n        }\n        last\n    }`,
    );
  if (has("count"))
    steps.push(`    fn count(mut self) -> usize {\n        println!("own count");\n        let mut n = 0;\n        while self.next().is_some() {\n            n += 1;\n        }\n        n\n    }`);
  const back: string[] = [];
  if (own.back) {
    back.push(`    fn next_back(&mut self) -> Option<D> {\n        if self.at == self.end {\n            return None;\n        }\n        self.end -= 1;\n        yielded(self.end, self.boom)\n    }`);
    if (has("rfold"))
      back.push(
        `    fn rfold<B, F: FnMut(B, D) -> B>(mut self, init: B, mut f: F) -> B {\n        println!("own rfold");\n        let mut acc = init;\n        while let Some(x) = self.next_back() {\n            acc = f(acc, x);\n        }\n        acc\n    }`,
      );
    if (has("nth_back"))
      back.push(
        `    fn nth_back(&mut self, n: usize) -> Option<D> {\n        println!("own nth_back {n}");\n        for _ in 0..n {\n            self.next_back()?;\n        }\n        self.next_back()\n    }`,
      );
  }
  return [
    `struct ${own.name} {\n    at: u32,\n    end: u32,\n    boom: u32,\n}`,
    `impl ${own.name} {\n    fn new(boom: u32) -> Self {\n        ${own.name} { at: 1, end: 6, boom }\n    }\n}`,
    `impl Drop for ${own.name} {\n    fn drop(&mut self) {\n        println!("drop src");\n    }\n}`,
    `impl Iterator for ${own.name} {\n    type Item = D;\n    fn next(&mut self) -> Option<D> {\n        if self.at == self.end {\n            return None;\n        }\n        self.at += 1;\n        yielded(self.at - 1, self.boom)\n    }\n${steps.join("\n")}\n}`,
    ...(own.exact ? [`impl ExactSizeIterator for ${own.name} {}`] : []),
    ...(own.back ? [`impl DoubleEndedIterator for ${own.name} {\n${back.join("\n")}\n}`] : []),
  ].join("\n\n");
}

const prelude = `// Generated by test/iterator-matrix.ts (ADR 1090). Each case prints its
// name, what it prints and drops, and whether it panicked.
use std::collections::{HashMap, HashSet};
use std::iter::Sum;
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicU32, Ordering};

/** The item whose drop panics, or none. */
static BOOM: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.load(Ordering::Relaxed) {
            BOOM.store(0, Ordering::Relaxed);
            panic!("drop");
        }
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        D(self.0)
    }
}

/** An accumulator, which prints as it's dropped. */
struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

/** What \`sum\` makes, which prints as it's dropped. */
struct Total(u32);

impl Drop for Total {
    fn drop(&mut self) {
        println!("drop total {}", self.0);
    }
}

impl Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Total {
        iter.fold(Total(0), |mut t, d| {
            t.0 += d.0;
            t
        })
    }
}

impl<'a> Sum<&'a D> for Total {
    fn sum<I: Iterator<Item = &'a D>>(iter: I) -> Total {
        iter.fold(Total(0), |mut t, d| {
            t.0 += d.0;
            t
        })
    }
}

impl Sum<u32> for Total {
    fn sum<I: Iterator<Item = u32>>(iter: I) -> Total {
        Total(iter.fold(0, |t, n| t + n))
    }
}

impl<'a> Sum<&'a u32> for Total {
    fn sum<I: Iterator<Item = &'a u32>>(iter: I) -> Total {
        Total(iter.fold(0, |t, n| t + n))
    }
}

/** An item's number. */
trait V {
    fn v(&self) -> u32;
}

impl V for D {
    fn v(&self) -> u32 {
        self.0
    }
}

impl V for u32 {
    fn v(&self) -> u32 {
        *self
    }
}

impl V for usize {
    fn v(&self) -> u32 {
        *self as u32
    }
}

impl<T: V + ?Sized> V for &T {
    fn v(&self) -> u32 {
        (**self).v()
    }
}

impl<A: V, B: V> V for (A, B) {
    fn v(&self) -> u32 {
        self.0.v() * 1000 + self.1.v()
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

/** A crate iterator's item \`n\`, or its panic. */
fn yielded(n: u32, boom: u32) -> Option<D> {
    if n == boom {
        panic!("next");
    }
    Some(D(n))
}

/** The items' numbers, by an index, not an iterator. */
fn show<T: V>(items: &[T]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < items.len() {
        out.push_str(&format!(" {}", items[i].v()));
        i += 1;
    }
    out
}

fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

fn run(name: &str, case: fn()) {
    println!("case {name}");
    let ended = catch_unwind(case);
    BOOM.store(0, Ordering::Relaxed);
    match ended {
        Ok(()) => println!("ended"),
        Err(payload) => println!("panicked: {}", payload.downcast_ref::<&str>().copied().unwrap_or("?")),
    }
}
`;

/** `text`, the prelude or the crate's iterators, with the points table's
 * points: after each drop and clone prints, and first in each `next` and
 * `next_back` of the crate's iterator. */
function withPoints(text: string): string {
  const points: [string, string][] = [
    ['println!("drop {}", self.0);', 'println!("drop {}", self.0);\n        point();'],
    ['println!("clone {}", self.0);', 'println!("clone {}", self.0);\n        point();'],
    ['println!("drop acc {}", self.0);', 'println!("drop acc {}", self.0);\n        point();'],
    ['println!("drop total {}", self.0);', 'println!("drop total {}", self.0);\n        point();'],
    ['println!("drop src");', 'println!("drop src");\n        point();'],
    ["-> Option<D> {\n        if self.at == self.end {", "-> Option<D> {\n        point();\n        if self.at == self.end {"],
  ];
  return points.reduce((out, [from, to]) => out.replaceAll(from, to), text);
}

/** The prelude of a program of a case of `drop+drop` (#847): once the
 * drop that panics has, the next drop panics too, which natively aborts
 * where it's as the first unwinds, and ends the case otherwise. */
function withTwice(text: string): string {
  const twice: [string, string][] = [
    [
      "static BOOM: AtomicU32 = AtomicU32::new(0);\n",
      "static BOOM: AtomicU32 = AtomicU32::new(0);\n\n/** Of a case of `drop+drop`: 1 once it's set, 2 once that drop has panicked. */\nstatic TWICE: AtomicU32 = AtomicU32::new(0);\n",
    ],
    [
      'println!("drop {}", self.0);\n        if self.0 == BOOM.load(Ordering::Relaxed) {\n            BOOM.store(0, Ordering::Relaxed);\n',
      'println!("drop {}", self.0);\n        if TWICE.load(Ordering::Relaxed) == 2 {\n            TWICE.store(0, Ordering::Relaxed);\n            panic!("drop again");\n        }\n        if self.0 == BOOM.load(Ordering::Relaxed) {\n            BOOM.store(0, Ordering::Relaxed);\n            if TWICE.load(Ordering::Relaxed) == 1 {\n                TWICE.store(2, Ordering::Relaxed);\n            }\n',
    ],
    ["    BOOM.store(0, Ordering::Relaxed);\n    match ended", "    BOOM.store(0, Ordering::Relaxed);\n    TWICE.store(0, Ordering::Relaxed);\n    match ended"],
  ];
  return twice.reduce((out, [from, to]) => {
    if (!out.includes(from)) throw new Error(`withTwice: the prelude has no ${JSON.stringify(from)}`);
    return out.replace(from, to);
  }, text);
}

/** A program of `cases`, which runs each in turn. */
export function program(cases: Case[], of: Table = table): string {
  const used = new Set(cases.map((c) => c.source.name.replace(/^generic /, "")));
  const iterators = owns.filter((own) => used.has(`own ${own.name}`)).map((own) => ownIterator(own, of !== "matrix"));
  const fns = cases.map((c, n) => caseFn(c, n));
  const runner = of === "points" ? "run_points" : "run";
  const main = `fn main() {\n${cases.map((c, n) => `    ${runner}(${JSON.stringify(c.id)}, case_${n});`).join("\n")}\n}`;
  if (cases.some((c) => c.site === "drop+drop")) {
    return [withTwice(prelude), ...iterators, ...fns, main].join("\n\n") + "\n";
  }
  if (of !== "points") return [prelude, ...iterators, ...fns, main].join("\n\n") + "\n";
  return [withPoints(prelude) + pointsRunner, ...iterators.map(withPoints), ...fns, main].join("\n\n") + "\n";
}

/** Each case's lines in a run's output, by its id: from its `case` line
 * to the next. What came before the first is under "". */
export function segments(stdout: string): Map<string, string> {
  const out = new Map<string, string>();
  let id = "";
  let lines: string[] = [];
  // Without the last line's newline, so the last case's lines are as another's.
  for (const line of stdout.replace(/\n$/, "").split("\n")) {
    if (line.startsWith("case ")) {
      out.set(id, lines.join("\n"));
      id = line.slice("case ".length);
      lines = [];
    } else {
      lines.push(line);
    }
  }
  out.set(id, lines.join("\n"));
  return out;
}

const header = `// The iterator matrix's cases (ADR 1090), the conformance table's (ADR
// 1092), the points table's (#817), the unwind table's (#854), the
// formatting grid's (#941), the numbers grid's (#940), the async table's
// (#942) and the consumer net's that don't run as native Rust does yet,
// each under its issue, \`#<issue>
// <title>\`: a line a build, \`plain\` or \`std\` of the matrix's,
// \`conformance:plain\` or \`unwind:std\` of a table's, the numbers of its
// cases in \`allCases()\`, \`conformanceCases()\`, \`pointsCases()\`,
// \`unwindCases()\`, \`formatCases()\`, \`numberCases()\`,
// \`asyncCases()\` or \`consumerCases()\` as ranges. Each listed
// must still fail, so the list only shrinks: \`bun test/iterator-matrix.ts
// take-off\` takes off what the last runs found fixed. Its numbers are of
// each table's generator version below; a change to a table's cases makes
// its rows again, with each issue's cases found again.
`;

// `bun test/iterator-matrix.ts take-off`: the known failures the last runs
// of either table found running as Rust does, taken off the list. `add
// <issue> <title> <regex>`: the failures written to target/iterator-matrix/
// whose names match, `plain: ..` or `conformance:plain: ..`, put under the
// issue, for an issue filed for them. `program <n>`: case n's program
// alone, of `ITERATOR_TABLE`'s table, to run it natively and as JS.
if (import.meta.main) {
  const [command, ...args] = process.argv.slice(2);
  const known = readKnown();
  for (const of of tables) {
    const current = version(casesOf(of));
    const listed = known.versions[of];
    const rows = known.issues.some(({ cases }) => [...cases].some(([build, numbers]) => ofTable(build, of) && numbers.length > 0));
    if (listed !== undefined && listed !== current && rows) {
      throw new Error(`the list's ${of} is of the generator's ${listed}, its cases are ${current}: make it again, each issue's cases found again`);
    }
    known.versions[of] = current;
  }
  if (command === "take-off") {
    const fixed = new Set(existsSync(fixedFile) ? readFileSync(fixedFile, "utf8").split("\n").filter(Boolean) : []);
    for (const { cases } of known.issues) {
      for (const [build, numbers] of cases) cases.set(build, numbers.filter((n) => !fixed.has(`${build} ${n}`)));
    }
    writeFileSync(knownFile, printKnown(known, header));
    writeFileSync(fixedFile, "");
    console.log(`took ${fixed.size} off`);
  } else if (command === "add" && args.length === 3) {
    const [issue, title, matching] = args;
    const re = new RegExp(matching);
    const listed = knownCases(known);
    const found = readdirSync(failures)
      .filter((file) => file.endsWith(".json"))
      .map((file) => JSON.parse(readFileSync(join(failures, file), "utf8")) as { name: string; build: string; n: number })
      .filter(({ name, build, n }) => re.test(name) && !listed.has(`${build} ${n}`));
    let at = known.issues.find(({ heading }) => heading.startsWith(`#${issue} `));
    if (!at) known.issues.push((at = { heading: `#${issue} ${title}`, cases: new Map() }));
    for (const { build, n } of found) at.cases.set(build, [...(at.cases.get(build) ?? []), n]);
    writeFileSync(knownFile, printKnown(known, header));
    console.log(`added ${found.length} under #${issue}`);
  } else if (command === "program" && args.length === 1) {
    const of = tableOf(table);
    const c = of.cases()[Number(args[0])];
    if (!c) throw new Error(`no case ${args[0]}`);
    process.stdout.write(of.program([c]));
  } else {
    throw new Error("`take-off`, `add <issue> <title> <regex>`, or `program <n>`, case n's program alone");
  }
}
