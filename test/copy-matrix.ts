// The copy matrix (ITERATOR_TABLE=copies): every compiled iterator form,
// each source of the iterator matrix's that's `Clone` and each adapter
// chain over it, plain and through generic code, copied each way a program
// copies an iterator, each copy and its original stepped interleaved from
// both ends by each ability an iterator has. What a value knows is attached
// where it's used, not carried in its type, in a compiled form (a std
// struct's `$knowing`, an in-place maker's `$cloning`, generic code's
// `$given` wrapper): a copy that loses it steps wrong, as #1973, #1974 and
// #1976 did, each found by hand. Run by test/iterator-matrix.test.ts in
// both builds against native Rust, as the iterator tables are; a per-push
// slice where the iterators' code changes, all of it nightly.
//
// The ways of copying, or of handing it on:
//   clone      `b = a.clone()`
//   dup        `b = dup(&a)`, a clone inside generic code, `I: Clone`
//   pass       `b = pass(a)`, a move through an opaque generic identity
//   by_ref     `b = pass(a.by_ref())`, a lend through the same, then `a` again
//   dyn        `b: &mut dyn DoubleEndedIterator = &mut a`, then `a` again
//   steps      `back_of(a)`, a callee that steps it from its back and gives
//              back only the item, by `next_back` alone
// A move apart, an iterator moved into what a JS unit holds, is the
// runtime package's rows' (ADR 0561, amended with #1976), not here.
//
// The abilities: `next`, `next_back`, `len`, `size_hint`, `nth`,
// `nth_back`, `fold`, `rfold`, `try_fold`, `try_rfold`, each where the
// chain's shape has it (`back`, `exact`). Each case steps the original
// once, copies it, then steps the copy by its ability and the original from
// its other end, interleaved, and drains what's left of each, every item's
// value, clone and drop printed.

import { adapters, sources, stepAdapters, type Row } from "./iterator-matrix";
import { prelude } from "./iterator-matrix-prelude";

type Source = (typeof sources)[number];
type Adapter = (typeof adapters)[number];
type Shape = Source["shape"];

/** A way of copying an iterator. */
export type Way = "clone" | "dup" | "pass" | "by_ref" | "dyn" | "steps";
export const ways: Way[] = ["clone", "dup", "pass", "by_ref", "dyn", "steps"];

/** Of a way, the abilities it steps by: a `dyn`'s, those a trait object
 * has, not `Sized`'s folds nor `ExactSizeIterator`'s `len`; a callee's,
 * `next_back` alone. */
const waysAbilities: Partial<Record<Way, string[]>> = {
  dyn: ["next", "next_back", "nth", "nth_back", "size_hint"],
  steps: ["next_back"],
};

/** An ability of an iterator: what it needs of the chain's shape, and
 * whether it consumes the iterator. */
type Ability = { name: string; back?: boolean; exact?: boolean; consumes?: boolean; text: (it: string, tag: string) => string };
export const abilities: Ability[] = [
  { name: "next", text: (it, tag) => `println!("${tag} next {:?}", ${it}.next().map(|x| x.v()));` },
  { name: "next_back", back: true, text: (it, tag) => `println!("${tag} next_back {:?}", ${it}.next_back().map(|x| x.v()));` },
  { name: "len", exact: true, text: (it, tag) => `println!("${tag} len {}", ${it}.len());` },
  { name: "size_hint", text: (it, tag) => `println!("${tag} size_hint {:?}", ${it}.size_hint());` },
  { name: "nth", text: (it, tag) => `println!("${tag} nth {:?}", ${it}.nth(1).map(|x| x.v()));` },
  { name: "nth_back", back: true, text: (it, tag) => `println!("${tag} nth_back {:?}", ${it}.nth_back(1).map(|x| x.v()));` },
  { name: "fold", consumes: true, text: (it, tag) => `println!("${tag} fold {}", ${it}.fold(0u32, |a, x| a * 10 + x.v() % 10));` },
  { name: "rfold", back: true, consumes: true, text: (it, tag) => `println!("${tag} rfold {}", ${it}.rfold(0u32, |a, x| a * 10 + x.v() % 10));` },
  {
    name: "try_fold",
    text: (it, tag) => `println!("${tag} try_fold {:?}", ${it}.try_fold(0u32, |a, x| if a > 20 { None } else { Some(a * 10 + x.v() % 10) }));`,
  },
  {
    name: "try_rfold",
    back: true,
    text: (it, tag) => `println!("${tag} try_rfold {:?}", ${it}.try_rfold(0u32, |a, x| if a > 20 { None } else { Some(a * 10 + x.v() % 10) }));`,
  },
];

