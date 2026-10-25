// What a change is checked by before it's committed, all at once, in a
// minute or so: lint, the corpus in parts, the suites that check the JS
// bodies are lowered to from their MIR (ADR 0364), the snapshots and the
// diagnostics where the change touches what they read, and the mutations it
// adds or changes. CI runs the whole suite and every mutation of what the
// change touched after each push (DEVELOPMENT.md).
//
//   bun run check:fast                 the change's checks
//   bun run check:fast --all           every suite it can touch
//   bun run check:fast test/x.test.ts  and these files
import { readFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";
import { elapsed, timed } from "./timing";

const root = join(import.meta.dir, "..");
const args = process.argv.slice(2);
const all = args.includes("--all");
const extra = args.filter((arg) => !arg.startsWith("--"));

const git = (gitArgs: string[]) => Bun.spawnSync(["git", ...gitArgs], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
const changed = [...git(["diff", "--name-only", "HEAD"]), ...git(["ls-files", "--others", "--exclude-standard"])];
// What the snapshots and the diagnostics read: the compiler and the runtime.
const shared = all || changed.some((file) => file.startsWith("src/") || file.startsWith("runtime/"));
const jsx = all || changed.some((file) => /^src\/(lower\/jsx|jsx_syntax)|^react\//.test(file));
// A change to tidy is checked after each of its stages (ADR 0659).
const tidy = all || changed.some((file) => /^src\/lower\/mir\/tidy/.test(file));

// The mutations the change adds or changes: each `name` its lists' diff adds.
const diff = Bun.spawnSync(["git", "diff", "HEAD", "--", "scripts/mutations"], { cwd: root }).stdout.toString();
const untracked = changed.filter((file) => file.startsWith("scripts/mutations/") && !diff.includes(file));
const added = [...diff.matchAll(/^\+\s*"?name"?: "([^"]+)"/gm), ...untracked.flatMap((file) => [...readFileSync(join(root, file), "utf8").matchAll(/"?name"?: "([^"]+)"/g)])].map((m) => m[1]);

// Each step's time is kept, and said against its usual (scripts/timing.ts).
const building = Date.now();
const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
if (build.exitCode !== 0) process.exit(1);
console.log(timed("cargo build", (Date.now() - building) / 1000));

// And the node bindings the bindings and lowering tests compile against,
// once, before the jobs that ask for them: built by the first test to ask
// under the others' load, it outran bun's 5 s test timeout, was killed, and
// failed every test waiting on it (#227). support.ts's `buildNode`, whose
// stamp skips it where nothing changed.
const compiled = join(root, "target", "debug", "rust-js");
const binding = Date.now();
const node = Bun.spawnSync(["bun", "-e", "import { buildNode } from './test/support.ts'; buildNode();"], {
  cwd: root,
  env: { ...process.env, RUST_JS_COMPILER: compiled },
  stdout: "inherit",
  stderr: "inherit",
});
if (node.exitCode !== 0) process.exit(1);
console.log(timed("node bindings build", (Date.now() - binding) / 1000));

// Each part of a file of cases, a process of its own (test/support.ts):
// one for each two cores. A case is a compile, then its runs, one core at a
// time, so the cores a part leaves are the other jobs'. On 12 cores, 3
// parts took 85 s, 6 take 70 s, and 10 took 72 s, the other jobs twice as
// long and a test of them timing out: the check is bound by its CPU time.
const parts = Math.max(2, Math.floor(availableParallelism() / 2));
const inParts = (name: string, file: string) =>
  Array.from({ length: parts }, (_, i): [string, string[], Record<string, string>] => [
    `${name} ${i + 1}/${parts}`,
    ["bun", "test", file],
    { RUST_JS_CASES: `${i + 1}/${parts}` },
  ]);
const jobs: [string, string[], Record<string, string>][] = [
  ["fmt", ["bun", "run", "fmt:check"], {}],
  ["typecheck", ["bun", "run", "typecheck"], {}],
  ["clippy", ["cargo", "clippy", "--locked", "--quiet", "--", "-D", "warnings"], {}],
  ["architecture, mutation lists", ["bun", "test", "test/architecture.test.ts", "test/mutations.test.ts"], {}],
  ...inParts("corpus", "test/corpus.test.ts"),
  // What checks the JS bodies are lowered to.
  // Each a file of a concept's (ADR 0572), `test/lowering` all of lowering's.
  ...(
    [
      ["lowering", ["test/lowering"]],
      ["compiler", ["test/compiler.test.ts"]],
      ["bindings", ["test/bindings.test.ts", "test/js-builtins.test.ts", "test/node-bindings.test.ts", "test/webapi-bindings.test.ts"]],
    ] as const
  ).map(([name, files]): [string, string[], Record<string, string>] => [name, ["bun", "test", ...files], {}]),
  ...(jsx ? [["JSX", ["bun", "test", "test/jsx"], {}] as [string, string[], Record<string, string>]] : []),
  ...(tidy ? [["tidy's stages", ["bun", "test", "test/tidy-stages.test.ts"], {}] as [string, string[], Record<string, string>]] : []),
  // What pins the JS written, the names it gives and what it refuses; and
  // the guide's examples compiled, std's package and the readability ratchet,
  // each of which a lowering change has turned `main` red with (#212, #222).
  ...(shared
    ? [["guide, std package, readability", ["bun", "test", "test/docs.test.ts", "test/std-package.test.ts", "test/readability.test.ts"], {}] as [string, string[], Record<string, string>]]
    : []),
  ...(shared
    ? ["snapshots", "diagnostics"].map(
        (name): [string, string[], Record<string, string>] => [name, ["bun", "test", `test/${name}.test.ts`], {}],
      )
    : []),
  ...(added.length > 0
    ? [["mutations the change adds", ["bun", "scripts/mutations.ts", ...new Set(added)], {}] as [string, string[], Record<string, string>]]
    : []),
  ...extra.map((file): [string, string[], Record<string, string>] => [file, ["bun", "test", file], {}]),
];

// Each job is given the compiler just built: one that builds its own, as
// each test file and fmt does, links it again, and races the others.
const compiler = compiled;
const results = await Promise.all(
  jobs.map(async ([name, cmd, env]) => {
    const start = Date.now();
    const p = Bun.spawn(cmd, {
      cwd: root,
      env: { ...process.env, RUST_JS_COMPILER: compiler, ...env },
      stdout: "pipe",
      stderr: "pipe",
    });
    const timeout = setTimeout(() => p.kill(), 290_000);
    const [out, err] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text()]);
    const code = await p.exited;
    clearTimeout(timeout);
    const seconds = (Date.now() - start) / 1000;
    console.log(
      code === 0 ? `ok   ${timed(`check:fast ${name}`, seconds)}` : `FAIL ${name} (${seconds.toFixed(0)}s)`,
    );
    return { name, code, log: out + err };
  }),
);
const failed = results.filter((r) => r.code !== 0);
for (const { name, log } of failed) console.log(`\n── ${name} ──\n${log.slice(-3000)}`);
// From its start, its build too.
const total = elapsed();
const scope = shared ? "check:fast" : "check:fast, the change's";
console.log(
  `\n${failed.length === 0 ? `all passed: ${timed(scope, total)}` : `${failed.length} failed in ${total.toFixed(0)}s`}${shared ? "" : " (JSX, snapshots and diagnostics skipped)"}`,
);
process.exit(failed.length === 0 ? 0 : 1);
