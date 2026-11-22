// The runtime's `$stdSites`, runtime/std_sites.js, where std raises each
// panic std's hook names its own line of, is what the walk of a whole
// std's MIR makes (#2292, ADR 1184): `bun run std` writes it, and one a
// change to the walk would move is caught here.
import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { raisedBuild, stdSites, stdSitesFile } from "../scripts/std";
import { compiler, fixture, root, run } from "./support";

// A path into a machine's own copy of std's source or its home, which no
// JS rust-js writes may hold: std's sites are `/rustc/<commit>/library/..`
// on every machine (ADR 0262's difference 62, #2343).
const local = /\/\.rustup\/|lib\/rustlib\/src\/rust\/|\/Users\/|\/home\//;
const commit = /^commit-hash: (\S+)$/m.exec(run(["rustc", "-vV"]))?.[1];

// Each `.js` file under `dir`, its path.
const jsUnder = (dir: string): string[] =>
  (readdirSync(join(root, dir), { recursive: true }) as string[]).filter((file) => file.endsWith(".js")).map((file) => join(dir, file));

test("no committed JS rust-js wrote names a machine's own source", () => {
  const files = [...jsUnder("std"), ...jsUnder("crates/rust-js-program/src/runtime"), ...jsUnder("test/corpus"), ...jsUnder("test/snapshots")];
  expect(files.length).toBeGreaterThan(3000);
  expect(files.filter((file) => local.test(readFileSync(join(root, file), "utf8")))).toEqual([]);
});

test("std's bodies name std's files as the shipped std does, wherever its source is", () => {
  const dir = fixture("std-sites-bodies");
  const built = runSync(
    [compiler, "std/lib.rs", "-o", join(dir, "std.js"), "--library", "--manifest", join(dir, "manifest.json"), "--std-bodies", "--", "--crate-name", "rust_js_std"],
    root,
    600_000,
  );
  expect(built.code).toBe(0);
  const iter = readFileSync(join(dir, "iter.js"), "utf8");
  expect(iter).toContain(`"/rustc/${commit}/library/core/src/iter/adapters/step_by.rs:`);
  expect(readdirSync(dir).filter((file) => file.endsWith(".js") && local.test(readFileSync(join(dir, file), "utf8")))).toEqual([]);
}, 600_000);

test("the runtime's sites of std's panics are what std's MIR says", () => {
  const out = join(fixture("std-sites"), "raised.json");
  const built = raisedBuild(compiler, root, out);
  expect(built.stderr).toBe("");
  expect(built.code).toBe(0);
  expect(stdSites(readFileSync(out, "utf8"), root)).toBe(readFileSync(stdSitesFile, "utf8"));
}, 300_000);
