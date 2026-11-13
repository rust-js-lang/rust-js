// The nightly's React Compiler sweep (ADR 1142): the lowering tests'
// programs that use React, each test file that compiles one run with its
// fixtures kept, and React Compiler over every module of React's they
// wrote, as the per-push net (test/react-compiler.test.ts) runs it over
// the guide, examples/ and its cases. A function it skips or refuses
// fails, but one test/react-compiler-known.txt lists of `sweep:<fixture>`.
//
//   bun scripts/react-compiler-sweep.ts [part/parts]
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import { basename, join, relative } from "node:path";
import { failures, known, names } from "../test/react-compiler-check";

const root = join(import.meta.dir, "..");
const fixtures = join(root, "target/fixtures");
const [part, parts] = (process.argv[2] ?? "1/1").split("/").map(Number);

// Each test file that compiles a program using React, this part's.
const files = readdirSync(join(root, "test"))
  .filter((name) => name.endsWith(".test.ts") && readFileSync(join(root, "test", name), "utf8").includes("use react::"))
  .sort()
  .filter((_, i) => i % parts === part - 1)
  .map((name) => `test/${name}`);

// What this run writes, not an earlier's.
const started = Date.now();
const ran = Bun.spawnSync(["bun", "test", "--timeout=600000", ...files], {
  cwd: root,
  env: { ...process.env, RUST_JS_KEEP_FIXTURES: "1" },
  stdout: "inherit",
  stderr: "inherit",
});

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (["node_modules", "target", "std", "dist"].includes(name) || name.startsWith(".")) return [];
    try {
      return statSync(path).isDirectory() ? walk(path) : [path];
    } catch {
      return [];
    }
  });

const entries = known();
const unlisted = new Set<string>();
const met = new Set<number>();
let modules = 0;
// Each file's fixtures are under its scope, `x` and its name's hash
// (test/support.ts), so parts on one machine walk only their own.
const scopes = files.map((file) => join(fixtures, `x${createHash("sha256").update(basename(file)).digest("hex").slice(0, 8)}`));
const written = scopes.flatMap((scope) => (existsSync(scope) ? walk(scope) : []));
for (const path of written.filter((p) => /\.(jsx?|mjs)$/.test(p) && statSync(p).mtimeMs >= started)) {
  const code = readFileSync(path, "utf8");
  // What the compiler wrote, beside its map, not a test's own text.
  if (!existsSync(`${path}.map`) || (!path.endsWith(".jsx") && !code.includes('from "react'))) continue;
  modules++;
  // `sweep:jsx-syntax`, of `target/fixtures/x5585429c/jsx-syntax_16/lib.jsx`.
  const fixture = relative(fixtures, path).split("/")[1]?.replace(/_\d+$/, "");
  const source = `sweep:${fixture}`;
  for (const failure of failures(code, path)) {
    const entry = entries.find((e) => names(e, source, failure));
    if (entry) met.add(entry.line);
    else unlisted.add(`${source}\t${failure.fn}\t${failure.reason}\t(${failure.kind}, ${relative(root, path)})`);
  }
}
console.log(`React Compiler over ${modules} modules of ${files.length} test files' programs`);
if (unlisted.size) {
  console.error(`React Compiler skipped or refused these, which test/react-compiler-known.txt doesn't list:\n${[...unlisted].join("\n")}`);
}
// Run whole, an entry of the sweep's nothing fails of is stale.
const stale = parts === 1 ? entries.filter((e) => e.source.startsWith("sweep:") && !met.has(e.line)) : [];
for (const e of stale) console.error(`test/react-compiler-known.txt:${e.line} is stale, nothing fails of it: take it off`);
process.exit(unlisted.size || stale.length || ran.exitCode ? 1 : 0);
