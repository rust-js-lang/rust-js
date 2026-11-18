// Each iterator the runtime makes that passes its source's steps on, by
// `$forward`, reaches each step its source has of its own, not `$Steps`'
// default of it over `next`, which steps otherwise where the source's
// differs: a `skip(1)`'s `tryFold` asks `nth(0)` where its `next` asks
// `nth(1)` (#2051), and a `filter`'s keeps what its accumulator holds as
// a stage panics (#2056). Every step is `$stepNames`, lent.js's, the one
// list; what a `&mut I` and a `Box<I>` pass on is test/passed-on.test.ts's.
// A step a wrapper doesn't pass on says why here, so a sixth one can't be
// missed: `$closing` lacked `tryFold`, and `$flattenEnds` `tryFold` and
// what `last` takes, `inners`, which `$flatMap` had (#2051).

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import * as std from "../std/runtime.js";

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
    ["fold", "tryFold", "count", "inners"].filter((step) => Object.hasOwn(made, step)).sort();
  const items = () => std.$iter([[1], [2]]);
  const flatMap = std.$flatMap(items(), (x: unknown) => x);
  const ends = std.$flattenEnds(items(), (x: unknown) => x);
  expect(own(ends)).toEqual(own(flatMap));
  expect(own(flatMap)).toEqual(["count", "fold", "inners", "tryFold"]);
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
