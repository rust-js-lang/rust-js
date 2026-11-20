// Each iterator the runtime makes that passes its source's steps on, by
// `$forward`, reaches each step its source has of its own, not `$Steps`'
// default of it over `next`, which steps otherwise where the source's
// differs: a `skip(1)`'s `tryFold` asks `nth(0)` where its `next` asks
// `nth(1)` (#2051), and a `filter`'s keeps what its accumulator holds as
// a stage panics (#2056). Every step is `$stepNames`, lent.js's, the one
// list; what a `&mut I` and a `Box<I>` pass on is std's steps' catalog's
// (src/lower/recognition/std_steps.rs).
// A step a wrapper doesn't pass on says why here, so a sixth one can't be
// missed: `$closing` lacked `tryFold`, and `$flattenEnds` `tryFold` and
// what `last` takes, `inners`, which `$flatMap` had (#2051).

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { helpersModule } from "./support";
import { ownerTable } from "../scripts/owners";
import { stdAnswers, stdRuns } from "./std-steps-catalog";

// The runtime the JS the tests write imports, `@rust-js/std`'s: a mutant's
// own where the mutation runner gives its tests one, so a runtime mutation
// this net alone catches is caught, and one that compiles nothing gives an
// empty module, which fails here (#2206).
const std: any = await import(helpersModule);

const steps = [...std.$stepNames].filter((step) => step !== "next");

/** A source of every step, each its own, as the crate's iterator's or a
 * compiled std struct's is, each call of one in `called`. */
function source(called: string[]) {
  // Its `advance_by` and `advance_back_by` of its kind, not its own
  // properties, which say it's a slice's, a `Vec`'s `IntoIter`, whose `nth`
  // and `count` its slice's (`$ownMethod`, `$closing`, #286, #648).
  const sliced = new Set(["advanceBy", "advanceBackBy"]);
  const kind: Record<string, unknown> = {};
  const it: Record<string, unknown> = Object.create(kind);
  it.return = () => ({ done: true, value: undefined });
  const done = { done: true, value: undefined };
  const answers: Record<string, (...args: unknown[]) => unknown> = {
    next: () => done,
    back: () => done,
    sizeHint: () => [0, 0],
    len: () => 0,
    count: () => 0,
    last: () => done,
    nth: () => done,
    nthBack: () => done,
    advanceBy: () => 0,
    advanceBackBy: () => 0,
    fold: (init: unknown) => init,
    rfold: (init: unknown) => init,
    tryFold: (init: unknown) => init,
    tryRfold: (init: unknown) => init,
  };
  for (const step of std.$stepNames) {
    const answer = answers[step];
    if (answer === undefined) throw new Error(`no answer of \`${step}\`: a step $stepNames has that this test doesn't`);
    (sliced.has(step) ? kind : it)[step] = (...args: unknown[]) => {
      called.push(step);
      return answer(...args);
    };
  }
  for (const [own, step] of std.$stepOf) if (!sliced.has(step)) it[own] = true;
  return it;
}

/** Each step asked of `wrapper`, as its consumer would ask it. */
function ask(wrapper: any, step: string) {
  const f = (a: unknown) => a;
  const args: Record<string, unknown[]> = {
    fold: [0, f],
    rfold: [0, f],
    tryFold: [0, f, () => false],
    tryRfold: [0, f, () => false],
    nth: [0],
    nthBack: [0],
    advanceBy: [1],
    advanceBackBy: [1],
  };
  // One it has none of, `$Steps` has none of (`advanceBackBy`, back_steps.js'
  // helpers'), reaches nothing of its source's.
  if (typeof wrapper[step] === "function") wrapper[step](...(args[step] ?? []));
}

/** Each wrapper that passes on what its source has, as it's made, and the
 * steps it doesn't pass on, each with why. */
const ofItsKind = "passed on where it's its source's own, a slice's or a compiled std struct's (#286); one of its kind is `$Steps`' default by `next`, as the wrapper's own is, so it steps alike";
const wrappers: [string, (it: any) => any, Record<string, string>][] = [
  ["$closing", (it) => std.$closing(it, () => {}), { advanceBy: ofItsKind, advanceBackBy: ofItsKind }],
  ["$holding", (it) => std.$holding(it, () => {}), { advanceBy: ofItsKind, advanceBackBy: ofItsKind }],
];

for (const [name, make, kept] of wrappers) {
  test(`${name} passes on each step its source has`, () => {
    const missed: string[] = [];
    for (const step of steps) {
      const called: string[] = [];
      ask(make(source(called)), step);
      if (!called.includes(step) && kept[step] === undefined) missed.push(step);
    }
    expect(missed).toEqual([]);
  });
}

