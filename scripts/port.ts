// The react.dev port, a Rust app beside this checkout (`../rust-js-react.dev`,
// or `$RUST_JS_PORT`), compiled and read against the JS it commits:
//
//   bun run port measure [--keep]  how much of its JS is written otherwise now
//   bun run port compare           its build's 823 pages against the reference
//
// Each starts from the JS as committed and puts it back after, `--keep`
// leaving the new to read. A compile that fails stops it: what failed to
// compile keeps the JS before, which would read as no difference.
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { elapsed, timed } from "./timing";

const root = join(import.meta.dir, "..");
const port = resolve(process.env.RUST_JS_PORT ?? join(root, "..", "react.dev-in-rust"));
const reference = `${port}.reference`;
const [command, ...flags] = process.argv.slice(2);
const generated = [":(glob)src/**/*.js", ":(glob)src/**/*.jsx", ":(glob)src/**/*.d.ts"];

function run(cmd: string[], env: Record<string, string> = {}, timeout = 280_000) {
  const p = Bun.spawnSync(cmd, { cwd: port, env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe", timeout });
  return { code: p.exitCode, out: p.stdout.toString() + p.stderr.toString() };
}

/** The JS as committed: what the port's Rust is, of its own, left as it is. */
function restore() {
  const changed = run(["git", "status", "--porcelain", "--", "src"]).out.split("\n").filter(Boolean);
  const own = changed.filter((line) => !/\.(js|jsx|d\.ts)$/.test(line));
  if (own.length > 0) throw new Error(`the port has changes of its own, not its JS:\n${own.join("\n")}`);
  run(["git", "checkout", "--", ...generated]);
}

/** MIR's compile of it, or what stopped it. */
function compile() {
  const { code, out } = run(["node", join(root, "tooling", "cli.js"), "build"]);
  const errors = out.split("\n").filter((line) => line.startsWith("error"));
  if (code !== 0 || errors.length > 0) {
    restore();
    throw new Error(`MIR's compile of the port failed:\n${out.slice(-4000)}`);
  }
}

if (!existsSync(port)) throw new Error(`no port at ${port}: set RUST_JS_PORT`);
if (command === "measure") {
  restore();
  compile();
  const diff = run(["git", "diff", "-U0", "--", ...generated]).out;
  console.log(run(["git", "diff", "--shortstat", "--", ...generated]).out.trim());
  // What a person wouldn't write, each as many lines as MIR adds of it.
  const shapes: [string, RegExp][] = [
    ["temporaries", /^\+.*\b(const|let) _\d+\b/],
    ["labeled blocks", /^\+\s*bb\d+:/],
    ["while (true)", /^\+\s*(bb\d+: )?while \(true\)/],
    ["$iter", /^\+.*\$iter\(/],
    ["Iterator.from", /^\+.*Iterator\.from/],
    ["return;", /^\+\s*return;$/],
    ["renamed x$1", /^\+.*\b[A-Za-z_]\w*\$\d+\b/],
  ];
  const added = diff.split("\n");
  for (const [name, shape] of shapes) console.log(`${name.padEnd(16)} ${added.filter((line) => shape.test(line)).length}`);
  console.log("most differing:");
  const numstat = run(["git", "diff", "--numstat", "--", ...generated]).out.trim().split("\n");
  for (const line of numstat.sort((a, b) => Number(b.split("\t")[0]) - Number(a.split("\t")[0])).slice(0, 5)) {
    console.log(`  ${line}`);
  }
  if (!flags.includes("--keep")) restore();
  console.log(timed("port measure", elapsed()));
} else if (command === "compare") {
  restore();
  const build = run(["bun", "run", "build"]);
  if (build.code !== 0) {
    restore();
    throw new Error(`MIR's build of the port failed:\n${build.out.slice(-4000)}`);
  }
  const built = elapsed();
  const compared = run(["bun", "scripts/snapshotPages.ts", "compare", reference], {}, 200_000);
  restore();
  console.log(compared.out.trim().split("\n").slice(-12).join("\n"));
  console.log(timed("port build", built));
  console.log(timed("port compare, pages", elapsed() - built));
  if (compared.code !== 0 || !/\b0 of \d+ pages differ/.test(compared.out)) process.exit(1);
} else {
  console.error("usage: bun run port measure [--keep] | compare");
  process.exit(2);
}
