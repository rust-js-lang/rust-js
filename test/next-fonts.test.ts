import { expect, test } from "bun:test";
import { join } from "node:path";

import { root } from "./support";

// next/src/font/google.rs is what next/generate-fonts.ts writes of Next.js's
// declarations (ADR 0359): run it after Next.js is updated.
test("next's Google Fonts are those of Next.js's declarations", () => {
  const generated = Bun.spawnSync([process.execPath, join(root, "next/generate-fonts.ts"), "--check"], { stderr: "pipe" });
  expect([generated.exitCode, generated.stderr.toString()]).toEqual([0, ""]);
});