/** The sources whose iterators are `Clone`, of their items' too, and
 * that aren't the crate's own (none of which is `Clone`) or a `dyn`'s; and
 * `Chars`, which hints and has no `TrustedRandomAccess`, its items made
 * numbers by a `map`. */
const cloneable = new Set(["vec", "rev vec", "range", "vec iter", "array iter", "strings", "strings iter", "options", "vecs", "d pairs"]);
export const copySources = (): Source[] => [
  ...sources.filter((source) => cloneable.has(source.name)),
  { name: "chars", shape: { item: "N", owns: false, back: true, exact: false }, expr: () => `"abcde".chars().map(|c| c as u32 - 96)`, own: false },
];

/** The chains each source is copied through: none, each adapter alone, and
 * each followed by `rev` or preceded by `copied`/`cloned` where its shape
 * has them, the paths a compiled form's knowledge takes (#1976's `rev` over
 * `copied` over `map`). `peekable`'s and `by_ref`'s lets aren't chains. */
export function chainsOf(from: Shape): Adapter[][] {
  // The catalog's adapters, the iterator matrix's and the table's
  // (test/adapter-catalog.test.ts holds it to std's), but those made by
  // a `let` of the case's own, `peekable`'s, `by_ref`'s and a holding
  // `map`'s, which `copyFn` doesn't write.
  const usable = [...adapters, ...stepAdapters].filter((a) => !["peekable", "by_ref", "flatten", "map holding", "map holding block"].includes(a.name));
  const shapeOf = (chain: Adapter[]) => chain.reduce<Shape | undefined>((shape, a) => (shape ? a.shape(shape) : undefined), from);
  const named = (name: string) => usable.find((a) => a.name === name)!;
  const chains: Adapter[][] = [[], ...usable.map((a) => [a])];
  for (const a of usable) {
    chains.push([a, named("rev")]);
    chains.push([named("copied"), a], [named("cloned"), a]);
    chains.push([named("map"), a]);
  }
  chains.push([named("copied"), named("map"), named("rev")], [named("map"), named("rev"), named("map")]);
  // And what std's zip reads by index through what the source's copies
  // are made into, its reader's chain (#1982, m1-lisa's rows).
  chains.push([named("cloned"), named("map"), named("zip")], [named("copied"), named("map"), named("zip")], [named("copied"), named("enumerate"), named("zip")]);
  const seen = new Set<string>();
  return chains.filter((chain) => {
    const key = chain.map((a) => a.name).join(".");
    if (seen.has(key) || shapeOf(chain) === undefined) return false;
    seen.add(key);
    return true;
  });
}

/** A case: a source, plain or through generic code, a chain, a way of
 * copying and an ability. */
export type CopyCase = Row & { source: Source; generic: boolean; chain: Adapter[]; way: Way; ability: Ability; shape: Shape };

/** Every case. */
export function copyCases(): CopyCase[] {
  const cases: CopyCase[] = [];
  for (const source of copySources()) {
    for (const generic of [false, true]) {
      for (const chain of chainsOf(source.shape)) {
        const shape = chain.reduce((s, a) => a.shape(s)!, source.shape);
        for (const way of ways) {
          for (const ability of abilities) {
            if ((ability.back && !shape.back) || (ability.exact && !shape.exact)) continue;
            if (waysAbilities[way] && !waysAbilities[way]!.includes(ability.name)) continue;
            if (way === "dyn" && !shape.back) continue;
            const id = `${source.name}${generic ? " generic" : ""} | ${chain.map((a) => a.name).join(".") || "-"} | ${way} | ${ability.name}`;
            cases.push({ n: cases.length, id, source, generic, chain, way, ability, shape });
          }
        }
      }
    }
  }
  return cases;
}

/** The Rust type of a source's items, as a generic function names it. */
const itemType: Record<string, string> = { D: "D", N: "u32", S: "String", "&D": "&'a D", "&N": "&'a u32", "&S": "&'a String", OD: "Option<D>", VD: "Vec<D>", DD: "(D, D)" };

/** A case's function, `case_<n>`, and of a generic one the function its
 * chain and copy are in, over `I`. */
