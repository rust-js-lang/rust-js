// The guide's examples are what the compiler makes of them
// (docs/guide/README.md#examples): each ```rust fence compiles, and the
// fences after it are its output, as `bun run docs` writes them.
import { expect, test } from "bun:test";
import { buildCompiler, root } from "./support";

test("each guide page's output fences are what the compiler makes", () => {
  buildCompiler();
  const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check"], { cwd: root, timeout: 280_000 });
  expect(result.exitCode, result.stderr.toString()).toBe(0);
}, 290_000);
