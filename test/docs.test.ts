// The guide's examples are what the compiler makes of them
// (docs/guide/README.md#examples): each ```rust fence compiles, and the
// fences after it are its output, as `bun run docs` writes them.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { underLoad } from "../scripts/timing";
import { buildCompiler, fixturesOf, root } from "./support";

test("each guide page's output fences are what the compiler makes", () => {
  buildCompiler();
  const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check"], { cwd: root, timeout: 280_000 });
  expect(result.exitCode, result.stderr.toString()).toBe(0);
}, 290_000);

// Each run compiles an example in a directory of its own, claimed as a
// test's fixture is, so two runs in one checkout don't remove each other's
// as they compile (#2198): with another run's claim held on the first,
// `_0`, this run compiles in `_1` and leaves `_0` as it was.
test("a guide run compiles in its own directory, leaving another run's", () => {
  buildCompiler();
  const held = join(fixturesOf("docs.ts"), "docs-tutorial_vite.md-0_0");
  const mine = held.replace(/_0$/, "_1");
  rmSync(mine, { recursive: true, force: true });
  mkdirSync(held, { recursive: true });
  writeFileSync(join(held, "another-run.txt"), "held");
  writeFileSync(`${held}.owner`, String(process.pid));
  try {
    const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check", "--page", "docs/guide/tutorial/vite.md"], {
      cwd: root,
      env: { ...process.env, RUST_JS_KEEP_FIXTURES: "1" },
      timeout: underLoad(120_000),
    });
    expect(result.exitCode, result.stderr.toString()).toBe(0);
    expect(readFileSync(join(held, "another-run.txt"), "utf8")).toBe("held");
    expect(existsSync(mine)).toBe(true);
  } finally {
    for (const dir of [held, mine]) {
      rmSync(dir, { recursive: true, force: true });
      rmSync(`${dir}.owner`, { force: true });
    }
  }
}, underLoad(130_000));
