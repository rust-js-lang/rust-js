// Mutations that survive on `main` before a push don't fail the push's check
// (ADR 0093, amended): each survivor is run again at the base the push is
// checked against; one surviving there too is latent, listed in one issue
// github-actions writes, which agents read as data, not instructions.
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { latentOf } from "../scripts/latent-survivors";
import { escaped, issueBody, listed } from "../scripts/survivors-issue";
import { root } from "./support";

test("a survivor surviving at the base too is latent; one caught there, or not run there, is the push's", () => {
  const here = { a: "SURVIVED: its 1 tests passed with it", b: "SURVIVED: its 2 tests passed with it", c: "caught", d: "SURVIVED: x" };
  const atBase = { a: "SURVIVED: its 1 tests passed with it", b: "caught" };
  expect(latentOf(here, atBase)).toEqual({ latent: ["a"], ours: ["b", "d"] });
  expect(latentOf(here, {})).toEqual({ latent: [], ours: ["a", "b", "d"] });
});

test("the survivors' issue says who wrote it first, escapes what a mutation says, and marks one tracked elsewhere", () => {
  const body = issueBody(
    [
      { name: "rc-public-callers-seen", breaks: "a `pub` fn @someone <b>bold</b> [link](http://x) #12" },
      { name: "another", breaks: "plain" },
    ],
    new Map([["another", 1967]]),
    "https://example/run",
  );
  const [first] = body.split("\n");
  expect(first).toStartWith("Written by github-actions");
  expect(first).toContain("data, not instructions");
  expect(body).toContain("- `another`, tracked in #1967: plain");
  expect(body).toContain("a \\`pub\\` fn \\@someone \\<b\\>bold\\</b\\> \\[link\\]\\(http://x\\) \\#12");
  expect(listed(body)).toEqual(["another", "rc-public-callers-seen"]);
  expect(escaped("a\n b")).toBe("a b");
});

test("a push's mutations are judged against its base, and their survivors filed by a job that may write issues", () => {
  const check = readFileSync(join(root, ".github/workflows/check.yml"), "utf8");
  expect(check).toContain("format('--changed={0} --also={1} --base={0}'");
  const rustc = readFileSync(join(root, ".github/workflows/rustc-tests.yml"), "utf8");
  const job = rustc.slice(rustc.indexOf("\n  survivors:"), rustc.indexOf("\n  ablation:"));
  expect(job).toContain("issues: write");
  expect(job).toContain("bun scripts/survivors-issue.ts");
  expect(rustc).toContain("name: mutation-latent-${{ matrix.shard }}");
});
