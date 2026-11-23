// The guide's examples are what the compiler makes of them
// (docs/guide/README.md#examples): each ```rust fence compiles, and the
// fences after it are its output, as `bun run docs` writes them.
import { expect, test } from "bun:test";
import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { underLoad } from "../scripts/timing";
import { buildCompiler, fixtureOf, root } from "./support";

test("each guide page's output fences are what the compiler makes", () => {
  buildCompiler();
  const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check"], { cwd: root, timeout: 280_000 });
  expect(result.exitCode, result.stderr.toString()).toBe(0);
}, 290_000);

// Each run compiles an example in a directory of its own, claimed as a
// test's fixture is, so two runs in one checkout don't remove each other's
// as they compile (#2198): with another run's claim held on the first,
// `_0`, this run compiles in another, the one it says it kept, and leaves
// `_0` as it was. Which, and whether it's there after: neither is this
// test's, as another guide run may claim `_1` first, and another
// process's sweep removes a kept one once its process is gone (#2422).
test("a guide run compiles in its own directory, leaving another run's", () => {
  buildCompiler();
  // Another run's claim, taken as a run takes its own, on the first index
  // no live process holds: a guide run beside it holds one too.
  const held = fixtureOf("docs.ts", "docs-tutorial_vite.md-0");
  writeFileSync(join(held, "another-run.txt"), "held");
  let mine: string | undefined;
  try {
    const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check", "--page", "docs/guide/tutorial/vite.md"], {
      cwd: root,
      env: { ...process.env, RUST_JS_KEEP_FIXTURES: "1" },
      timeout: underLoad(120_000),
    });
    expect(result.exitCode, result.stderr.toString()).toBe(0);
    expect(readFileSync(join(held, "another-run.txt"), "utf8")).toBe("held");
    mine = /^kept (.*docs-tutorial_vite\.md-0_\d+)$/m.exec(result.stderr.toString())?.[1];
    expect(mine).toBeDefined();
    expect(mine).not.toBe(held);
    expect(mine!.slice(0, mine!.lastIndexOf("_"))).toBe(held.slice(0, held.lastIndexOf("_")));
  } finally {
    if (mine !== undefined) {
      rmSync(mine, { recursive: true, force: true });
      rmSync(`${mine}.owner`, { force: true });
    }
  }
}, underLoad(130_000));