// The flattens a flat_map or a flatten is, of a source that knows its back
// or not, each std's `FlattenCompat`: the steps one has its own, the other
// does, each made of what the generic code that holds it may ask.
test("each flatten has the steps of its own the other has", () => {
  const own = (made: object) =>
    ["fold", "tryFold", "count", "inners", "advanceBy", "last"].filter((step) => Object.hasOwn(made, step)).sort();
  const items = () => std.$iter([[1], [2]]);
  const flatMap = std.$flatMap(items(), (x: unknown) => x);
  const ends = std.$flattenEnds(items(), (x: unknown) => x);
  expect(own(ends)).toEqual(own(flatMap));
  expect(own(flatMap)).toEqual(["advanceBy", "count", "fold", "inners", "last", "tryFold"]);
});

// Who drops what a step stepped, where the step panics, follows how the
// step takes its iterator, as std declares it: one by value, `fold(self,
// ..)`, drops it as it unwinds, so the wrapper that took it closes its
// source; one by `&mut self`, `try_fold`, leaves it its owner's, so the
// wrapper closes nothing (S2). Each step's receiver is read from this
// toolchain's core, not written here (#2051: `$closing`'s `tryFold` had
// closed as its `fold` does, and dropped the source twice).
const sysroot = Bun.spawnSync(["rustc", "--print", "sysroot"]).stdout.toString().trim();
const traits = ["iterator.rs", "double_ended.rs"].map((file) =>
  readFileSync(join(sysroot, "lib/rustlib/src/rust/library/core/src/iter/traits", file), "utf8"),
);
/** std's name of each step `$Steps` has. */
const rust: Record<string, string> = {
  fold: "fold",
  tryFold: "try_fold",
  rfold: "rfold",
  tryRfold: "try_rfold",
  nth: "nth",
  nthBack: "nth_back",
  count: "count",
  last: "last",
};
/** Whether std's `step` takes its iterator by value, as its trait declares. */
function byValue(step: string): boolean {
  const name = rust[step]!;
  const declared = traits.map((text) => text.match(new RegExp(`\\n    fn ${name}\\b[^(]*\\(\\s*(&mut self|&self|mut self|self)\\b`))).find(Boolean);
  if (!declared) throw new Error(`core declares no \`${name}\`: read its receiver again`);
  return !declared[1]!.startsWith("&");
}

for (const [name, make] of [
  ["$closing", (it: any, drop: () => void) => std.$closing(it, drop)],
  ["$holding", (it: any, drop: () => void) => std.$holding(it, drop)],
] as const) {
  test(`${name} closes its source where a step panics as the step's receiver says`, () => {
    expect(byValue("fold")).toBe(true);
    expect(byValue("tryFold")).toBe(false);
    const wrong: string[] = [];
    for (const step of Object.keys(rust)) {
      // Of an iterator that writes none of its steps, so each is std's
      // default over its `next` and `back`, each of which panics.
      const it: any = new std.$Steps(() => {
        throw new Error("boom");
      });
      // Stepped from its back too, as `rfold` and `nth_back` ask.
      it.back = () => {
        throw new Error("boom");
      };
      let closed = 0;
      const wrapper: any = make(it, () => closed++);
      // One it hasn't, of a source that hasn't it either, `rfold` of a
      // `$Steps` (back_steps.js' helpers'), isn't asked of it.
      if (typeof wrapper[step] !== "function") continue;
      try {
        ask(wrapper, step);
      } catch {}
      if (closed > 0 !== byValue(step)) wrong.push(`${step}: ${closed > 0 ? "closed" : "not closed"}`);
    }
    expect(wrong).toEqual([]);
  });
}

// Each of the runtime's adapters asks of what it holds what std's adapter
// asks, step by step: its trace, over sources that write every step and
// record each they're asked with its argument, is std's, the catalog's
// (src/lower/recognition/std_steps.txt, test/std-steps.test.ts, #2112), in
// each of its runs of the sources' lengths. Each step is asked as the
// lowering asks it, by the runtime's helper where it has one. A form made
// for one end steps none from the other: one with no `back` isn't asked a
// back step; but each is asked its length. A difference that no program
// can see is listed with why (ADR 0262's case N); any other is a wrong
// answer.


/** test/std-steps.rs's `SIZES`: the sources' lengths of each run. */
const sizes: [number, number][] = [
  [5, 3],
  [0, 3],
  [5, 0],
  [1, 1],
];

