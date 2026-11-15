// The nightly's test times (scripts/testTimes.ts): bun's JUnit reports
// and GitHub's job times merged, the slowest named, and what's slower than
// the last night's said.
import { expect, test } from "bun:test";
import { jobTimes, merge, parseJunit, slower, slowerThan, summary, type TestTimes } from "../scripts/testTimes";

const report = (file: string, seconds: number, cases: [string, number][]) =>
  `<?xml version="1.0" encoding="UTF-8"?>\n<testsuites name="bun test">\n  <testsuite name="${file}" file="${file}" tests="${cases.length}" time="${seconds}">\n` +
  cases.map(([name, s]) => `    <testcase name="${name}" classname="" time="${s}" file="${file}" line="1" />\n`).join("") +
  "  </testsuite>\n</testsuites>\n";

test("a JUnit report's files and tests are read with their times, its names unescaped", () => {
  expect(parseJunit(report("test/a.test.ts", 2.5, [["it&apos;s &lt;slow&gt;", 2], ["quick", 0.5]]))).toEqual({
    files: [{ file: "test/a.test.ts", seconds: 2.5, tests: 2 }],
    tests: [
      { file: "test/a.test.ts", name: "it's <slow>", seconds: 2 },
      { file: "test/a.test.ts", name: "quick", seconds: 0.5 },
    ],
  });
});

test("a file run in parts is summed, a test run twice its longest, each the slowest first", () => {
  const times = merge(
    [report("test/corpus.test.ts", 30, [["a.rs", 10]]), report("test/corpus.test.ts", 40, [["b.rs", 20]]), report("test/b.test.ts", 50, [["x", 3]]), report("test/b.test.ts", 1, [["x", 4]])],
    [],
    "c",
    "r",
  );
  expect(times.files).toEqual([
    { file: "test/corpus.test.ts", seconds: 70, tests: 2 },
    { file: "test/b.test.ts", seconds: 51, tests: 2 },
  ]);
  expect(times.tests.map((t) => [t.name, t.seconds])).toEqual([["b.rs", 20], ["a.rs", 10], ["x", 4]]);
});

test("a job's time and its steps' are from when each started to when it ended, a skipped or running one left out", () => {
  const at = (s: number) => new Date(Date.UTC(2026, 10, 15, 0, 0, s)).toISOString();
  expect(
    jobTimes([
      { name: "check / test (1/2)", started_at: at(0), completed_at: at(90), conclusion: "success", steps: [{ name: "Run bun run test", started_at: at(10), completed_at: at(80) }] },
      { name: "skipped", started_at: null, completed_at: at(0), conclusion: "skipped" },
      { name: "running", started_at: at(0), completed_at: null, conclusion: null },
    ]),
  ).toEqual([{ job: "check / test (1/2)", seconds: 90, steps: [{ name: "Run bun run test", seconds: 70 }] }]);
});

test("what's slower than the last night's by more than 30 s, or a quarter of 10 s or more, is said", () => {
  expect(slower(41, 10)).toBe(true);
  expect(slower(13, 10)).toBe(true);
  expect(slower(12, 10)).toBe(false);
  expect(slower(9, 5)).toBe(false);
  expect(slower(40, 9.9)).toBe(true);
  const night = (seconds: number): TestTimes => ({
    commit: "abcdef1234",
    run: "1",
    files: [{ file: "test/a.test.ts", seconds, tests: 1 }],
    tests: [{ file: "test/a.test.ts", name: "t", seconds }],
    jobs: [{ job: "check / wasm", seconds: seconds * 10, steps: [] }],
  });
  expect(slowerThan(night(100), night(60)).map((w) => w.what)).toEqual(["job check / wasm", "file test/a.test.ts", "test test/a.test.ts > t"]);
  expect(slowerThan(night(60), night(100))).toEqual([]);
  const text = summary(night(100), night(60), 1);
  expect(text).toContain("### Slower than the last night's (abcdef123): 3");
  expect(text).toContain("| check / wasm | 16 min 40 s |");
  expect(text).toContain("### The 1 slowest tests");
  expect(summary(night(1))).toContain("No earlier night's times to compare.");
});
