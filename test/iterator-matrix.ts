// The iterator conformance matrix (ADR 1092): each source, chain of
// adapters, consumer and panic site, as a case of a program whose items
// and accumulators print as they're dropped, run under `catch_unwind`.
// test/iterator-matrix.test.ts compiles each batch of cases natively and
// as JS, in both builds, and compares each case's lines.
//
// A combination that isn't Rust isn't made: each adapter says what it
// takes and makes (an item, double-ended, exact-sized), each consumer what
// item it takes, and each panic site where it can be. Nothing is skipped
// by a list.

import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { random } from "./generate";
import { root, target } from "./support";

/** The builds each case is compiled in: the recognizers' rewrites, and
 * std's iterator structs alone (ADR 0908). */
export const builds: [name: string, flags: string[]][] = [
  ["plain", []],
  ["std", ["--std-iterators"]],
];

/** Where each failure's program and lines are written. */
export const failures = join(target, "iterator-matrix");
/** The known failures a run found running as Rust does, one a line. */
export const fixedFile = join(failures, "fixed.txt");
const knownFile = join(root, "test", "iterator-matrix-known.txt");

/** The known failures, by `<build> <n>`, a case's build and its number
 * in `allCases()`, each one's issue. Under a line `#<issue> <title>`, a line
 * a build, `plain 120-179,4410`, the numbers of its cases as sorted ranges;
 * `//` a comment. Its `// generator <version>` line is the version of the
 * cases its numbers are of. */
export type Known = { version: string | undefined; issues: { heading: string; cases: Map<string, number[]> }[] };

export function parseKnown(text: string): Known {
  const known: Known = { version: undefined, issues: [] };
  for (const line of text.split("\n")) {
    const version = /^\/\/ generator (\w+)$/.exec(line);
    if (version) known.version = version[1];
    if (line === "" || line.startsWith("//")) continue;
    if (/^#\d+ /.test(line)) {
      known.issues.push({ heading: line, cases: new Map() });
      continue;
    }
    const at = known.issues.at(-1);
    const row = /^(\w+) (\d+(?:-\d+)?(?:,\d+(?:-\d+)?)*)$/.exec(line);
    if (!at || !row) throw new Error(`test/iterator-matrix-known.txt: \`${line}\` is no build's cases under an issue`);
    const numbers = row[2].split(",").flatMap((range) => {
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

/** The list's text: `header`, its version, each issue's builds' ranges. */
export function printKnown(known: Known, header: string): string {
  const issues = known.issues
    .map(({ heading, cases }) => {
      const rows = [...cases].filter(([, numbers]) => numbers.length > 0).sort(([a], [b]) => a.localeCompare(b));
      return rows.length === 0 ? "" : `\n${heading}\n${rows.map(([build, numbers]) => `${build} ${ranges(numbers)}`).join("\n")}\n`;
    })
    .join("");
  return `${header}// generator ${known.version}\n${issues}`;
}

export const readKnown = () => parseKnown(existsSync(knownFile) ? readFileSync(knownFile, "utf8") : "");

/** The cases' version: of their names, in their order, so a change to what
 * the generator makes, or in what order, is another. */
export function version(cases: Case[]): string {
  return new Bun.CryptoHasher("sha256").update(cases.map((c) => c.id).join("\n")).digest("hex").slice(0, 12);
}

/** What an iterator yields: an item that prints as it's dropped, `D`, a
 * number, a reference to either, or a pair of them. */
type Item = "D" | "N" | "&D" | "&N" | "pair";

/** An iterator so far: what it yields, whether a `D` it owns is in it
 * (whose `Drop` may panic), and the traits it has. */
type Shape = { item: Item; owns: boolean; back: boolean; exact: boolean };

/** Where a case panics: nowhere, in the source's `next`, in the chain's
 * first closure, in the consumer's closure, or in an item's `Drop`. */
export type Site = "none" | "next" | "stage" | "consumer" | "drop";
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
type Adapter = { name: string; closure: boolean; shape: (from: Shape) => Shape | undefined; text: (from: Shape, panics: boolean) => string };

const base = (item: Item) => item !== "pair";
const owned = (item: Item) => item === "D" || item === "N";
/** A closure's body, after a counter of its calls panics at the second, if
 * it's the one that does. */
const counted = (panics: boolean, what: string, body: string) =>
  panics ? `{ calls += 1; if calls == 2 { panic!("${what}") } ${body} }` : body;

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
  text: (it: string, panics: boolean) => string;
};

/** A consumer's closure's start: its call counted, and a panic at the
 * second call if it's the one that panics. */
const call = (panics: boolean) => `seen += 1;${panics ? ` if seen == 2 { panic!("consumer") }` : ""}`;
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
    // Broken off at the third item, the accumulator dropped where it is.
    text: (it, p) =>
      `let r = ${it}.try_fold(Acc(0), |mut a, x| { ${call(p)} a.0 += x.v(); if seen == 3 { Err(a.0) } else { Ok(a) } });\n    match &r { Ok(a) => println!("ok {}", a.0), Err(n) => println!("err {n}") }`,
  },
  { name: "any", closure: true, takes: any, text: (it, p) => `let r = ${it}.any(|x| { ${call(p)} x.v() % 3 == 0 });\n    println!("any {r} after {seen}");` },
  { name: "all", closure: true, takes: any, text: (it, p) => `let r = ${it}.all(|x| { ${call(p)} x.v() % 4 != 0 });\n    println!("all {r} after {seen}");` },
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

export type Case = { n: number; id: string; source: Source; chain: Adapter[]; consumer: Consumer; site: Site };

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
      return shapes.some((shape) => shape.owns);
  }
}