/** std's name of each of `$forward`'s steps. */
const stdName: Record<string, string> = {
  next: "next",
  back: "next_back",
  sizeHint: "size_hint",
  len: "len",
  nth: "nth",
  advanceBy: "advance_by",
  fold: "fold",
  tryFold: "try_fold",
  last: "last",
  count: "count",
  nthBack: "nth_back",
  advanceBackBy: "advance_back_by",
  rfold: "rfold",
  tryRfold: "try_rfold",
};

/** The kinds of source each run is over, as test/std-steps.rs's: one that
 * writes every step, `""`, one that writes none of the folds, and one that
 * writes only `next`, `next_back` and `size_hint`, the traits' defaults
 * over them the rest, as `$iterator` makes the crate's. */
const kinds = ["", "nofold", "bare"] as const;
type SourceKind = (typeof kinds)[number];

/** Each kind's steps besides `next`, `back` and `sizeHint`, and of those,
 * the ones its own (`$ownStep`). */
const kindSteps: Record<SourceKind, { writes: string[]; own: string[] }> = {
  "": {
    writes: ["len", "nth", "advanceBy", "fold", "tryFold", "count", "last", "nthBack", "advanceBackBy", "rfold", "tryRfold"],
    own: ["fold", "rfold", "nth", "nthBack", "last", "count", "tryRfold"],
  },
  nofold: { writes: ["len", "nth", "advanceBy", "count", "last", "nthBack", "advanceBackBy"], own: ["nth", "nthBack", "last", "count"] },
  bare: { writes: [], own: [] },
};

/** A source of `items` of `kind` that records each step it writes as it's
 * asked, `src.nth(1)`, in `log`, as test/std-steps.rs's `Loud`. One that
 * writes no `len` has std's default, its `size_hint`'s, as `$iterator`
 * gives the crate's (`$exactLen`). */
function loud(name: string, items: unknown[], log: string[], kind: SourceKind = ""): any {
  const done = { done: true, value: undefined };
  const say = (step: string, arg?: number) => log.push(`${name}.${stdName[step]}${arg === undefined ? "" : `(${arg})`}`);
  const it: any = new std.$Steps(() => (say("next"), items.length > 0 ? { done: false, value: items.shift() } : done));
  it.back = () => (say("back"), items.length > 0 ? { done: false, value: items.pop() } : done);
  it.sizeHint = () => (say("sizeHint"), [items.length, items.length]);
  const steps: Record<string, (...args: any[]) => unknown> = {
    len: () => (say("len"), items.length),
    nth: (k: number) => (say("nth", k), k < items.length ? (items.splice(0, k), { done: false, value: items.shift() }) : ((items.length = 0), done)),
    advanceBy: (m: number) => (say("advanceBy", m), m - items.splice(0, m).length),
    fold: (init: unknown, f: (a: unknown, x: unknown) => unknown) => (say("fold"), items.splice(0).reduce(f, init)),
    tryFold: (init: unknown, f: (a: unknown, x: unknown) => unknown, broke: (a: unknown) => boolean) => {
      say("tryFold");
      let a = init;
      while (items.length > 0 && !broke((a = f(a, items.shift())))) {}
      return a;
    },
    count: () => (say("count"), items.splice(0).length),
    last: () => (say("last"), items.length > 0 ? { done: false, value: items.splice(0).pop() } : done),
    nthBack: (k: number) => (say("nthBack", k), items.splice(Math.max(items.length - k, 0)), items.length > 0 ? { done: false, value: items.pop() } : done),
    advanceBackBy: (m: number) => (say("advanceBackBy", m), m - items.splice(Math.max(items.length - m, 0)).length),
    rfold: (init: unknown, f: (a: unknown, x: unknown) => unknown) => (say("rfold"), items.splice(0).reduceRight(f, init)),
    tryRfold: (init: unknown, f: (a: unknown, x: unknown) => unknown, broke: (a: unknown) => boolean) => {
      say("tryRfold");
      let a = init;
      while (items.length > 0 && !broke((a = f(a, items.pop())))) {}
      return a;
    },
  };
  for (const step of kindSteps[kind].writes) it[step] = steps[step];
  it.len ??= () => std.$exactLen(it.sizeHint());
  for (const step of kindSteps[kind].own) it[std.$ownStep(step)] = true;
  return it;
}

const count = (a: number) => a + 1;
/** test/std-steps.rs's `second!()`: a predicate that holds at the second
 * item it's asked of. */
const second = () => {
  let asked = 0;
  return () => ++asked === 2;
};
/** Each step asked as the lowering asks it, test/std-steps.rs's arguments:
 * `nth(1)`, `advance_by(2)`, a `try_fold` that breaks at its second item,
 * and of std's consumers, a predicate that holds at its second (#2142). */
