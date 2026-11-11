// `scripts/file-sizes.ts`' check that a file near the cap doesn't grow
// (#1334), of a repository made here: a file over 1,350 lines that grew
// since where the change left main, or that main hadn't, is one to split
// by concept first; one that shrank, or that's under it, isn't.
import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { NEAR, grownNearCap, mergeBase, nearCapReport, splitFirst, staleMain } from "../scripts/file-sizes";

const lines = (n: number) => Array.from({ length: n }, (_, i) => `// ${i}`).join("\n") + "\n";

test("a file over 1,350 lines that grew since main is told to be split first", () => {
  const at = mkdtempSync(join(tmpdir(), "file-sizes-"));
  try {
    const git = (...args: string[]) => {
      const run = Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd: at });
      if (run.exitCode !== 0) throw new Error(run.stderr.toString());
      return run.stdout.toString().trim();
    };
    const put = (file: string, n: number) => writeFileSync(join(at, file), lines(n));
    git("init", "-q");
    mkdirSync(join(at, "src"));
    put("src/near.rs", NEAR + 10);
    put("src/small.rs", 1000);
    put("src/shrinks.rs", NEAR + 50);
    put("src/still.rs", NEAR + 20);
    git("add", "-A");
    git("commit", "-q", "-m", "main");
    git("update-ref", "refs/remotes/origin/main", "HEAD");
    const main = git("rev-parse", "HEAD");
    // The change: a commit of its own, and what's not committed yet.
    put("src/near.rs", NEAR + 11);
    put("src/made.rs", NEAR + 1);
    git("add", "-A");
    git("commit", "-q", "-m", "change");
    put("src/small.rs", 1200);
    put("src/shrinks.rs", NEAR + 40);
    put("src/new.rs", NEAR + 5);

    const base = mergeBase(at);
    expect(base).toBe(main);
    const grown = grownNearCap(["src"], base!, at);
    expect(grown).toEqual([
      ["src/near.rs", NEAR + 10, NEAR + 11],
      ["src/new.rs", 0, NEAR + 5],
      ["src/made.rs", 0, NEAR + 1],
    ]);
    expect(splitFirst(grown[0])).toBe(
      "src/near.rs grew from 1360 to 1361 lines, past 1350: split it by concept in its own commit first, never squeeze it (the user's rule, #1334)",
    );
    // Split by concept, the file left under it is no longer grown.
    put("src/near.rs", NEAR - 100);
    expect(grownNearCap(["src"], base!, at).map(([file]) => file)).toEqual(["src/new.rs", "src/made.rs"]);
  } finally {
    rmSync(at, { recursive: true, force: true });
  }
});

// m1-lisa's check named three files others had grown on main: its
// `origin/main` was stale, so main's newer commits, which the branch was
// rebased onto, looked like the branch's own.
test("a file that looks grown against a stale origin/main says to fetch first", () => {
  const at = mkdtempSync(join(tmpdir(), "file-sizes-"));
  try {
    const git = (cwd: string, ...args: string[]) => {
      const run = Bun.spawnSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd });
      if (run.exitCode !== 0) throw new Error(run.stderr.toString());
      return run.stdout.toString().trim();
    };
    const [origin, clone] = [join(at, "origin"), join(at, "clone")];
    mkdirSync(join(origin, "src"), { recursive: true });
    git(origin, "init", "-q", "-b", "main");
    writeFileSync(join(origin, "src/big.rs"), lines(NEAR + 10));
    git(origin, "add", "-A");
    git(origin, "commit", "-q", "-m", "main");
    git(at, "clone", "-q", origin, clone);
    const fetched = git(clone, "rev-parse", "origin/main");
    // Another's commit on main grows the file; the clone has it, rebased
    // onto, but its `origin/main` is where it was fetched.
    writeFileSync(join(origin, "src/big.rs"), lines(NEAR + 20));
    git(origin, "commit", "-q", "-am", "another's");
    git(clone, "pull", "-q", "--rebase", origin, "main");
    git(clone, "update-ref", "refs/remotes/origin/main", fetched);

    expect(grownNearCap(["src"], mergeBase(clone)!, clone).map(([file]) => file)).toEqual(["src/big.rs"]);
    const now = git(origin, "rev-parse", "HEAD");
    expect(staleMain(clone)).toEqual({ local: fetched, remote: now });
    expect(nearCapReport(["src"], clone)).toEqual([
      `origin/main is ${fetched.slice(0, 9)}, behind origin's main at ${now.slice(0, 9)}: what looks grown (src/big.rs) may be others' commits, so fetch origin first and measure again (#1334)`,
    ]);
    // Fetched, it's main's, and nothing grew.
    git(clone, "fetch", "-q", "origin");
    expect(staleMain(clone)).toBeUndefined();
    expect(nearCapReport(["src"], clone)).toEqual([]);
  } finally {
    rmSync(at, { recursive: true, force: true });
  }
});

test("a checkout with no origin/main has no merge base to measure from", () => {
  const at = mkdtempSync(join(tmpdir(), "file-sizes-"));
  try {
    Bun.spawnSync(["git", "init", "-q"], { cwd: at });
    expect(mergeBase(at)).toBeUndefined();
  } finally {
    rmSync(at, { recursive: true, force: true });
  }
});
