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
import { readFileSync } from "node:fs";
import { join } from "node:path";

import * as std from "../std/runtime.js";
import { stdRuns } from "./std-steps-catalog";

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
// back step, nor one with no `len` its length. A difference that no program
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
 * to two `inner`s of the second length. */
type Sources = { src: () => any; other: () => any; nested: () => any };
const sourcesOf = ([a, b]: [number, number], log: string[], kind: SourceKind): Sources => {
  const range = (from: number, n: number) => Array.from({ length: n }, (_, i) => from + i);
  return {
    src: () => loud("src", range(1, a), log, kind),
    other: () => loud("other", range(6, b), log, kind),
    nested: () => loud("src", range(0, Math.min(a, 2)).map(() => loud("inner", range(1, b), log, kind)), log, kind),
  };
};

const same = (x: unknown) => x;
/** Each of the runtime's forms of a std adapter, by std's name. */
const forms: [std: string, form: string, make: (s: Sources) => any][] = [
  ["Map", "$mapEnds", (s) => std.$mapEnds(s.src(), same)],
  ["Map", "$Steps.map", (s) => s.src().map(same)],
  ["Copied", "$copied", (s) => std.$copied(s.src(), same)],
  ["Cloned", "$mapEnds", (s) => std.$mapEnds(s.src(), same)],
  ["Cloned", "$Steps.map", (s) => s.src().map(same)],
  ["Filter", "$filterEnds", (s) => std.$filterEnds(s.src(), (x: number) => x % 2 === 1)],
  ["Filter", "$Steps.filter", (s) => s.src().filter((x: number) => x % 2 === 1)],
  ["Enumerate", "$enumerate", (s) => std.$enumerate(s.src())],
  ["Skip", "$skipping", (s) => std.$skipping(s.src(), 1)],
  ["Take", "$taking", (s) => std.$taking(s.src(), 3)],
  ["StepBy", "$stepBy", (s) => std.$stepBy(s.src(), 2)],
  ["StepBy", "$stepByEnds", (s) => std.$stepByEnds(s.src(), 2)],
  ["Chain", "$lazyChain", (s) => std.$lazyChain(s.src(), s.other())],
  ["Zip", "$lazyZip", (s) => std.$lazyZip(s.src(), s.other())],
  ["Rev", "$rev", (s) => std.$rev(s.src())],
  ["Flatten", "$flatMap", (s) => std.$flatMap(s.nested(), same)],
  ["Flatten", "$flattenEnds", (s) => std.$flattenEnds(s.nested(), same)],
  ["TakeWhile", "$lazyTakeWhile", (s) => std.$lazyTakeWhile(s.src(), (x: number) => x < 4)],
  ["SkipWhile", "$lazySkipWhile", (s) => std.$lazySkipWhile(s.src(), (x: number) => x < 2)],
  ["&mut I", "$lent", (s) => std.$lent(s.src())],
  ["&mut dyn I", "$lent unsized", (s) => std.$lent(s.src(), "lentUnsized")],
  ["Box<I>", "$lent boxed", (s) => std.$lent(s.src(), "boxed")],
  ["Box<dyn I>", "$lent boxedUnsized", (s) => std.$lent(s.src(), "boxedUnsized")],
];

/** Each run: its kind of source, its number and its sources' lengths. */
const runs = kinds.flatMap((kind) => sizes.map((size, run) => [kind, run, size] as const));

/** Each form's step that asks other than std's does, a wrong answer not
 * yet fixed, under its issue: none is let differ for good (#2112). */
const differs: Record<string, string> = {
};

for (const [stdForm, form, make] of forms) {
  test(`${form} asks what it holds as std's ${stdForm} does`, () => {
    const wrong: string[] = [];
    const listed = new Set<string>();
    for (const step of Object.keys(asked)) {
      for (const [kind, run, size] of runs) {
        const want = stdRuns.get(`${stdForm}#${kind}${run} ${step}`);
        if (want === undefined) continue;
        const log: string[] = [];
        const it = make(sourcesOf(size, log, kind));
        // A form for one end: none of the other's steps.
        if ((/back|rfold|rfind|rposition/.test(step) && typeof it.back !== "function") || (/^len$|rposition/.test(step) && typeof it.len !== "function")) continue;
        log.length = 0;
        asked[step]!(it);
        const got = log.join(" ");
        if (got === want) continue;
        const key = `${form} ${step}${kind === "" ? "" : ` ${kind}`}`;
        if (differs[key] === undefined) wrong.push(`${step}#${kind}${run}: std ${want}; ${got}`);
        else listed.add(key);
      }
    }
    expect(wrong).toEqual([]);
    // A listed one that asks as std's does now comes off the list.
    const fixed = Object.keys(differs).filter((key) => key.startsWith(`${form} `) && !listed.has(key));
    expect(fixed).toEqual([]);
  });
}

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
