// Mutations of test/workers.ts (ADR 0093): the worker a test process keeps
// between its jobs, each answered as its own builds' compiles would be
// (#2301 step 2, ADR 1182).
import type { Mutation } from "../mutations";

const tested = ["test/worker.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "worker-death-unsaid",
    breaks: "a worker that ends is said to have ended and no more, its panic's message dropped: 4 builds ran apart in each of two nightly parts, named by nothing (#2448)",
    file: "test/workers.ts",
    find: "    const said = worker.said.text.trim();\n",
    replace: "    const said = \"\";\n",
    tests: [...tested, "-t", "a worker that ends says what it said"],
  },
  {
    name: "worker-not-kept",
    breaks: "each job starts a worker of its own, paying the compiler's start again (#2301 step 2)",
    file: "test/workers.ts",
    find: "  const worker = kept;\n",
    replace: "  const worker = kept;\n  kept = undefined;\n",
    tests: tested,
  },
  {
    name: "worker-answers-swapped",
    breaks: "a job's answers are given to its builds in the other order: the program is given the library's exit and output (#2301)",
    file: "test/workers.ts",
    find: "  return answer.builds.map((build) => {\n",
    replace: "  return answer.builds.reverse().map((build) => {\n",
    tests: tested,
  },
  {
    name: "worker-kept-past-its-share",
    breaks: "a kept worker holding more than a test's share of the machine's memory is kept on: twelve of them keep 13 to 20 GB on a 16 GB machine (#2301)",
    file: "test/workers.ts",
    find: "  if (kept?.key !== key || kept.held > share) stop();\n",
    replace: "  if (kept?.key !== key) stop();\n",
    tests: tested,
  },
];
