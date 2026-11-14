// The bridge's list (ADR 1137, #1672): each wrap of the corpus the bridge
// leaves out, one THIR's ranges know never wraps and MIR's intervals don't
// prove yet, is in test/intervals-bridged.txt, with the issue of the MIR
// precision that would prove it; and each listed one still is. A new one
// is a gap to recover by MIR's precision, not to add; one MIR now proves
// goes. M4 deletes THIR's ranges and the bridge together once it's empty.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fixture, root } from "./support";
import { underLoad } from "../scripts/timing";

// The issues a site may cite: a closure's (#1701), `Ref`s of one `RefCell`
// (#1741).
const issues = ["#1701", "#1741"];

test("each wrap the bridge leaves out is listed, with its issue, and each listed one still is", () => {
  const out = join(fixture("intervals-bridged"), "shadow.txt");
  const shadow = Bun.spawnSync(["bun", join(root, "scripts", "intervals-shadow.ts"), "--out", out], { cwd: root, stdout: "pipe", stderr: "pipe" });
  expect(shadow.stderr.toString()).not.toContain("failed to compile");
  expect(shadow.exitCode).toBe(0);
  const corpus = `${join(root, "test", "corpus")}/`;
  const bridged = readFileSync(out, "utf8")
    .split("\n")
    .filter((line) => line.startsWith("bridged\t"))
    .map((line) => {
      const [, owner, where] = line.split("\t");
      return `${where.replace(corpus, "").split(": ")[0]}\t${owner}`;
    })
    .sort();
  const listed = readFileSync(join(root, "test", "intervals-bridged.txt"), "utf8")
    .split("\n")
    .filter((line) => line !== "" && !line.startsWith("#"));
  for (const line of listed) expect([line, issues.includes(line.split("\t")[2])]).toEqual([line, true]);
  const sites = listed.map((line) => line.split("\t").slice(0, 2).join("\t")).sort();
  expect({ unlisted: bridged.filter((site) => !sites.includes(site)), stale: sites.filter((site) => !bridged.includes(site)) }).toEqual({ unlisted: [], stale: [] });
}, underLoad(600_000));