/** Every case of the matrix, in one order, each id its own. */
export function allCases(): Case[] {
  const out: Case[] = [];
  for (const source of sources) {
    for (const [chain, shape] of chains(source.shape)) {
      const shapes = [source.shape];
      for (const a of chain) shapes.push(a.shape(shapes.at(-1)!)!);
      for (const consumer of consumers) {
        if (!consumer.takes(shape)) continue;
        for (const site of sites) {
          if (!applies(site, source, chain, consumer, shapes)) continue;
          const id = `${source.name} | ${chain.map((a) => a.name).join(".") || "-"} | ${consumer.name} | ${site}`;
          out.push({ n: out.length, id, source, chain, consumer, site });
        }
      }
    }
  }
  return out;
}

/** The cases in programs of `perBatch`, in their order. */
export function batches(cases: Case[], perBatch = 60): Case[][] {
  const out: Case[][] = [];
  for (let i = 0; i < cases.length; i += perBatch) out.push(cases.slice(i, i + perBatch));
  return out;
}

/** `count` of `items`, the seed saying which, in their order. */
export function sample<T>(items: T[], seed: number, count: number): T[] {
  const { int } = random(seed);
  const pool = items.map((_, i) => i);
  const picked: number[] = [];
  while (picked.length < count && pool.length > 0) picked.push(pool.splice(int(pool.length), 1)[0]);
  return picked.sort((a, b) => a - b).map((i) => items[i]);
}

/** A case's function, `case_<n>`, and for a generic source the generic
 * one its chain is in. */
