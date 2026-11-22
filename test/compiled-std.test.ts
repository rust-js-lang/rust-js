// The catalogue of std's bodies `@rust-js/std` compiles, kept in
// std/compiled-std.json by `bun run std` and read by every compile whose
// std's crates are those it names (#2305), is what the walk of std's MIR
// says, the one owner: made again here, byte for byte.

import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { catalogueBuild } from "../scripts/std";
import { stdDir } from "./packages-of";
import { compiler } from "./support";

test("the catalogue kept is the walk's of each of its groups", () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-catalogue-"));
  try {
    const out = join(dir, "compiled-std.json");
    const built = catalogueBuild(compiler, out);
    expect(built.stderr).toBe("");
    expect(built.code).toBe(0);
    expect(readFileSync(out, "utf8")).toBe(readFileSync(join(stdDir, "compiled-std.json"), "utf8"));
    // Of a std whose crates it names, the base and each group of its own.
    const kept = JSON.parse(readFileSync(out, "utf8"));
    expect(Object.keys(kept.crates)).toContain("core");
    expect(kept.base.length).toBeGreaterThan(0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
