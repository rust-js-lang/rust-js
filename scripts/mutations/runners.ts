// The runners a scheduled workflow may hold at once (scripts/runners.ts):
// what its `if:`s rule out, a matrix's `max-parallel`, and the most jobs
// no `needs` orders.
import type { Mutation } from "../mutations";

const bound = ["test/runners.test.ts", "-t", "the runners' bound reads"];

export const mutations: Mutation[] = [
  {
    name: "runners-unknown-if-ruled-out",
    breaks: "an `if:` a run's results decide, `needs.*.result`, is read as false, and a job that may run holds no runner",
    file: "scripts/runners.ts",
    find: "    return { known: false };\n  };\n  const compared",
    replace: "    return { known: true, value: \"\" };\n  };\n  const compared",
    tests: bound,
  },
  {
    name: "runners-contains-of-an-input-unread",
    breaks: "a `contains` of a caller's input and a literal, `contains(inputs.mutation_args, '--corpus')`, is unknown, so a `--corpus` dispatch's job counts as one more of the nightly's runners (#2404)",
    file: "scripts/runners.ts",
    find: "    if (holds) return { known: true,",
    replace: "    if (holds && false) return { known: true,",
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
    name: "runners-string-cap-taken",
    breaks: "a `max-parallel` that reads an input without `fromJSON` is taken as its number, though GitHub, given a string, creates none of the matrix's jobs",
    file: "scripts/runners.ts",
    find: "  if (/\\binputs\\./.test(expr)) throw new Error(",
    replace: "  if (/\\binputs\\./.test(expr)) return Number(valueOf(value, inputs));\n  if (false) throw new Error(",
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
    tests: ["test/runners.test.ts", "-t", "a scheduled workflow holds at most NIGHTLY_RUNNERS"],
  },
];
