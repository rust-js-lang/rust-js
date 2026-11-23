// Mutations of scripts/artifacts.ts: a run's artifacts downloaded, each
// retry into an empty directory.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "artifact-retry-into-what-was-left",
    breaks: "a download asked again extracts into what the failed one left, `openat timings.json: file exists`, and fails again (main's check of 6d9fcfb45)",
    file: "scripts/artifacts.ts",
    find: "    rmSync(dir, { recursive: true, force: true });\n    mkdirSync(dir, { recursive: true });\n    const got = fetch(",
    replace: "    mkdirSync(dir, { recursive: true });\n    const got = fetch(",
    tests: ["test/ci.test.ts", "-t", "asked again into an empty directory"],
  },
  {
    name: "artifact-unmatched-asked-again",
    breaks: "a download that matches no artifact is asked again, twice, 30 s of waiting for what no run kept",
    file: "scripts/artifacts.ts",
    find: "    if (once) return false;\n",
    replace: "",
    tests: ["test/ci.test.ts", "-t", "asked again into an empty directory"],
  },
  {
    name: "artifact-timed-out-asked-again",
    breaks: "a download past its time is asked again, twice, as slow, where the push's map took 12 min at some 85 KB/s (#2385)",
    file: "scripts/artifacts.ts",
    find: "    const once = none || got.timedOut === true;\n",
    replace: "    const once = none;\n",
    tests: ["test/ci.test.ts", "-t", "asked again into an empty directory"],
  },
];
