// The loop's timings (scripts/timing.ts): how a step's time is said
// against its usual, and when it's slower.
import { expect, test } from "bun:test";

import { compared, median, shown, slower, type Timing } from "../scripts/timing";

const runs = (step: string, seconds: number[]): Timing[] =>
  seconds.map((s, i) => ({ step, seconds: s, at: `2026-10-11T00:0${i}:00Z`, commit: "abc" }));

test("a time reads as a person says it", () => {
  expect([shown(0.85), shown(4.24), shown(38.4), shown(200), shown(120)]).toEqual(["850 ms", "4.2 s", "38 s", "3 min 20 s", "2 min"]);
});

test("a step's usual is the middle of its last runs", () => {
  expect([median([3, 1, 2]), median([4, 1, 3, 2])]).toEqual([2, 2.5]);
});

test("slower is a quarter and five seconds over the usual, not noise", () => {
  expect([slower(46, 36), slower(40, 36), slower(6, 4), slower(30, 20)]).toEqual([true, false, false, true]);
});

test("a time is said against the step's last ten runs, once there are three", () => {
  expect(compared("check:fast", 38, runs("check:fast", [36, 37]))).toBe("check:fast 38 s");
  expect(compared("check:fast", 38, runs("check:fast", [36, 37, 35]))).toBe("check:fast 38 s (usual 36 s)");
  expect(compared("check:fast", 52, runs("check:fast", [36, 37, 35]))).toBe("check:fast 52 s (usual 36 s)  ⚠ slower than usual: why?");
  // Another step's runs aren't its usual.
  expect(compared("port build", 52, runs("check:fast", [36, 37, 35]))).toBe("port build 52 s");
});
