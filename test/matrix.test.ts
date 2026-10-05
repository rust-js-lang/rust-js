// The boundary matrices in the corpus are what scripts/matrix.ts writes
// (ADR 0182): an edit goes there, where what's left out is listed.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { excluded, matrices } from "../scripts/matrix";
import { root } from "./support";

test("the boundary matrices are what their generator writes", () => {
  for (const [name, source] of Object.entries(matrices)) {
    expect(readFileSync(join(root, "test", "corpus", name), "utf8"), name).toBe(source);
  }
});

test("what the matrices leave out says why", () => {
  for (const [what, why] of Object.entries(excluded)) expect(why.length, what).toBeGreaterThan(0);
});
