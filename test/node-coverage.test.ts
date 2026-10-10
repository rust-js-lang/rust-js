// The node crate's coverage of @types/node is a
// ratchet: each export, bound or not, is listed in node/coverage.txt, and
// losing one fails here, as gaining one does until it's blessed
// (`BLESS=1`), so the diff shows it. See docs/decisions/0351-node-coverage.md.

import { expect, test } from "bun:test";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { measure, render } from "../node/coverage";
import { root } from "./support";

const baseline = join(root, "node/coverage.txt");

test("the node crate binds what it bound of Node.js", async () => {
  const now = await measure();
  if (process.env.BLESS) writeFileSync(baseline, render(now));
  const bound = (text: string) => text.split("\n").filter((l) => l.startsWith("+ ")).map((l) => l.slice(2));
  const before = existsSync(baseline) ? bound(readFileSync(baseline, "utf8")) : [];
  const boundNow = new Set(bound(render(now)));
  // Each export bound before still is.
  expect(before.filter((x) => !boundNow.has(x))).toEqual([]);
  // And what's new is in the baseline, blessed.
  expect(render(now)).toBe(readFileSync(baseline, "utf8"));
}, 120_000);
