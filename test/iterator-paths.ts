// The paths table (#2112's Change A, step 2a): each std adapter rust-js
// lowers, through each step std's adapter writes itself, reached by each
// path generic and boxed code reaches it by: a `&mut I`, a `&mut dyn I`, a
// `Box<I>`, a `Box<dyn I>`, a generic function's parameter, an `impl
// Iterator` a function returns, a clone, and a `cycle()`; and, as #2185's
// family found them (step 2b), generic code given it through a trait,
// `fn f<I>(it: impl Is<I>)`, so `I` comes with its dictionary, of the
// adapter, a `Box` of it and a `&mut` of it, and the wrappers nested: a
// `Box` of a `Box`, a `&mut` of a `Box`, and generic code's `Box<J>` of
// a type parameter's value. Its rows are
// std's steps catalog's (src/lower/recognition/std_steps.txt, read by
// test/std-steps-catalog.ts): of each adapter A and each step s that A
// writes, the consumer that asks s, so a hand-written form that passes on
// another set of steps than std's adapter, or closes or drops in another
// order, runs otherwise than native Rust on some path. The harness, its
// sources, adapters and consumers are the iterator tables'
// (test/iterator-matrix.ts, ADR 1090 and 1092).

import { adapters, consumers, stepAdapters, stepConsumers, type Case } from "./iterator-matrix";
import { sources, type Site } from "./iterator-sources";
import { stdRuns, stdWrites } from "./std-steps-catalog";

/** How a case's iterator reaches its consumer. */
export type Path = "&mut" | "&mut dyn" | "Box" | "Box dyn" | "generic" | "impl" | "clone" | "cycle" | "trait" | "trait Box" | "trait &mut" | "Box Box" | "&mut Box" | "Box J";
export const paths: Path[] = ["&mut", "&mut dyn", "Box", "Box dyn", "generic", "impl", "clone", "cycle", "trait", "trait Box", "trait &mut", "Box Box", "&mut Box", "Box J"];

/** The trait generic code is given its iterator through, so `I` comes with
 * its dictionary (`trait` paths, #2185). */
export const isTrait = "trait Is<T> {\n    fn is(self) -> T;\n}\n\nimpl<T> Is<T> for T {\n    fn is(self) -> T {\n        self\n    }\n}";

/** The catalog's adapters, by std's name, that are paths here, not rows:
 * a `Cycle` is `cycle()`'s path (endless, it's stepped by a `take(7)`), and
 * the four wrappers are each a path's. */
const asPaths = new Set(["Cycle", "&mut I", "&mut dyn I", "Box<I>", "Box<dyn I>"]);

/** std's name of an adapter as the generator names it: `StepBy`, `step_by`. */
const snake = (name: string) => name.replace(/[A-Z]/g, (c, at: number) => `${at === 0 ? "" : "_"}${c.toLowerCase()}`);

/** The sources a row runs over, the first two of which its adapter takes:
 * the crate's iterator that writes every step it can, and one that writes
 * none; else, of a `copied()` or a `cloned()`, a slice's items. */
const rowSources = ["own Every", "own Plain"];
const borrowedSources = ["strings iter", "array iter"];

/** The steps of the catalog that no stable consumer asks of what it holds:
 * `advance_by`, `advance_back_by` and `ExactSizeIterator::is_empty` are
 * unstable, and no consumer's trace over a `Map` asks them. */
export function unasked(): string[] {
  return [...new Set([...stdWrites].filter(([name]) => !asPaths.has(name)).flatMap(([, writes]) => writes))].filter((s) => consumersOf(s).length === 0);
}

/** The consumers that ask step `s` of what they hold: the one named `s`;
 * else each whose trace over a `Map` of a source that writes every step,
 * the catalog's `Map#0`, asks `src.<s>` (`rfind`, `try_rfold`). */
export function consumersOf(s: string) {
  const all = [...consumers, ...stepConsumers];
  const named = all.filter((c) => c.name === s);
  if (named.length > 0) return named;
  return all.filter((c) => stdRuns.get(`Map#0 ${c.name}`)?.split(" ").includes(`src.${s}`));
}

/** Of each adapter, its name in the generator, and the steps the catalog
 * says it writes: its rows. */
export function catalogAdapters(): { std: string; name: string; writes: string[] }[] {
  const generator = new Set([...adapters, ...stepAdapters].map((a) => a.name));
  const out = [...stdWrites]
    .filter(([name]) => !asPaths.has(name))
    .map(([std, writes]) => ({ std, name: snake(std), writes }));
  const missing = out.filter((a) => !generator.has(a.name)).map((a) => `${a.std} (${a.name})`);
  if (missing.length > 0) throw new Error(`the steps catalog's ${missing.join(", ")}: no adapter of the iterator tables is named so`);
  return out;
}

