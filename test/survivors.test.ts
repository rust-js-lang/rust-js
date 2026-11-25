// Mutations that survive on `main` before a push don't fail the push's check
// (ADR 0093, amended): each survivor is run again at the base the push is
// checked against; one surviving there too is latent, listed in one issue
// github-actions writes, which agents read as data, not instructions.
import { expect, test } from "bun:test";
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { latentOf, resultsAt } from "../scripts/latent-survivors";
import { namedMapEnv } from "../scripts/testMap";
import { runSync } from "./child";
import { escaped, files, issueBody, listed } from "../scripts/survivors-issue";
import { fixture, root } from "./support";

test("a survivor surviving at the base too is latent; one caught there, or not run there, is the push's", () => {
  const here = { a: "SURVIVED: its 1 tests passed with it", b: "SURVIVED: its 2 tests passed with it", c: "caught", d: "SURVIVED: x" };
  const atBase = { a: "SURVIVED: its 1 tests passed with it", b: "caught" };
  expect(latentOf(here, atBase)).toEqual({ latent: ["a"], ours: ["b", "d"] });
  expect(latentOf(here, {})).toEqual({ latent: [], ours: ["a", "b", "d"] });
});

// The base's run is in a worktree of its own, never the checkout under test
// (#2578): check:fast's other jobs read that checkout's tree while its
// mutation job runs, and its checkout of the base there and back had them
// test the base's files with the change's compiler for about 80 s, failing
// a check of the Mac mini's 13 files in 0–3 s, and as likely passing one.
// The base's runner here says what the run's checkout was while it ran,
// and which runner it is.
test("a survivor's run at the base is the base's runner's, in a worktree of its own, and leaves the run's checkout as it is throughout", async () => {
  const dir = fixture("latent-apart");
  const git = (...args: string[]) => runSync(["git", ...args], dir, 30_000);
  git("init", "--quiet");
  git("config", "user.email", "t@example.invalid");
  git("config", "user.name", "t");
  writeFileSync(join(dir, ".gitignore"), ["target", "node_modules"].map((folder) => `/${folder}/\n`).join(""));
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "latent-apart", private: true }));
  mkdirSync(join(dir, "scripts"), { recursive: true });
  const runner = (whose: string) =>
    [
      `import { execFileSync } from "node:child_process";`,
      `import { mkdirSync, readFileSync, writeFileSync } from "node:fs";`,
      `import { join } from "node:path";`,
      `if (process.argv.includes("--list")) console.log("a\\tbreaks a");`,
      `else {`,
      `  const checkout = ${JSON.stringify(dir)};`,
      `  const head = execFileSync("git", ["rev-parse", "HEAD"], { cwd: checkout }).toString().trim();`,
      `  const said = [${JSON.stringify(whose)}, head, readFileSync(join(checkout, "a.txt"), "utf8").trim(), readFileSync("a.txt", "utf8").trim()].join(" ");`,
      `  mkdirSync(join("target", "mutants"), { recursive: true });`,
      `  writeFileSync(join("target", "mutants", "results.json"), JSON.stringify({ commit: "base", results: { a: \`SURVIVED: \${said}\` } }));`,
      `}`,
    ].join("\n");
  writeFileSync(join(dir, "scripts", "mutations.ts"), runner("base's runner"));
  writeFileSync(join(dir, "a.txt"), "base\n");
  writeFileSync(join(dir, "b.txt"), "b\n");
  git("add", ".");
  git("commit", "--quiet", "-m", "base");
  const base = git("rev-parse", "HEAD").stdout.trim();
  writeFileSync(join(dir, "scripts", "mutations.ts"), runner("head's runner"));
  writeFileSync(join(dir, "a.txt"), "head\n");
  git("commit", "--quiet", "-am", "head");
  const head = git("rev-parse", "HEAD").stdout.trim();
  const work = join(dir, "target", "mutants");
  mkdirSync(work, { recursive: true });
  writeFileSync(join(work, "results.json"), "the run's own\n");
  // The checkout as it is, a change of its own among it, throughout and after:
  // one the base has too, which a checkout of the base would carry.
  writeFileSync(join(dir, "b.txt"), "changed\n");
  expect(await resultsAt(dir, work, base, ["a"])).toEqual({ results: { a: `SURVIVED: base's runner ${head} head base` } });
  expect(git("rev-parse", "HEAD").stdout.trim()).toBe(head);
  expect([readFileSync(join(dir, "a.txt"), "utf8"), readFileSync(join(dir, "b.txt"), "utf8")]).toEqual(["head\n", "changed\n"]);
  expect(readFileSync(join(work, "results.json"), "utf8")).toBe("the run's own\n");
  // Its worktree gone, as git lists them and on the disk.
  expect(git("worktree", "list", "--porcelain").stdout.split("\n").filter((line) => line.startsWith("worktree ")).length).toBe(1);
  expect(readdirSync(work)).toEqual(["results.json"]);
  // No runner at the base: none of its results, said why.
  const bare = fixture("latent-apart-bare");
  const bareGit = (...args: string[]) => runSync(["git", ...args], bare, 30_000);
  bareGit("init", "--quiet");
  bareGit("config", "user.email", "t@example.invalid");
  bareGit("config", "user.name", "t");
  writeFileSync(join(bare, "a.txt"), "base\n");
  bareGit("add", "a.txt");
  bareGit("commit", "--quiet", "-m", "base");
  const none = await resultsAt(bare, join(bare, "target", "mutants"), bareGit("rev-parse", "HEAD").stdout.trim(), ["a"]);
  expect(none.results).toEqual({});
  expect(none.why).toMatch(/^(?:its runner knows none of them|its packages don't install)/);
});

