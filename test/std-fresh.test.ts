// std/'s freshness, the one check of it (scripts/std-fresh.ts), which
// `bun run push`'s std phase and check.yml's lint job run: a file of std/
// that differs from what's committed, or one that's new, is named, with the
// fix; none of a checkout where it's what's committed (#1971).

import { expect, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { staleSaid, staleStd } from "../scripts/std-fresh";
import { fixture } from "./support";

test("std/ that differs from what's committed is named, with the fix", () => {
  const dir = fixture("std-fresh");
  const git = (...args: string[]) => {
    const p = spawnSync("git", args, { cwd: dir, encoding: "utf8" });
    if (p.status !== 0) throw new Error(`git ${args.join(" ")}: ${p.stderr}`);
  };
  git("init", "-q");
  mkdirSync(join(dir, "std"), { recursive: true });
  writeFileSync(join(dir, "std", "runtime.js"), "export const a = 1;\n");
  writeFileSync(join(dir, "other.js"), "x\n");
  git("add", "-A");
  git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "std");
  expect(staleStd(dir)).toEqual([]);
  // What's outside std/ is no part of it.
  writeFileSync(join(dir, "other.js"), "y\n");
  expect(staleStd(dir)).toEqual([]);
  writeFileSync(join(dir, "std", "runtime.js"), "export const a = 2;\n");
  writeFileSync(join(dir, "std", "iter.js"), "export {};\n");
  expect(staleStd(dir)).toEqual([" M std/runtime.js", "?? std/iter.js"]);
  expect(staleSaid(staleStd(dir))).toContain("run `bun run std` and commit std/");
});