/** A case's iterator as its path gives it to the consumer: whether it's
 * stepped from its back and knows its length, as the path's type says. */
export function pathShape<S extends { back: boolean; exact: boolean }>(path: Path, shape: S): S {
  if (path === "&mut dyn" || path === "Box dyn") return { ...shape, exact: false };
  if (path === "cycle") return { ...shape, back: false, exact: false };
  return shape;
}

/** Every case of the paths table, in one order: of each adapter of the
 * catalog, each step it writes, each path, each source that it takes,
 * and each site of `none` and `consumer`; each (adapter, consumer, path,
 * source, site) once. */
export function pathsCases(): Case[] {
  const out: Case[] = [];
  const seen = new Set<string>();
  const all = [...adapters, ...stepAdapters];
  for (const { name, writes } of catalogAdapters()) {
    const adapter = all.find((a) => a.name === name)!;
    let takers = rowSources.map((s) => sources.find((x) => x.name === s)!).filter((s) => adapter.shape(s.shape));
    if (takers.length === 0) takers = [borrowedSources.map((s) => sources.find((x) => x.name === s)!).find((s) => adapter.shape(s.shape))!];
    for (const s of writes) {
      for (const consumer of consumersOf(s)) {
        for (const path of paths) {
          for (const source of takers) {
            // A function that returns it can't borrow the case's local.
            if (path === "impl" && source.before) continue;
            const shape = pathShape(path, adapter.shape(source.shape)!);
            if (!consumer.takes(shape)) continue;
            for (const site of ["none", "consumer"] as Site[]) {
              if (site === "consumer" && !consumer.closure) continue;
              const id = `${source.name} | ${name} | ${consumer.name} | ${site} | ${path}`;
              if (seen.has(id)) continue;
              seen.add(id);
              out.push({ n: out.length, id, source, chain: [adapter], consumer, site, variant: { kind: "path", path } });
            }
          }
        }
      }
    }
  }
  return out;
}

/** The consumers that bind what they're given, `let mut it = ..`, and step
 * it by `&mut self`: of a reference, the referent's own step, but where
 * it's lent (`pathFns`). */
const bindsAndSteps = new Set(["next", "next_back", "nth", "nth_back"]);

/** The type of the items `chain` makes of `source`'s, as Rust writes it. */
function itemType(c: Case): string {
  let ty = { D: "D", "&N": "&u32", "&S": "&String", "&D": "&D", N: "u32", S: "String" }[c.source.shape.item as string];
  if (ty === undefined) throw new Error(`the paths table has no type of ${c.source.name}'s items`);
  for (const a of c.chain) {
    if (a.name === "map") ty = ty === "D" || ty === "String" ? ty : "u32";
    else if (a.name === "enumerate") ty = `(usize, ${ty})`;
    else if (a.name === "zip") ty = `(${ty}, u32)`;
    else if (a.name === "copied" || a.name === "cloned") ty = ty.replace(/^&/, "");
  }
  return ty;
}

/** The traits a path's type says of the iterator it holds: `Iterator` or
 * `DoubleEndedIterator` of its items, and `ExactSizeIterator` where the
 * type isn't `dyn`, which has one trait of its own. */
function bounds(c: Case, shape: { back: boolean; exact: boolean }, dyn: boolean): string {
  const of = `${shape.back ? "DoubleEndedIterator" : "Iterator"}<Item = ${itemType(c)}>`;
  return dyn || !shape.exact ? of : `${of} + ExactSizeIterator`;
}

/** A path case's functions: `case_<n>`, whose `body` makes the chain
 * `expr` of the shape `shape`, and gives it to its consumer by its path;
 * and the function its path calls, of `generic` and `impl`. */
