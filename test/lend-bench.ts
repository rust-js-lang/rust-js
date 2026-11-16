// A micro-benchmark of the runtime's `$lent` made each step, as a compiled
// `filter`'s `next` lends its source each step: a getter of each step on
// each lend held runtime_long_writes past its 10 s (d90bda6bc, #2003). Its
// times are reported, never asserted, as a loaded machine's are noise:
// the nightly's `ci-spend` keeps them in #1989's comment and flags one
// grown past half again the night before's (scripts/ci-spend.ts).
//
//   bun test/lend-bench.ts [out.json]   each benchmark's best of 5, in ms

import { writeFileSync } from "node:fs";

import { $lent } from "../std/runtime.js";

/** The steps each benchmark takes, a lend each. */
export const steps = 200_000;

/** Each benchmark: what it lends, made once, stepped `steps` times, a lend a step. */
const benchmarks: Record<string, () => () => unknown> = {
  // A JS iterator, an array's.
  "$lent each step, a JS iterator": () => {
    const it = Array.from({ length: steps }, (_, i) => i).values();
    return () => $lent(it).next();
  },
  // Std's compiled struct, `$steps`' (ADR 0871): its lend a `Proxy`.
  "$lent each step, std's compiled struct": () => {
    let at = 0;
    const it = { $steps: true, iter: undefined, next: () => (at < steps ? { done: false, value: at++ } : { done: true, value: undefined }), [Symbol.iterator]() { return this; } };
    return () => $lent(it).next();
  },
};

/** Each benchmark's best of `tries`, in ms. */
export function measure(tries = 5): Record<string, number> {
  return Object.fromEntries(
    Object.entries(benchmarks).map(([name, make]) => {
      let best = Infinity;
      for (let t = 0; t < tries; t++) {
        const step = make();
        const start = performance.now();
        for (let i = 0; i < steps; i++) step();
        best = Math.min(best, performance.now() - start);
      }
      return [name, Math.round(best * 10) / 10];
    }),
  );
}

if (import.meta.main) {
  const times = measure();
  for (const [name, ms] of Object.entries(times)) console.log(`${name}: ${ms} ms for ${steps} steps`);
  if (process.argv[2]) writeFileSync(process.argv[2], JSON.stringify(times));
}
