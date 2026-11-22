// The catalogue of std's bodies `@rust-js/std` compiles, kept in
// std/compiled-std.json by `bun run std` and read by every compile whose
// std's crates are those it names (#2305), is what the walk of std's MIR
// says, the one owner: made again here, byte for byte.

import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
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

// Built again where it's the same, it's left as it is, its time too, as
// emit's `publish` leaves every file whose bytes are those (#2317):
// build.rs tracks std/compiled-std.json by its time, and the push's std
// phase rewriting it relinked the compiler in the next `cargo build`, as
// the push's React port ran it.
test("the catalogue built again, the same, is left as it is", () => {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-catalogue-"));
  try {
    const out = join(dir, "compiled-std.json");
    expect(catalogueBuild(compiler, out).code).toBe(0);
    const first = statSync(out, { bigint: true });
    Bun.sleepSync(20);
    const again = catalogueBuild(compiler, out);
    expect(again.stderr).toBe("");
    expect(again.code).toBe(0);
    const second = statSync(out, { bigint: true });
    expect([second.ino, second.mtimeNs]).toEqual([first.ino, first.mtimeNs]);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