const asked: Record<string, (it: any) => unknown> = {
  next: (it) => it.next(),
  size_hint: (it) => std.$sizeHint(it),
  size_hint_after_next: (it) => (it.next(), std.$sizeHint(it)),
  nth: (it) => std.$nth(it, 1),
  advance_by: (it) => std.$advanceBy(it, 2),
  fold: (it) => std.$foldOf(it, 0, count),
  try_fold: (it) => it.tryFold(0, count, (a: number) => a === 2),
  last: (it) => std.$last(it),
  count: (it) => std.$count(it),
  next_back: (it) => it.back(),
  nth_back: (it) => std.$nthBack(it, 1),
  advance_back_by: (it) => std.$advanceBackBy(it, 2),
  rfold: (it) => std.$foldBack(it, 0, count),
  try_rfold: (it) => std.$tryRfoldOf(it, 0, count, (a: number) => a === 2),
  len: (it) => it.len(),
  position: (it) => std.$position(it, second()),
  find: (it) => it.find(second()),
  find_map: (it) => {
    const found = second();
    return std
      .$lent(it)
      .map((x: unknown) => (found() ? x : undefined))
      .find((item: unknown) => item != null);
  },
  any: (it) => it.some(second()),
  all: (it) => {
    const found = second();
    return it.every(() => !found());
  },
  max_by: (it) => std.$maxBy(it, () => 0),
  min_by: (it) => std.$minBy(it, () => 0),
  rfind: (it) => std.$findBack(it, second()),
  rposition: (it) => std.$rposition(it, second()),
};

/** Of a run, its sources: `src`, `other`, and `nested`, a flatten's, of up
 * to two `inner`s of the second length; `lenless`, each with no `len`, a
 * JS form that has none, std's compiled `Fuse` given no `$knowing` (#2194). */
type Sources = { src: () => any; other: () => any; nested: () => any };
const sourcesOf = ([a, b]: [number, number], log: string[], kind: SourceKind, lenless = false): Sources => {
  const range = (from: number, n: number) => Array.from({ length: n }, (_, i) => from + i);
  const made = (it: any) => {
    if (lenless) delete it.len;
    return it;
  };
  return {
    src: () => made(loud("src", range(1, a), log, kind)),
    other: () => made(loud("other", range(6, b), log, kind)),
    nested: () => made(loud("src", range(0, Math.min(a, 2)).map(() => loud("inner", range(1, b), log, kind)), log, kind)),
  };
};

const same = (x: unknown) => x;
const odd = (x: number) => x % 2 === 1;
const present = (item: unknown) => item != null;
/** Each form lowering makes a std adapter as in place, by std's name and
 * the form's, as src/lower/iterators/in_place_forms.txt names them, made
 * as it's emitted: the table both ways (Change A step 3, #2112). */
