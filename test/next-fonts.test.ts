import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { generatorOutput, root } from "./support";

// next/src/font/google.rs is what next/generate-fonts.ts writes of Next.js's
// declarations (ADR 0359): run it after Next.js is updated.
test("next's Google Fonts are those of Next.js's declarations", () => {
  expect(generatorOutput("next/generate-fonts.ts")).toBe(readFileSync(join(root, "next/src/font/google.rs"), "utf8"));
});
