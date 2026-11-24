// Mutations of test/bounded.ts: the disk's floor, which the loop's stores
// keep nothing below, sized by the volume (#2505).
import type { Mutation } from "../mutations";

const sized = ["test/support.test.ts", "-t", "the disk's floor and its stores' caps are sized by the volume"];

export const mutations: Mutation[] = [
  {
    name: "disk-floor-fixed",
    breaks: "the disk's floor is 40 GiB on every disk, 18% of a 256 GB Mac's, whose stores then keep nothing under 40 free (#2505)",
    file: "test/bounded.ts",
    find: "  const floor = Math.min(40 * GiB, Math.max(20 * GiB, 0.08 * volume));",
    replace: "  const floor = 40 * GiB;",
    tests: sized,
  },
  {
    name: "disk-floor-unbounded",
    breaks: "the disk's floor grows with the disk past the 40 GiB that was enough everywhere, so a roomy disk keeps less than before (#2505)",
    file: "test/bounded.ts",
    find: "  const floor = Math.min(40 * GiB, Math.max(20 * GiB, 0.08 * volume));",
    replace: "  const floor = Math.max(20 * GiB, 0.08 * volume);",
    tests: sized,
  },
];
