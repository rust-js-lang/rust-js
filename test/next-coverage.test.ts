// The next crate's coverage of Next.js's public modules is a ratchet: each
// export, bound or not, is listed in next/coverage.txt, and losing one fails
// here, as gaining one does until it's blessed (`BLESS=1`), so the diff
// shows it. See docs/decisions/0346-next-coverage.md.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { measure, render } from "../next/coverage";
import { root } from "./support";
import { underLoad } from "../scripts/timing";

const baseline = join(root, "next/coverage.txt");

test("the next crate binds what it bound of Next.js", async () => {
  const now = await measure();
  if (process.env.BLESS) writeFileSync(baseline, render(now));
  const bound = (text: string) => text.split("\n").filter((l) => l.startsWith("+ ")).map((l) => l.slice(2));
  const before = existsSync(baseline) ? bound(readFileSync(baseline, "utf8")) : [];
  const boundNow = new Set(bound(render(now)));
  // Each export bound before still is.
  expect(before.filter((x) => !boundNow.has(x))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(render(now)).toBe(readFileSync(baseline, "utf8"));
}, underLoad(120_000));