export function pathFns(c: Case & { variant: { kind: "path"; path: Path } }, name: string, body: string[], expr: string, shape: { back: boolean; exact: boolean }, consumed: (it: string) => string): string {
  const { path } = c.variant;
  const seen = c.consumer.closure ? ["let mut seen = 0;"] : [];
  const mut = c.consumer.lends ? "mut " : "";
  // A step by `&mut self`, `try_fold` or `nth`, called on a `&mut` itself
  // is the referent's own, as a method call reborrows: a consumer that
  // steps so is given the reference lent, so its step is `&mut I`'s.
  const lent = c.consumer.lends || bindsAndSteps.has(c.consumer.name);
  const reference = (r: string) => (lent ? `(&mut ${r})` : r);
  const fn = (header: string, lines: string[]) => `fn ${header} {\n${lines.map((line) => `    ${line}`).join("\n")}\n}`;
  switch (path) {
    case "&mut":
      return fn(`${name}()`, [...body, `let mut w = ${expr};`, `let ${lent ? "mut " : ""}r = &mut w;`, ...seen, consumed(reference("r"))]);
    case "&mut dyn":
      return fn(`${name}()`, [...body, `let mut w = ${expr};`, `let ${lent ? "mut " : ""}d: &mut dyn ${bounds(c, shape, true)} = &mut w;`, ...seen, consumed(reference("d"))]);
    case "Box":
      return fn(`${name}()`, [...body, ...seen, consumed(`Box::new(${expr})`)]);
    case "Box dyn":
      return fn(`${name}()`, [...body, `let ${mut}b: Box<dyn ${bounds(c, shape, true)} + '_> = Box::new(${expr});`, ...seen, consumed("b")]);
    case "generic":
      return [fn(`${name}()`, [...body, `${name}_consume(${expr});`]), fn(`${name}_consume(${mut}it: impl ${bounds(c, shape, false)})`, [...seen, consumed("it")])].join("\n\n");
    case "impl":
      return [fn(`${name}()`, [...seen, consumed(`${name}_make()`)]), fn(`${name}_make() -> impl ${bounds(c, shape, false)}`, [...body, expr])].join("\n\n");
    case "clone":
      return fn(`${name}()`, [...body, `let w = ${expr};`, ...seen, consumed("w.clone()"), "drop(w);"]);
    case "cycle":
      return fn(`${name}()`, [...body, ...seen, consumed(`${expr}.cycle().take(7)`)]);
    case "trait":
    case "trait Box":
    case "trait &mut": {
      const given = path === "trait" ? expr : path === "trait Box" ? `Box::new(${expr})` : "&mut w";
      const before = path === "trait &mut" ? [`let mut w = ${expr};`] : [];
      return [fn(`${name}()`, [...body, ...before, `${name}_consume(${given});`]), fn(`${name}_consume<I: ${bounds(c, shape, false)}>(it: impl Is<I>)`, [`let ${mut}it: I = Is::is(it);`, ...seen, consumed("it")])].join("\n\n");
    }
    case "Box Box":
      return fn(`${name}()`, [...body, ...seen, consumed(`Box::new(Box::new(${expr}))`)]);
    case "&mut Box":
      return fn(`${name}()`, [...body, `let mut w = Box::new(${expr});`, `let ${lent ? "mut " : ""}r = &mut w;`, ...seen, consumed(reference("r"))]);
    case "Box J":
      return [fn(`${name}()`, [...body, `${name}_consume(${expr});`]), fn(`${name}_consume<J: ${bounds(c, shape, false)}>(j: J)`, [...seen, consumed("Box::new(j)")])].join("\n\n");
  }
}

/** The crate's iterator `name`'s `Clone`, which prints as it runs, so a
 * clone and a `cycle()` of it show where they clone. */
export const ownClone = (name: string) =>
  `impl Clone for ${name} {\n    fn clone(&self) -> Self {\n        println!("clone src");\n        ${name} { at: self.at, end: self.end, boom: self.boom }\n    }\n}`;

/** What a slice of the paths table covers, of a case: each path with each
 * adapter, each consumer and each site, and each adapter with each
 * consumer. */
export function pathStrata(c: Case): string[] {
  const path = c.variant.kind === "path" ? c.variant.path : "-";
  const adapter = c.chain.map((a) => a.name).join(".");
  return [`path ${path} | adapter ${adapter} | source ${c.source.name}`, `path ${path} | consumer ${c.consumer.name} | site ${c.site}`, `adapter ${adapter} | consumer ${c.consumer.name}`];
}

// `bun test/iterator-paths.ts`: the table's size, by path and by adapter,
// and the catalog's steps no consumer asks.
if (import.meta.main) {
  const cases = pathsCases();
  const by = (key: (c: Case) => string) => {
    const counts = new Map<string, number>();
    for (const c of cases) counts.set(key(c), (counts.get(key(c)) ?? 0) + 1);
    return [...counts].map(([k, n]) => `${k} ${n}`).join(", ");
  };
  const pairs = catalogAdapters().reduce((n, a) => n + a.writes.length, 0);
  console.log(`${cases.length} cases, ${Math.ceil(cases.length / 60)} batches of 60, of ${pairs} (adapter, step) pairs`);
  console.log(`unasked steps: ${unasked().join(", ")}`);
  console.log(`by path: ${by((c) => (c.variant.kind === "path" ? c.variant.path : ""))}`);
  console.log(`by adapter: ${by((c) => c.chain[0].name)}`);
}
