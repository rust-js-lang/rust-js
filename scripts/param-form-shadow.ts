// `ParamForm`'s shadow over the corpus (#2539, Campaign 6 step 0, ADR
// 1203): each case compiled as a binary, a program and a library with
// `--shadow param-form=<file>`, which writes, of each value converted where
// it crosses into or out of generic code, the form the compile takes beside
// every form generic code's view differs in there, the plan `ParamForm`
// would derive, and of each iterator generic code is asked how it holds,
// whether a bound that reads its value meets a JS iterator; the lines
// gathered and counted. No output changes: it measures where `ParamForm`'s
// one record would decide otherwise than today's forms. A heavy run, it
// takes the machine's turn (ADR 1146).
//
//   bun scripts/param-form-shadow.ts [--lines <file or dir>] [name ..]
//
// `--lines` counts lines a run wrote already: the React port's, each
// crate's file of `RUST_JS_SHADOW=param-form=<dir> bun
// scripts/react-in-rust.ts regen` (ADR 1199).
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { directives } from "../test/corpus-cases";
import { compileCommand, type Build } from "../test/programs";
import { buildCompiler, root } from "../test/support";
import { run as runAsync } from "../test/child";
import { atOnce } from "./shard";
import { holdLease } from "./leases";

const at = process.argv.indexOf("--lines");
const given = at >= 0 ? process.argv[at + 1] : undefined;
const named = process.argv.slice(2).filter((arg, i, all) => !arg.startsWith("--") && all[i - 1] !== "--lines");

/** A line of the shadow, of the build and case it's of: `site` or `iterated`, then its fields. */
type Line = { build: string; name: string; fields: string[] };

const failed: string[] = [];
const lines: Line[] = [];

function linesOf(text: string, build: string, name: string) {
  for (const line of text.split("\n").filter((l) => l.startsWith("param-form\t"))) {
    lines.push({ build, name, fields: line.split("\t").slice(1) });
  }
}

async function shadowOf(name: string, work: string) {
  const file = join(root, "test/corpus", name);
  const source = readFileSync(file, "utf8");
  const want = directives(source);
  if (typeof want === "string" || want.kind === "compile-fail") return;
  for (const build of ["binary", "program", "library"] as Build[]) {
    const dir = join(work, name, build);
    mkdirSync(join(dir, build), { recursive: true });
    const log = join(dir, "shadow.txt");
    const { cmd } = compileCommand(file, dir, want.edition, build, ["--shadow", `param-form=${log}`]);
    const p = await runAsync(cmd, root, 180_000);
    if (p.code !== 0 && p.stderr.includes("panicked")) failed.push(`${name} (${build})`);
    if (existsSync(log)) linesOf(readFileSync(log, "utf8"), build, name);
  }
}

holdLease("param-form-shadow");
// The compiler this tree makes, built once the turn is this run's.
buildCompiler();
if (given) {
  const files = statSync(given).isDirectory() ? readdirSync(given).filter((n) => n.endsWith(".txt")).map((n) => join(given, n)) : [given];
  for (const file of files) linesOf(readFileSync(file, "utf8"), "port", file);
} else {
  const work = mkdtempSync(join(root, "target", "param-form-shadow-"));
  const cases = named.length ? named : readdirSync(join(root, "test/corpus")).filter((n) => n.endsWith(".rs")).sort();
  let next = 0;
  await Promise.all(
    Array.from({ length: atOnce("test") }, async () => {
      while (next < cases.length) await shadowOf(cases[next++]!, work);
    }),
  );
  rmSync(work, { recursive: true, force: true });
}

/** Counts of `rows` by `key`, most first. */
function counted<T>(rows: T[], key: (row: T) => string): [string, number][] {
  const counts = new Map<string, number>();
  for (const row of rows) counts.set(key(row), (counts.get(key(row)) ?? 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1]);
}

for (const build of [...new Set(lines.map((l) => l.build))]) {
  const of = lines.filter((l) => l.build === build);
  // site: route verdict taken plan callee declared instantiated owner
  const sites = of.filter((l) => l.fields[0] === "site");
  console.log(`\n## ${build}: ${sites.length} conversions in ${new Set(sites.map((l) => l.name)).size} cases\n`);
  console.log("| route | verdict | taken | plan | conversions | cases |");
  console.log("|---|---|---|---|---|---|");
  const key = (l: Line) => l.fields.slice(1, 5).join(" | ");
  for (const [k, n] of counted(sites, key)) {
    console.log(`| ${k} | ${n} | ${new Set(sites.filter((l) => key(l) === k).map((l) => l.name)).size} |`);
  }
  for (const l of sites.filter((l) => l.fields[2] !== "agrees")) {
    const [, route, verdict, taken, plan, callee, declared, instantiated, owner] = l.fields;
    console.log(`- ${verdict} ${route}: ${taken}, plan ${plan}: \`${callee}\`'s \`${declared}\` as \`${instantiated}\`, in \`${owner}\` (${l.name})`);
  }
  // iterated: which verdict answer readers fn ty
  const iterated = of.filter((l) => l.fields[0] === "iterated");
  console.log(`\n${iterated.length} iterator forms asked, by answer and verdict:`);
  for (const [k, n] of counted(iterated, (l) => `${l.fields[1]} ${l.fields[2]} ${l.fields[3]}`)) console.log(`- ${k}: ${n}`);
  for (const l of iterated.filter((l) => l.fields[2] !== "agrees")) {
    const [, which, verdict, answer, readers, fn, ty] = l.fields;
    console.log(`- ${verdict} ${which}: \`${ty}\` of \`${fn}\`, ${answer}, read by ${readers} (${l.name})`);
  }
}
if (failed.length) {
  console.log(`\npanicked: ${failed.join(", ")}`);
  process.exitCode = 1;
}
