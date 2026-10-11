// What a change is checked by before it's committed, all at once, in a
// minute or so: lint, the MIR corpus and the JSX tests MIR lists in parts,
// what THIR shares only where the change touches it, and the mutations it
// adds or changes. CI runs the whole suite and every mutation of what the
// change touched after (DEVELOPMENT.md).
//
//   bun run check:fast                 the change's checks
//   bun run check:fast --all           THIR's suites too
//   bun run check:fast test/x.test.ts  and these files
import { readFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const args = process.argv.slice(2);
const all = args.includes("--all");
const extra = args.filter((arg) => !arg.startsWith("--"));

const git = (gitArgs: string[]) => Bun.spawnSync(["git", ...gitArgs], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
const changed = [...git(["diff", "--name-only", "HEAD"]), ...git(["ls-files", "--others", "--exclude-standard"])];
// What THIR's lowering, or what both share, reads: all of the compiler but
// MIR's own lowering, and the runtime.
const mirOnly = (file: string) => file === "src/lower/mir.rs" || file.startsWith("src/lower/mir/");
const shared = all || changed.some((file) => (file.startsWith("src/") && !mirOnly(file)) || file.startsWith("runtime/"));
const corpus = shared || changed.some((file) => file.startsWith("test/corpus") || ["test/support.ts", "test/programs.ts"].includes(file));
const jsx = all || changed.some((file) => /^src\/(lower\/jsx|jsx_syntax)|^react\//.test(file));

// The mutations the change adds or changes: each `name` its lists' diff adds.
const diff = Bun.spawnSync(["git", "diff", "HEAD", "--", "scripts/mutations"], { cwd: root }).stdout.toString();
const untracked = changed.filter((file) => file.startsWith("scripts/mutations/") && !diff.includes(file));
const added = [...diff.matchAll(/^\+\s*"?name"?: "([^"]+)"/gm), ...untracked.flatMap((file) => [...readFileSync(join(root, file), "utf8").matchAll(/"?name"?: "([^"]+)"/g)])].map((m) => m[1]);

const build = Bun.spawnSync(["cargo", "build", "--quiet"], { cwd: root, stdout: "inherit", stderr: "inherit" });
if (build.exitCode !== 0) process.exit(1);

// Each part of a file of cases, a process of its own (test/support.ts).
const parts = Math.max(2, Math.floor(availableParallelism() / 4));
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
  ...inParts("MIR corpus", "test/mir.test.ts"),
  ...(corpus ? inParts("corpus", "test/corpus.test.ts") : []),
  ...(jsx ? [["JSX", ["bun", "test", "test/jsx.test.ts"], {}] as [string, string[], Record<string, string>]] : []),
  // What pins the JS THIR writes, the names it gives and what it refuses.
  ...(shared
    ? ["lowering", "compiler", "snapshots", "diagnostics"].map(
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
const compiler = join(root, "target", "debug", "rust-js");
const started = Date.now();
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
    const seconds = ((Date.now() - start) / 1000).toFixed(0);
    console.log(`${code === 0 ? "ok  " : "FAIL"} ${name} (${seconds}s)`);
    return { name, code, log: out + err };
  }),
);
const failed = results.filter((r) => r.code !== 0);
for (const { name, log } of failed) console.log(`\n── ${name} ──\n${log.slice(-3000)}`);
console.log(`\n${failed.length === 0 ? "all passed" : `${failed.length} failed`} in ${((Date.now() - started) / 1000).toFixed(0)}s${shared ? "" : " (MIR only: THIR's corpus, JSX and lowering skipped)"}`);
process.exit(failed.length === 0 ? 0 : 1);