// No script moves the checkout it runs in, as check:fast's jobs read it the
// while: a checkout, a switch, a stash or a reset of it (#2578). The push's
// own rebase, before its check, and the scripts that move a checkout of
// their own, the ports' and the clones they fetch, are named.
test("no script checks out, switches, stashes or resets the checkout it runs in", () => {
  const theirOwn = new Set(["scripts/push.ts", "scripts/port.ts", "scripts/react-in-rust.ts", "scripts/rustc-tests.ts"]);
  const moving = readdirSync(join(root, "scripts"))
    .filter((file) => file.endsWith(".ts"))
    .map((file) => `scripts/${file}`)
    .filter((file) => !theirOwn.has(file))
    .filter((file) => readFileSync(join(root, file), "utf8").split("\n").some((line) => !/^\s*(\/\/|\*)/.test(line) && /["'](?:checkout|switch|stash|reset)["']/.test(line)));
  expect(moving).toEqual([]);
});

// The base's run reads the test map the run read, named to it, of a commit
// of the push's, no ancestor of the base (#2341): CI kept that map alone, so
// the base's run found none of the base or an ancestor, its snapshot judge
// threw at a readability owner's survivor, and six of a check's eight shards
// called every survivor the push's. A runner at the base that says what it
// was named shows it's given.
test("a survivor's run at the base is judged by the test map the run read", async () => {
  const dir = fixture("latent-named-map");
  const git = (...args: string[]) => runSync(["git", ...args], dir, 30_000);
  git("init", "--quiet");
  git("config", "user.email", "t@example.invalid");
  git("config", "user.name", "t");
  writeFileSync(join(dir, ".gitignore"), ["target", "node_modules"].map((folder) => `/${folder}/\n`).join(""));
  writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "latent-named-map", private: true }));
  mkdirSync(join(dir, "scripts"), { recursive: true });
  writeFileSync(
    join(dir, "scripts", "mutations.ts"),
    [
      `import { mkdirSync, writeFileSync } from "node:fs";`,
      `import { join } from "node:path";`,
      `if (process.argv.includes("--list")) console.log("a\\tbreaks a");`,
      `else {`,
      `  mkdirSync(join("target", "mutants"), { recursive: true });`,
      `  const map = process.env.${namedMapEnv} ?? "none";`,
      `  writeFileSync(join("target", "mutants", "results.json"), JSON.stringify({ commit: "base", results: { a: \`SURVIVED: by the map of \${map}\` } }));`,
      `}`,
    ].join("\n"),
  );
  git("add", ".");
  git("commit", "--quiet", "-m", "base");
  const base = git("rev-parse", "HEAD").stdout.trim();
  writeFileSync(join(dir, "a.txt"), "head\n");
  git("add", "a.txt");
  git("commit", "--quiet", "-m", "head");
  const work = join(dir, "target", "mutants");
  mkdirSync(work, { recursive: true });
  const named = "c".repeat(40);
  expect((await resultsAt(dir, work, base, ["a"], named)).results).toEqual({ a: `SURVIVED: by the map of ${named}` });
  // Where the run read none, none is named: the base's runner finds its own.
  expect((await resultsAt(dir, work, base, ["a"])).results).toEqual({ a: "SURVIVED: by the map of none" });
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