function copyFn(c: CopyCase, n: number): string {
  const name = `case_${n}`;
  const { source, chain, way, ability } = c;
  const lets: string[] = [];
  if (source.before) lets.push(source.before);
  if (chain.some((a) => a.name === "chain") && source.shape.item === "&D") lets.push("let tail = vec![D(6), D(7)];");
  if (chain.some((a) => a.name === "chain") && source.shape.item === "&S") lets.push("let tail = texts_of(&[6, 7]);");
  let shape = source.shape;
  let expr = c.generic ? "src" : source.expr("none");
  for (const a of chain) {
    expr += a.text(shape, false);
    shape = a.shape(shape)!;
  }
  const other = c.shape.back ? "next_back" : "next";
  const copied = way === "clone" ? "a.clone()" : way === "dup" ? "dup(&a)" : "pass(a)";
  const body: string[] = [`let mut a = ${expr};`, `println!("a next {:?}", a.next().map(|x| x.v()));`, `let mut b = ${copied};`];
  if (way === "steps") {
    // A callee steps it from its back and gives back only the item.
    body.splice(2, 1, `println!("callee back {:?}", back_of(a).map(|x| x.v()));`);
  } else if (way === "by_ref" || way === "dyn") {
    // Lent: the lend stepped, then the original again from its other end.
    body.splice(2, 1, way === "by_ref" ? "let mut b = pass(a.by_ref());" : "let b: &mut dyn DoubleEndedIterator<Item = _> = &mut a;");
    body.push(ability.text("b", "b"));
    if (!ability.consumes) body.push(`println!("b next {:?}", b.next().map(|x| x.v()));`);
    body.push(`println!("a ${other} {:?}", a.${other}().map(|x| x.v()));`, `for x in a {\n        println!("a rest {}", x.v());\n    }`);
  } else if (way === "pass") {
    // The original is moved: its copy alone is stepped.
    body.push(ability.text("b", "b"));
    if (!ability.consumes) body.push(`for x in b {\n        println!("b rest {}", x.v());\n    }`);
  } else if (ability.consumes) {
    body.push(`println!("a ${other} {:?}", a.${other}().map(|x| x.v()));`, `println!("b next {:?}", b.next().map(|x| x.v()));`, ability.text("b", "b"), ability.text("a", "a"));
  } else {
    body.push(
      ability.text("b", "b"),
      `println!("a ${other} {:?}", a.${other}().map(|x| x.v()));`,
      `println!("b next {:?}", b.next().map(|x| x.v()));`,
      ability.text("a", "a"),
      `for x in a {\n        println!("a rest {}", x.v());\n    }`,
      `for x in b {\n        println!("b rest {}", x.v());\n    }`,
    );
  }
  const inner = [...(c.generic ? [] : lets), ...body].map((line) => `    ${line}`).join("\n");
  if (!c.generic) return `fn ${name}() {\n${inner}\n}`;
  // Through generic code: the source passed in as `I`, the chain over it.
  const bounds = [`Iterator<Item = ${itemType[source.shape.item]}>`, ...(source.shape.back ? ["DoubleEndedIterator"] : []), ...(source.shape.exact ? ["ExactSizeIterator"] : []), "Clone"].join(" + ");
  const tails = chain.some((a) => a.name === "chain") && (source.shape.item === "&D" || source.shape.item === "&S");
  const tail = tails ? `, tail: &'a Vec<${source.shape.item === "&D" ? "D" : "String"}>` : "";
  return [
    `fn ${name}() {\n${lets.map((line) => `    ${line}`).join("\n")}\n    ${name}_of(${source.expr("none")}${tails ? ", &tail" : ""});\n}`,
    `fn ${name}_of<'a, I: ${bounds}>(src: I${tail}) {\n${inner}\n}`,
  ].join("\n\n");
}

/** The generic helpers each way of copying is through. */
const helpers = `fn dup<I: Clone>(it: &I) -> I {\n    it.clone()\n}\n\nfn pass<I>(it: I) -> I {\n    it\n}\n\nfn back_of<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {\n    it.next_back()\n}`;

/** A program of `cases`, each run in turn. */
export function copyProgram(cases: CopyCase[]): string {
  const fns = cases.map((c, n) => copyFn(c, n));
  const main = `fn main() {\n${cases.map((c, n) => `    run(${JSON.stringify(c.id)}, case_${n});`).join("\n")}\n}`;
  return [prelude, helpers, ...fns, main].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers, of a case: each adapter
 * with each way and each ability, each source with each, generic or not. */
export const copyStrata = (c: CopyCase) => [
  ...(c.chain.length ? c.chain : [{ name: "-" }]).map((a) => `adapter ${a.name} | ${c.way} | ${c.ability.name}`),
  `source ${c.source.name}${c.generic ? " generic" : ""} | ${c.way} | ${c.ability.name}`,
];