const forms: [std: string, form: string, make: (s: Sources) => any][] = [
  ...["Map", "Cloned", "Inspect"].flatMap((name): [string, string, (s: Sources) => any][] => [
    [name, "$mapEnds", (s) => std.$mapEnds(s.src(), same)],
    [name, "$Steps.map", (s) => s.src().map(same)],
  ]),
  ["Copied", "$copied", (s) => std.$copied(s.src(), same)],
  ["Filter", "$filterEnds", (s) => std.$filterEnds(s.src(), odd)],
  ["Filter", "$Steps.filter", (s) => s.src().filter(odd)],
  ["Filter", "$filterDropping", (s) => std.$filterDropping(s.src(), odd, () => {})],
  // The `Some`s, a `filter_map`'s made in place, of a map and of a map
  // that knows its back: a filter whose own `count` is its fold, never
  // asking its source's hint (#2199); and as JS writes them where the hint
  // runs nothing, which a flatten of `Option`s or a `flat_map` to them is
  // made as too where their steps run nothing of the crate's.
  ["FilterMap", "$somes", (s) => std.$somes(s.src().map(same))],
  ["FilterMap", "$somes of ends", (s) => std.$somes(std.$mapEnds(s.src(), same))],
  ...["FilterMap", "Flatten<Option>", "FlatMap<Option>"].map((name): [string, string, (s: Sources) => any] => [
    name,
    "$Steps.filter of somes",
    (s) => s.src().map(same).filter(present),
  ]),
  // A flatten of `Option`s' or a `flat_map` to them made in place
  // elsewhere: std's `FlattenCompat` of one-shot inner iterators, which
  // steps its source by `next` (#2205).
  ...["Flatten<Option>", "FlatMap<Option>"].flatMap((name): [string, string, (s: Sources) => any][] => [
    [name, "$someItems", (s) => std.$someItems(s.src().map(same))],
    [name, "$someItems of ends", (s) => std.$someItems(std.$mapEnds(s.src(), same))],
  ]),
  ["Enumerate", "$enumerate", (s) => std.$enumerate(s.src())],
  ["Skip", "$skipping", (s) => std.$skipping(s.src(), 1)],
  ["Take", "$taking", (s) => std.$taking(s.src(), 3)],
  ["StepBy", "$stepBy", (s) => std.$stepBy(s.src(), 2)],
  ["StepBy", "$stepByEnds", (s) => std.$stepByEnds(s.src(), 2)],
  ["StepBy", "$Steps.filter by index", (s) => s.src().filter((_: unknown, i: number) => i % 2 === 0)],
  ["Chain", "$lazyChain", (s) => std.$lazyChain(s.src(), s.other())],
  ["Zip", "$lazyZip", (s) => std.$lazyZip(s.src(), s.other())],
  ["Zip", "$zipEnds", (s) => std.$zipEnds(s.src(), s.other())],
  ["Rev", "$rev", (s) => std.$rev(s.src())],
  ...["Flatten", "FlatMap"].flatMap((name): [string, string, (s: Sources) => any][] => [
    [name, "$flatMap", (s) => std.$flatMap(s.nested(), same)],
    [name, "$flattenEnds", (s) => std.$flattenEnds(s.nested(), same)],
    [name, "$Steps.flatMap", (s) => s.nested().flatMap(same)],
  ]),
  ["TakeWhile", "$lazyTakeWhile", (s) => std.$lazyTakeWhile(s.src(), (x: number) => x < 4)],
  ["SkipWhile", "$lazySkipWhile", (s) => std.$lazySkipWhile(s.src(), (x: number) => x < 2)],
  ["MapWhile", "$lazyMapWhile", (s) => std.$lazyMapWhile(s.src(), same, (x: unknown) => x !== undefined)],
  ["Scan", "$lazyScan", (s) => std.$lazyScan(s.src(), 0, (_: unknown, x: unknown) => x)],
  // `$cycleOf`'s, which makes it: its clones, each of what it started
  // with, never stepped, as fresh as std's of an unstepped `Loud` (#2192).
  ["Cycle", "$cycleFrom", (s) => std.$cycleOf(s.src(), () => s.src(), std.$next)],
];

/** Each form lowering makes a consumer as in place, by std's method and
 * the form's name, as the table names them: asked of a map of the source,
 * `src.map(f).nth(1)`, and held to std's `Map`'s row of the method. */
const consumerForms: [std: string, form: string, ask: (it: any) => unknown][] = [
  ["nth", "$nth", (it) => std.$nth(it, 1)],
  ["nth", "$Steps.drop.take", (it) => Array.from(it.drop(1).take(1))[0]],
  ["find_map", "$Steps.map.find", (it) => asked.find_map!(it)],
];

/** std's wrappers the runtime passes steps on through, by std's name. */
const lentForms: [std: string, form: string, make: (s: Sources) => any][] = [
  ["&mut I", "$lent", (s) => std.$lent(s.src())],
  ["&mut dyn I", "$lent unsized", (s) => std.$lent(s.src(), "lentUnsized")],
  ["Box<I>", "$lent boxed", (s) => std.$lent(s.src(), "boxed")],
  ["Box<dyn I>", "$lent boxedUnsized", (s) => std.$lent(s.src(), "boxedUnsized")],
  ["Box<dyn I>", "$stepsAround", (s) => std.$stepsAround(s.src())],
];

/** The in-place forms' table, src/lower/iterators/in_place_forms.txt: of
 * each std type and form, where it's chosen; and each `Where`'s APIs. */
const table = readFileSync(join(import.meta.dir, "../src/lower/iterators/in_place_forms.txt"), "utf8").split("\n");
const chosen = new Map(
  table.filter((line) => !line.startsWith("#") && line.includes(" | ")).map((line) => {
    const [stdName, form, at] = line.split(" | ");
    return [`${stdName} | ${form}`, at!] as const;
  }),
);
const whereAsks = new Map(
  table.flatMap((line) => {
    const m = /^where (\w+):(.*)$/.exec(line);
    return m ? [[m[1]!, m[2]!.split(" ").filter(Boolean)] as const] : [];
  }),
);
/** What the net runs a form over, of where it's chosen: its kinds of
 * source, and the steps of what it holds that run nothing of the crate's
 * there, left out of both traces. Of an array's items, none of a loud
 * source's: what it's asked runs nothing. */
