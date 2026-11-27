// Mutations of crates/rust-js-program/src/runtime/tests.js (ADR 0692, amended).
import type { Mutation } from "../../mutations";

const refusedFirst = ["test/rust-js-test-runners.test.ts", "-t", "by node, a test after one refused at run time is run again"];

export const mutations: Mutation[] = [
  {
    name: "test-after-a-refusal-runs",
    breaks: "a test after a refusal at run time in its process runs on, its drops' writes in a `finally` left undone as `$live()` is false, `least = Least(key)` among them, and gives 4 for 3 (itertools' `extrema_set`, #2625)",
    file: "crates/rust-js-program/src/runtime/tests.js",
    find: "  if (!$refused) return f();\n",
    replace: "  return f();\n",
    tests: refusedFirst,
  },
  {
    name: "test-refused-unrecorded",
    breaks: "a test refused after a refusal at run time in its process isn't written down, so `rust-js test` never runs it again in a process of its own, and it stays refused (#2625)",
    file: "crates/rust-js-program/src/runtime/tests.js",
    find: '  if (alone) $nodeFs("`rust-js test`").appendFileSync(alone, JSON.stringify({ name }) + "\\n");\n',
    replace: "",
    tests: refusedFirst,
  },
];