function caseFn(c: Case, n: number): string {
  const lines: string[] = [];
  const name = `case_${n}`;
  const { source, chain, consumer, site } = c;
  const body: string[] = [];
  if (site === "drop") body.push("BOOM.store(3, Ordering::Relaxed);");
  if (source.before && !source.generic) body.push(source.before);
  if (chain.some((a) => a.name === "chain") && source.shape.item === "&D") body.push("let tail = vec![D(6), D(7)];");
  if (site === "stage") body.push("let mut calls = 0;");
  if (consumer.closure) body.push("let mut seen = 0;");
  let shape = source.shape;
  let expr = source.generic ? "src" : source.expr(site);
  let panicked = false;
  for (const a of chain) {
    const panics = site === "stage" && a.closure && !panicked;
    if (panics) panicked = true;
    if (a.name === "peekable") {
      // Peeked first, so what it holds is in each step after.
      body.push(`let mut it = ${expr}.peekable();`, `println!("peeked {:?}", it.peek().map(|x| x.v()));`);
      expr = "it";
    } else {
      expr += a.text(shape, panics);
    }
    shape = a.shape(shape)!;
  }
  body.push(consumer.text(expr, site === "consumer"));
  const inner = body.map((line) => `    ${line}`).join("\n");
  if (source.generic) {
    lines.push(`fn ${name}() {\n    ${name}_of(${source.expr(site)});\n}`);
    // `try_fold`, `any` and `all` take `&mut self`: of the parameter
    // itself, it's `mut`.
    const held = chain.length === 0 && ["try_fold", "any", "all"].includes(consumer.name) ? "mut src" : "src";
    lines.push(`fn ${name}_of<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(${held}: I) {\n${inner}\n}`);
  } else {
    lines.push(`fn ${name}() {\n${inner}\n}`);
  }
  return lines.join("\n\n");
}

/** The crate's iterator `own`, of `D(1)` to `D(5)`, its `next` panicking
 * as it would yield `D(boom)`. */
function ownIterator(own: Own): string {
  const has = (step: Own["steps"][number]) => own.steps.includes(step);
  const steps: string[] = [];
  if (has("size_hint")) steps.push(`    fn size_hint(&self) -> (usize, Option<usize>) {\n        let n = (self.end - self.at) as usize;\n        (n, Some(n))\n    }`);
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

const prelude = `// Generated by test/iterator-matrix.ts (ADR 1092). Each case prints its
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

/** A program of `cases`, which runs each in turn. */
export function program(cases: Case[]): string {
  const used = new Set(cases.map((c) => c.source.name.replace(/^generic /, "")));
  const iterators = owns.filter((own) => used.has(`own ${own.name}`)).map(ownIterator);
  const fns = cases.map((c, n) => caseFn(c, n));
  const main = `fn main() {\n${cases.map((c, n) => `    run(${JSON.stringify(c.id)}, case_${n});`).join("\n")}\n}`;
  return [prelude, ...iterators, ...fns, main].join("\n\n") + "\n";
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

const header = `// The iterator matrix's cases that don't run as native Rust does yet (ADR
// 1090), each under its issue, \`#<issue> <title>\`: a line a build, the
// numbers of its cases in \`allCases()\` as ranges. Each listed must still
// fail, so the list only shrinks: \`bun test/iterator-matrix.ts take-off\`
// takes off what the last runs found fixed. Its numbers are of the
// generator's version below; a change to the cases makes the list again,
// with each issue's cases found again.
`;

// `bun test/iterator-matrix.ts take-off`: the known failures the last runs
// found running as Rust does, taken off the list. `add <issue> <title>
// <regex>`: the failures written to target/iterator-matrix/ whose names
// match, put under the issue, for an issue filed for them. `program <n>`:
// case n's program alone, to run it natively and as JS.
if (import.meta.main) {
  const [command, ...args] = process.argv.slice(2);
  const known = readKnown();
  const current = version(allCases());
  if (known.version !== undefined && known.version !== current && known.issues.length > 0) {
    throw new Error(`the list is of the generator's ${known.version}, the cases are ${current}: make it again, each issue's cases found again`);
  }
  known.version = current;
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
    const c = allCases()[Number(args[0])];
    if (!c) throw new Error(`no case ${args[0]}`);
    process.stdout.write(program([c]));
  } else {
    throw new Error("`take-off`, `add <issue> <title> <regex>`, or `program <n>`, case n's program alone");
  }
}