const whereRuns: Record<string, { kinds: readonly SourceKind[]; unseen?: RegExp }> = {
  Always: { kinds },
  HintRunsNothing: { kinds, unseen: /^\w+\.size_hint$/ },
  // Of a source that writes no step `next` and `try_fold` would ask apart,
  // `bare`'s, its hint unseen: no `nth` of the crate's between (#2205).
  StepsRunNothing: { kinds: ["bare"], unseen: /^\w+\.size_hint$/ },
  OfArrayItems: { kinds: [] },
  NoOwnNth: { kinds: ["bare"] },
  PlainInners: { kinds: ["bare"] },
};
const runsOf = (stdName: string, form: string) => {
  const at = chosen.get(`${stdName} | ${form}`);
  if (at === undefined || whereRuns[at] === undefined) throw new Error(`\`${stdName} | ${form}\`: no row of in_place_forms.txt, or a where the net doesn't run`);
  return whereRuns[at]!;
};
const seen = (trace: string, unseen?: RegExp) => (unseen === undefined ? trace : trace.split(" ").filter((call) => !unseen.test(call)).join(" "));

/** What a step answered, as std's catalog prints it, Rust's `Debug`:
 * `size_hint`'s pair, `(4, Some(4))`, and `len`'s number; `usize::MAX`
 * by name, a JS `usize`'s 2^32 - 1 and std's 2^64 - 1 alike. */
const usize = (n: number) => (n === 2 ** 32 - 1 ? "usize::MAX" : String(n));
const answered = (step: string, answer: any) =>
  step.startsWith("size_hint") ? `(${usize(answer[0])}, ${answer[1] == null ? "None" : `Some(${usize(answer[1])})`})` : usize(answer);
const native = (answer: string | undefined) => answer?.replaceAll("18446744073709551615", "usize::MAX");

/** Each run: its kind of source, its number and its sources' lengths. */
const runs = kinds.flatMap((kind) => sizes.map((size, run) => [kind, run, size] as const));

/** Each form's step that asks other than std's does, a wrong answer not
 * yet fixed, under its issue: none is let differ for good (#2112). */
const differs: Record<string, string> = {};

for (const [stdForm, form, make] of [...forms, ...lentForms]) {
  test(`${form} asks what it holds as std's ${stdForm} does`, () => {
    const wrong: string[] = [];
    const listed = new Set<string>();
    const { kinds: allowed, unseen } = lentForms.some(([, f]) => f === form) ? whereRuns.Always! : runsOf(stdForm, form);
    for (const step of Object.keys(asked)) {
      for (const [kind, run, size] of runs.filter(([kind]) => allowed.includes(kind))) {
        const ran = stdRuns.get(`${stdForm}#${kind}${run} ${step}`);
        if (ran === undefined) continue;
        const want = seen(ran, unseen);
        const log: string[] = [];
        const it = make(sourcesOf(size, log, kind));
        // A form for one end: none of the other's steps. Its length is
        // asked wherever std's adapter has one, as generic code asks it of
        // whatever form an `ExactSizeIterator` bound is given: a form
        // without one was let off, which hid `$taking`'s and `$Steps.map`'s
        // (#2193, #2194).
        if (/back|rfold|rfind|rposition/.test(step) && typeof it.back !== "function") continue;
        log.length = 0;
        const answer = asked[step]!(it);
        // What it answered too, of `size_hint` and `len`: std's one-shot
        // `Flatten`'s hint is `(0, upper)` where the generic one's is
        // `(lo, None)`, which no trace tells apart (#2205).
        const [got, wanted] = [seen(log.join(" "), unseen), stdAnswers.get(`${stdForm}#${kind}${run} ${step}`)];
        const gave = wanted === undefined ? undefined : answered(step, answer);
        if (got === want && gave === native(wanted)) continue;
        const key = `${stdForm} | ${form} ${step}${kind === "" ? "" : ` ${kind}`}`;
        const told = gave === native(wanted) ? "" : ` = ${native(wanted)}`;
        if (differs[key] === undefined) wrong.push(`${step}#${kind}${run}: std ${want}${told}; ${got}${told && ` = ${gave}`}`);
        else listed.add(key);
      }
      // And where std's length asks no `len` of what it holds that writes
      // one, a `Take`'s of its `size_hint`, the same of sources with none: a
      // form whose length is its source's `len` lacked it there, `$taking`
      // of std's compiled `Fuse` (#2194). Not of a source that writes none,
      // whose `len` is std's default, its `size_hint`, which a trace can't
      // tell from it.
      for (const [kind, run, size] of step === "len" ? runs.filter(([kind]) => allowed.includes(kind)) : []) {
        const ran = stdRuns.get(`${stdForm}#${kind}${run} len`);
        if (kind === "bare" || ran === undefined || ran.includes(".len")) continue;
        const want = seen(ran, unseen);
        const log: string[] = [];
        const it = make(sourcesOf(size, log, kind, true));
        log.length = 0;
        const length = typeof it.len === "function" ? it.len() : undefined;
        const got = length === undefined ? "no len" : seen(log.join(" "), unseen);
        const [wanted, gave] = [native(stdAnswers.get(`${stdForm}#${kind}${run} len`)), length === undefined ? undefined : answered("len", length)];
        if (got !== want || gave !== wanted) wrong.push(`len#${kind}${run} of sources with no len: std ${want} = ${wanted}; ${got} = ${gave}`);
      }
    }
    expect(wrong).toEqual([]);
    // A listed one that asks as std's does now comes off the list.
    const own = (key: string) => key.startsWith(`${stdForm} | ${form} `) && key.slice(`${stdForm} | ${form} `.length).split(" ")[0]! in asked;
    const fixed = Object.keys(differs).filter((key) => own(key) && !listed.has(key));
    expect(fixed).toEqual([]);
  });
}

