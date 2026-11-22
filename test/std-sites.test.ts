// The runtime's `$stdSites`, runtime/std_sites.js, where std raises each
// panic std's hook names its own line of, is what the walk of a whole
// std's MIR makes (#2292, ADR 1184): `bun run std` writes it, and one a
// change to the walk would move is caught here.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { raisedBuild, stdSites, stdSitesFile } from "../scripts/std";
import { compiler, fixture, root } from "./support";

test("the runtime's sites of std's panics are what std's MIR says", () => {
  const out = join(fixture("std-sites"), "raised.json");
  const built = raisedBuild(compiler, root, out);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(stdSites(readFileSync(out, "utf8"), root)).toBe(readFileSync(stdSitesFile, "utf8"));
}, 300_000);
