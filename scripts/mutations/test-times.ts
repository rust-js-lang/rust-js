// The nightly's test times (scripts/testTimes.ts): what's merged, and what's
// said to be slower.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "test-times-parts-not-summed",
    breaks: "a file run in parts, the corpus, is timed as its last part alone",
    file: "scripts/testTimes.ts",
    find: "seconds: was.seconds + f.seconds, tests: was.tests + f.tests",
    replace: "seconds: f.seconds, tests: was.tests + f.tests",
    tests: ["test/test-times.test.ts"],
  },
  {
    name: "test-times-slower-by-a-minute",
    breaks: "a test 40 s slower than the last night's isn't said",
    file: "scripts/testTimes.ts",
    find: "return now - before > 30 ||",
    replace: "return now - before > 60 ||",
    tests: ["test/test-times.test.ts"],
  },
  {
    name: "test-times-quick-tests-flagged",
    breaks: "a test of a few seconds is said slower for a second's noise",
    file: "scripts/testTimes.ts",
    find: "(before >= 10 && now > 1.25 * before)",
    replace: "(now > 1.25 * before)",
    tests: ["test/test-times.test.ts"],
  },
  {
    name: "test-times-running-job-timed",
    breaks: "a job still running, the merge's own, is timed as ended",
    file: "scripts/testTimes.ts",
    find: '.filter((job) => job.completed_at && job.conclusion !== "skipped")',
    replace: '.filter((job) => job.conclusion !== "skipped")',
    tests: ["test/test-times.test.ts"],
  },
];