// Each consumer made in place asks what it holds as std's method does, of
// a map of the source, where its form is chosen: `nth(1)` of one whose
// `nth` isn't the crate's, `[...it.drop(1).take(1)][0]`.
for (const [method, form, ask] of consumerForms) {
  test(`${form} asks what it holds as std's ${method} does`, () => {
    const { kinds: allowed, unseen } = runsOf(method, form);
    const wrong: string[] = [];
    for (const [kind, run, size] of runs.filter(([kind]) => allowed.includes(kind))) {
      const ran = stdRuns.get(`Map#${kind}${run} ${method}`);
      if (ran === undefined) continue;
      const log: string[] = [];
      const it = sourcesOf(size, log, kind).src().map(same);
      log.length = 0;
      ask(it);
      const [want, got] = [seen(ran, unseen), seen(log.join(" "), unseen)];
      if (got !== want) wrong.push(`${method}#${kind}${run}: std ${want}; ${got}`);
    }
    expect(wrong).toEqual([]);
  });
}

// The net runs each pair of the in-place forms' owner, and nothing else:
// src/lower/iterators/in_place_forms.txt's rows are the forms above, both
// ways, so a form the owner gains is run, and one the net runs is the
// owner's (Change A step 3, #2112).
test("the net runs each in-place form the owner names, and only those", () => {
  const run = [...forms, ...consumerForms].map(([stdName, form]) => `${stdName} | ${form}`).sort();
  expect(run).toEqual([...chosen.keys()].sort());
});

// A form chosen only of an array's items is run over one: what it's asked
// of the array runs nothing of the crate's, so no trace tells it from
// std's, but what it gives does.
test("each form chosen of an array's items gives std's items", () => {
  const ofItems = forms.filter(([stdName, form]) => chosen.get(`${stdName} | ${form}`) === "OfArrayItems");
  expect(ofItems.map(([stdName, form]) => `${stdName} | ${form}`)).toEqual(["StepBy | $Steps.filter by index"]);
  const items = (it: any) => Array.from({ [Symbol.iterator]: () => it });
  const plain = { src: () => std.$iter([1, 2, 3, 4, 5]), other: () => std.$iter([]), nested: () => std.$iter([]) };
  for (const [, , make] of ofItems) expect(items(make(plain))).toEqual([1, 3, 5]);
});

