// A mutation run reads the crate as it began (scripts/mutations.ts): its
// builds and its mutations' sites are of a snapshot taken once, so an edit
// to `src/` while an hour's run goes on is the next run's, not in the
// mutants built after it.
import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { crateText, snapshotCrate, unjudgedSaid } from "../scripts/mutations";

test("a run's builds and sites read the crate's snapshot, not the checkout as it's edited", () => {
  const dir = mkdtempSync(join(tmpdir(), "mutation-snapshot-"));
  try {
    const checkout = join(dir, "checkout"), snapshot = join(dir, "snapshot");
    mkdirSync(join(checkout, "src"), { recursive: true });
    writeFileSync(join(checkout, "src", "main.rs"), "fn main() {}\n");
    expect(snapshotCrate(snapshot, checkout)).toEqual([join("src", "main.rs")]);
    writeFileSync(join(checkout, "src", "main.rs"), "// an edit while the run goes on\n");
    expect(crateText("src/main.rs")).toBe("fn main() {}\n");
    expect(readFileSync(join(snapshot, "src", "main.rs"), "utf8")).toBe("fn main() {}\n");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// A retirement that judged fewer names than it was asked says so last,
// and fails: its trailers alone read as if it judged them all (#2272).
test("a retirement that loses names says which, last", () => {
  expect(unjudgedSaid(7, [])).toBeUndefined();
  expect(unjudgedSaid(7, ["a", "b", "c", "d", "e", "f"])).toBe("✗ --retire judged 1 of 7: a, b, c, d, e, f said nothing, so this run is no evidence of them; run them again");
  expect(unjudgedSaid(2, ["a"])).toBe("✗ --retire judged 1 of 2: a said nothing, so this run is no evidence of it; run it again");
});
