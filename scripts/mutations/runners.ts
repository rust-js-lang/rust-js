// The runners a scheduled workflow may hold at once (scripts/runners.ts):
// what its `if:`s rule out, a matrix's `max-parallel`, and the most jobs
// no `needs` orders.
import type { Mutation } from "../mutations";

const bound = ["test/ci.test.ts", "-t", "the runners' bound reads"];

export const mutations: Mutation[] = [
  {
    name: "runners-unknown-if-ruled-out",
    breaks: "an `if:` a run's results decide, `needs.*.result`, is read as false, and a job that may run holds no runner",
    file: "scripts/runners.ts",
    find: "    if (/^inputs\\.[\\w-]+$/.test(text)) return { known: true, value: inputs[text.slice(\"inputs.\".length)] ?? \"\" };\n    return { known: false };\n",
    replace: "    if (/^inputs\\.[\\w-]+$/.test(text)) return { known: true, value: inputs[text.slice(\"inputs.\".length)] ?? \"\" };\n    return { known: true, value: \"\" };\n",
    tests: bound,
  },
  {
    name: "runners-implied-success-unread",
    breaks: "a job whose `if:` asks no status is counted though a job it needs was ruled out, which GitHub skips it for",
    file: "scripts/runners.ts",
    find: "    return !asksStatus && [job.needs ?? []].flat().some((need) => out.has(need));\n",
    replace: "    return false;\n",
    tests: bound,
  },
  {
    name: "runners-max-parallel-unread",
    breaks: "a matrix job's `max-parallel` is unread, so the Nightly's capped matrices count every job of theirs",
    file: "scripts/runners.ts",
    find: "    const at = size === undefined ? capped : capped === undefined ? size : Math.min(size, capped);\n",
    replace: "    const at = size;\n",
    tests: bound,
  },
  {
    name: "runners-chains-unmerged",
    breaks: "the most at once is every job's runners summed, as though no `needs` ordered any",
    file: "scripts/runners.ts",
    find: "  return total - merged;\n",
    replace: "  return total;\n",
    tests: bound,
  },
  {
    name: "runners-caller-inputs-unread",
    breaks: "a called workflow is read without the inputs its caller gives, so the Nightly's `part` and `max_parallel` rule nothing out",
    file: "scripts/runners.ts",
    find: "      const called = heldJobs(job.uses.slice(\"./.github/workflows/\".length), given, `${name}/`, read);\n",
    replace: "      const called = heldJobs(job.uses.slice(\"./.github/workflows/\".length), {}, `${name}/`, read);\n",
    tests: ["test/ci.test.ts", "-t", "a scheduled workflow holds at most NIGHTLY_RUNNERS"],
  },
];