// Where a form is chosen, which the net runs it over, is a fact of every
// value of its source's type, or MIR's, never a crate-wide fact: each
// `Where`'s APIs are owners of docs/architecture.md of class 1 or 2, so
// the net's runs can't leave out a source a crate-wide fact chose a form
// for, #2197's kind (the lead, 2026-11-20; #2200).
test("each Where of the in-place forms is answered by owners of class 1 or 2", () => {
  const rows = ownerTable(readFileSync(join(import.meta.dir, "../docs/architecture.md"), "utf8"));
  const weak: string[] = [];
  for (const [at, apis] of whereAsks) {
    for (const api of apis) {
      const row = rows.find((row) => row.api.includes(api));
      if (row === undefined || row.source > 2) weak.push(`${at}: \`${api}\` ${row === undefined ? "is no owner's API" : `is a class ${row.source} owner's`}`);
    }
  }
  expect(weak).toEqual([]);
  expect([...whereAsks.keys()].sort()).toEqual(Object.keys(whereRuns).sort());
});

// Every iterator the runtime makes, each top-level function of src/runtime
// that makes a `$Steps`, is a form above, held to std's catalog, or is
// listed here with why it isn't: the net missed `$cycleOf`, which wrote
// neither `try_fold` nor `advance_by`, as it wasn't one (#2192).
const notForms: Record<string, string> = {
  $cycle: "a `cycle()` of std's iterator over an array: its items, no source whose steps it asks",
  $trustedZip: "std's `TrustedRandomAccess` zip, by index, which the catalog's sources, not random access, never take",
  $closing: "no std adapter: what drops what an iterator holds as it's closed, each step passed on, as the test above holds",
  $iterator: "the crate's own iterator, a source: its steps are its impl's",
  $lazyRange: "a range, a source",
  $lazyRangeBack: "a range stepped from its back, a source",
  $fromFn: "`from_fn`'s, a source",
  $repeating: "`repeat`'s, a source",
  $repeatingN: "`repeat_n`'s, a source",
  $decodeUtf16: "`decode_utf16`'s, of a string's units, which the catalog doesn't probe",
  $lazyMatchesBy: "a string's pattern iterator, a source",
  $lazyRsplitBy: "a string's pattern iterator, a source",
  $lazySplitBy: "a string's pattern iterator, a source",
  $lazySplitInclusiveBy: "a string's pattern iterator, a source",
  $lazySplitTerminatorBy: "a string's pattern iterator, a source",
  $lazyChunkBy: "a slice's iterator, a source",
  $lazySliceSplitBy: "a slice's iterator, a source",
  $hashSetUnion: "a hash set's union, a source",
  $setSteps: "a B-tree set's algebra, a source",
  $mapExtractIf: "a map's `extract_if`, a source",
  $grownBy: "no std iterator: what `collect` steps to grow a `Vec` as std's `from_iter` does",
  $roomForHint: "no std iterator: what `collect` steps to ask room as std's `from_iter` does",
  $tryProcessing: "`try_process`'s private `GenericShunt`, which no program can name for the catalog to make; the consumer tables' `option vec` and `result vec` rows hold it",
};

test("every iterator the runtime makes is a form above or says why not", () => {
  const dir = join(import.meta.dir, "../src/runtime");
  const makers = new Set<string>();
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".js"))) {
    // Each top-level function, to the next top-level declaration.
    for (const part of readFileSync(join(dir, file), "utf8").split(/^(?=function |class |const )/m)) {
      const name = /^function (\$\w+)\(/.exec(part)?.[1];
      if (name !== undefined && part.includes("new $Steps(")) makers.add(name);
    }
  }
  const covered = new Set([...forms, ...lentForms].map(([, form]) => form.split(" ")[0]!));
  expect([...makers].filter((name) => !covered.has(name) && notForms[name] === undefined)).toEqual([]);
  // And each listed one is still one, and not a form too.
  expect(Object.keys(notForms).filter((name) => !makers.has(name) || covered.has(name))).toEqual([]);
});

// Each form generic code's `eq` is given `LengthFirst` `true` of
// (`length_first`, `counts_where_given`, #1182) is counted by its hint, as
// std's `SpecIterEq` reads it: exact, its length, and its `len()` where it
// has one, as `$lengthFirst` reads no `len` (#2159).
test("each LengthFirst form's hint is exact and its length", () => {
  const drop = std.$dropEach(() => {});
  const forms: [string, () => any][] = [
    ["a slice's or a Vec's iterator, `$iter`", () => std.$iter([1, 2, 3])],
    ["one stepped from both ends", () => {
      const it = std.$iter([1, 2, 3, 4]);
      it.next();
      it.back();
      return it;
    }],
    ["an array or a Vec given whole, `$intoIter`", () => std.$intoIter([1, 2])],
    ["a Vec's IntoIter of items with a destructor, `$closing`", () => std.$closing(std.$iter([1, 2, 3]), drop)],
    ["one that drops each as it's passed", () => std.$closing(std.$intoIter([1, 2]), drop, false)],
    ["a range", () => std.$lazyRange(0, 3)],
    ["an empty range", () => std.$lazyRange(3, 3)],
  ];
  const wrong: string[] = [];
  for (const [name, make] of forms) {
    let n = 0;
    for (const it = make(); !it.next().done; ) n++;
    const it = make();
    const hint = std.$sizeHint(it);
    const len = typeof it.len === "function" ? it.len() : n;
    const [, counted] = std.$lengthFirst(make(), true);
    if (hint[0] !== n || hint[1] !== n || len !== n || counted !== n) wrong.push(`${name}: ${n} items, hint ${JSON.stringify(hint)}, len ${len}, counted ${counted}`);
  }
  expect(wrong).toEqual([]);
});
