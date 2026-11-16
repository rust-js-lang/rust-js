// Mutations that survive on `main` before a push don't fail the push's check
// (ADR 0093, amended): each survivor is run again at the base the push is
// checked against; one surviving there too is latent, listed in one issue
// github-actions writes, which agents read as data, not instructions.
import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { latentOf } from "../scripts/latent-survivors";
import { escaped, files, issueBody, listed } from "../scripts/survivors-issue";
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

// A called workflow's every job is held to what its caller grants, checked
// before anything runs: a job asking for more fails the whole run at its
// start (`startup_failure`), as check.yml's `rustc` call did once
// rustc-tests.yml's `survivors` job asked for `issues: write`. So every
// call of a workflow of ours grants each permission any of its jobs asks.
test("every call of a reusable workflow grants what its jobs ask", () => {
  const dir = join(root, ".github/workflows");
  const level = { none: 0, read: 1, write: 2 } as Record<string, number>;
  const parse = (name: string) => Bun.YAML.parse(readFileSync(join(dir, name), "utf8")) as any;
  const asked = (name: string): Record<string, string> => {
    const want: Record<string, string> = {};
    for (const job of Object.values<any>(parse(name).jobs ?? {})) {
      const perms = job.permissions && typeof job.permissions === "object" ? job.permissions : {};
      for (const [scope, value] of Object.entries<string>(perms)) {
        if ((level[value] ?? 0) > (level[want[scope]] ?? 0)) want[scope] = value;
      }
    }
    return want;
  };
  const short: string[] = [];
  for (const file of readdirSync(dir).filter((f) => /\.ya?ml$/.test(f))) {
    const flow = parse(file);
    for (const [id, job] of Object.entries<any>(flow.jobs ?? {})) {
      const called = typeof job.uses === "string" && job.uses.match(/^\.\/\.github\/workflows\/(.+)$/)?.[1];
      if (!called) continue;
      const granted = job.permissions ?? flow.permissions ?? {};
      for (const [scope, value] of Object.entries(asked(called))) {
        if ((level[granted[scope]] ?? 0) < level[value]) short.push(`${file}'s ${id} calls ${called}, whose jobs ask ${scope}: ${value}`);
      }
    }
  }
  expect(short).toEqual([]);
});

// A push's run with no latent survivor uploads no `latent.json`, so the
// `survivors` job downloads nothing: nothing to add, not a failure (main's
// checks 43871190255 and 43872319768 failed on the missing directory).
test("a push's run with no latent survivor has nothing to add to the issue", () => {
  expect(files(join(root, "target", "no-such-survivors"), "latent.json")).toEqual([]);
  const run = Bun.spawnSync([process.execPath, "scripts/survivors-issue.ts", "latent", join(root, "target", "no-such-survivors"), "https://example.invalid/run"], { cwd: root, env: { ...process.env, GH_TOKEN: "", PATH: "/nonexistent" }, stdout: "pipe", stderr: "pipe" });
  expect([run.exitCode, run.stdout.toString().trim()]).toEqual([0, "Mutation survivors on main: no latent survivor to add"]);
});
