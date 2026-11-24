// `CellForm`'s shadow over the corpus (#2490, Campaign 4 phase 1): each
// case that holds a `RefCell` compiled as a binary, a program and a library
// with `--shadow cell-form=<file>`, which writes each borrow's form as the
// compile takes it beside the one `CellForm`'s proof gives, alone and with
// the crate's bodies' borrow summaries; the lines gathered and counted by
// the borrow's shape in its source, and what defeated each proof. No
// output changes: it measures what a flip would.
//
//   bun scripts/cell-form-shadow.ts [--out <file>] [--lines <file>] [name ..]
//
// `--lines <file or dir>` counts lines a run wrote already: the React
// port's, each crate's file of `RUST_JS_SHADOW=cell-form=<dir> bun
// scripts/react-in-rust.ts regen` (ADR 1199).
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { directives } from "../test/corpus-cases";
import { compileCommand, type Build } from "../test/programs";
import { root } from "../test/support";
import { run as runAsync } from "../test/child";
import { atOnce } from "./shard";

const valueOf = (flag: string) => {
  const at = process.argv.indexOf(flag);
  return at >= 0 ? process.argv[at + 1] : undefined;
};
const out = valueOf("--out");
const given = valueOf("--lines");
const named = process.argv.slice(2).filter((arg, i, all) => !arg.startsWith("--") && !["--out", "--lines"].includes(all[i - 1] ?? ""));

/** A borrow's line: the build and case it's of, then `--shadow`'s fields. */
type Site = {
  build: string;
  name: string;
  today: string;
  alone: string;
  summed: string;
  shape: string;
  of: string;
  field: string;
  cell: string;
  defeatAlone: string;
  defeatSummed: string;
  fn: string;
  at: string;
};

function sitesOf(text: string, build: string, name: string): Site[] {
  return text
    .split("\n")
    .filter((line) => line.startsWith("cell-form\tsite\t"))
    .map((line) => {
      const [, , today, alone, summed, shape, of, field, cell, defeatAlone, defeatSummed, fn, at] = line.split("\t");
      return { build, name, today, alone, summed, shape, of, field, cell, defeatAlone, defeatSummed, fn, at };
    });
}

/** Of each build and case, each field used whole, and whether every use is
 * a lend to a parameter only ever borrowed (`lent-to-borrows`). */
const wholes = new Map<string, string>();
function wholesOf(text: string, build: string, name: string) {
  for (const line of text.split("\n").filter((l) => l.startsWith("cell-form\twhole\t"))) {
    const [, , field, lent] = line.split("\t");
    wholes.set(`${build} ${name} ${field}`, lent!);
  }
}

const failed: string[] = [];

async function shadowOf(name: string, work: string): Promise<Site[]> {
  const file = join(root, "test/corpus", name);
  const source = readFileSync(file, "utf8");
  const want = directives(source);
  if (typeof want === "string" || want.kind === "compile-fail" || !source.includes("RefCell")) return [];
  const sites: Site[] = [];
  for (const build of ["binary", "program", "library"] as Build[]) {
    const dir = join(work, name, build);
    mkdirSync(join(dir, build), { recursive: true });
    const log = join(dir, "shadow.txt");
    const { cmd } = compileCommand(file, dir, want.edition, build, ["--shadow", `cell-form=${log}`]);
    const p = await runAsync(cmd, root, 180_000);
    // A refusal is the case's to make, a library's where it says so, not a crash.
    const refused = want.kind === "ignore-rust-js" || (build === "library" && source.includes("//@ library-refused"));
    if (p.code !== 0 && (!refused || p.stderr.includes("panicked"))) failed.push(`${name} (${build})`);
    if (existsSync(log)) {
      const text = readFileSync(log, "utf8");
      sites.push(...sitesOf(text, build, name));
      wholesOf(text, build, name);
    }
  }
  return sites;
}

let sites: Site[] = [];
if (given) {
  // A file, or a directory of them, `RUST_JS_SHADOW`'s, a crate's each.
  const files = statSync(given).isDirectory() ? readdirSync(given).filter((n) => n.endsWith(".txt")).map((n) => join(given, n)) : [given];
  for (const file of files) {
    const text = readFileSync(file, "utf8");
    sites.push(...sitesOf(text, "port", file));
    wholesOf(text, "port", file);
  }
} else {
  const work = mkdtempSync(join(root, "target", "cell-form-shadow-"));
  const cases = named.length ? named : readdirSync(join(root, "test/corpus")).filter((n) => n.endsWith(".rs")).sort();
  let next = 0;
  await Promise.all(
    Array.from({ length: atOnce("test") }, async () => {
      while (next < cases.length) sites.push(...(await shadowOf(cases[next++], work)));
    }),
  );
  rmSync(work, { recursive: true, force: true });
}

/** `n` of `of`, and its share. */
const share = (n: number, of: number) => (of === 0 ? "0" : `${n} (${Math.round((100 * n) / of)}%)`);
// A declared form, a field that holds its value or a flattened state, is
// refused where its proof fails, never counted (design point 1).
const quietToday = (s: Site) => s.today === "unchecked" || s.today === "momentary";
const declared = (s: Site) => s.today === "declared";

function table(rows: Site[], by: (s: Site) => string): string {
  const groups = new Map<string, Site[]>();
  for (const s of rows) groups.set(by(s), [...(groups.get(by(s)) ?? []), s]);
  const lines = [
    "| shape | sites | quiet today | kept quiet, alone | kept quiet, summaries | gain `$borrow`, alone | gain `$borrow`, summaries | counted today, proof quiet | first defeats with summaries |",
    "|---|---|---|---|---|---|---|---|---|",
  ];
  for (const [key, group] of [...groups].sort((a, b) => b[1].length - a[1].length)) {
    const quiet = group.filter(quietToday);
    const keptAlone = quiet.filter((s) => s.alone === "quiet").length;
    const keptSummed = quiet.filter((s) => s.summed === "quiet").length;
    const loses = group.filter((s) => s.today === "counted" && s.summed === "quiet").length;
    const defeats = new Map<string, number>();
    for (const s of quiet.filter((s) => s.summed === "counted")) defeats.set(s.defeatSummed, (defeats.get(s.defeatSummed) ?? 0) + 1);
    const top = [...defeats].sort((a, b) => b[1] - a[1]).map(([d, n]) => `${d} ${n}`).join(", ");
    lines.push(
      `| ${key} | ${group.length} | ${quiet.length} | ${share(keptAlone, quiet.length)} | ${share(keptSummed, quiet.length)} | ${quiet.length - keptAlone} | ${quiet.length - keptSummed} | ${loses} | ${top || "-"} |`,
    );
  }
  return lines.join("\n");
}

/** lisa's 22 patterns (#2490 comment 7132169613), each a predicate of a
 * borrow's line: they overlap, as hers do. */
const temporary = (s: Site) => ["method", "read", "write", "argument", "other", "tail"].includes(s.shape);
const defeated = (s: Site, ...kinds: string[]) => kinds.includes(s.defeatAlone);
const patterns: [string, (s: Site) => boolean][] = [
  ["momentary: a method of the guarded value", (s) => s.shape === "method"],
  ["momentary: read through `*`", (s) => s.shape === "read"],
  ["momentary: written whole", (s) => s.shape === "write"],
  ["a borrow of a local's or parameter's field", (s) => s.of === "param-field" || s.of === "local-field"],
  ["a borrow of `self`'s field in a method", (s) => s.of === "self-field"],
  ["guard bound by `let`", (s) => s.shape === "let"],
  ["guard held across a call to a crate function", (s) => s.shape.startsWith("let") && defeated(s, "crate-fn")],
  ["guard in a `for` loop's iterable", (s) => s.shape === "for"],
  ["  of which the loop body calls a crate function", (s) => s.shape === "for" && defeated(s, "crate-fn")],
  ["temporary guard while a closure runs", (s) => temporary(s) && defeated(s, "closure", "std-runs-crate")],
  ["guard held while a closure runs", (s) => s.shape.startsWith("let") && defeated(s, "closure", "std-runs-crate")],
  ["guard given as an argument", (s) => s.shape === "argument"],
  ["another borrow while a guard lives", (s) => defeated(s, "second-borrow")],
  ["guard bound and never used", (s) => s.shape === "let-unread"],
  ["guard in a `match` scrutinee", (s) => s.shape === "match"],
  ["guard in an `if let` scrutinee", (s) => s.shape === "if-let"],
  ["guard in a `let`-`else` scrutinee", (s) => s.shape === "let-else"],
  ["guard in a `while let` scrutinee", (s) => s.shape === "while-let"],
  ["a borrow inside `LOCAL.with(|c| ..)`", (s) => s.of === "param" && s.fn.includes("{closure#")],
  ["`Ref::map`/`RefMut::map`", (s) => s.shape === "map"],
  ["guard returned from a fn", (s) => s.shape === "returned"],
  ["`try_borrow*` (stays counted)", (s) => defeated(s, "try-borrow")],
];

function patternTable(rows: Site[]): string {
  const lines = [
    "| pattern | sites | quiet today | gain `$borrow`, alone | gain `$borrow`, with summaries | first defeats with summaries | counted today, proof quiet | declared, proof fails (refused) |",
    "|---|---|---|---|---|---|---|---|",
  ];
  for (const [name, holds] of patterns) {
    const group = rows.filter(holds);
    const quiet = group.filter(quietToday);
    const gainsAlone = quiet.filter((s) => s.alone === "counted").length;
    const gains = quiet.filter((s) => s.summed === "counted");
    const defeats = new Map<string, number>();
    for (const s of gains) defeats.set(s.defeatSummed, (defeats.get(s.defeatSummed) ?? 0) + 1);
    const top = [...defeats].sort((a, b) => b[1] - a[1]).map(([d, n]) => `${d} ${n}`).join(", ");
    const loses = group.filter((s) => s.today === "counted" && s.summed === "quiet").length;
    const refused = group.filter((s) => declared(s) && s.summed === "counted").length;
    lines.push(`| ${name} | ${group.length} | ${quiet.length} | ${share(gainsAlone, quiet.length)} | ${share(gains.length, quiet.length)} | ${top || "-"} | ${loses} | ${refused} |`);
  }
  return lines.join("\n");
}

for (const build of [...new Set(sites.map((s) => s.build))]) {
  const of = sites.filter((s) => s.build === build);
  console.log(`\n## ${build}, by lisa's patterns\n`);
  console.log(patternTable(of));
}
for (const build of [...new Set(sites.map((s) => s.build))]) {
  const of = sites.filter((s) => s.build === build);
  console.log(`\n## ${build}: ${of.length} borrows in ${new Set(of.map((s) => s.name)).size} cases\n`);
  console.log(table(of, (s) => s.shape));
  console.log(`\n### by what it's borrowed of\n`);
  console.log(table(of, (s) => s.of));
}
// Each field quiet today, unchecked at every borrow, that a site's proof
// counts: a cell that would gain `$borrow` (or, declared, a refusal).
for (const build of [...new Set(sites.map((s) => s.build))]) {
  const fields = new Map<string, Site[]>();
  for (const s of sites.filter((s) => s.build === build && s.field !== "-")) fields.set(`${s.name} ${s.field}`, [...(fields.get(`${s.name} ${s.field}`) ?? []), s]);
  const quietFields = [...fields].filter(([, of]) => of.every((s) => s.today === "unchecked" || s.today === "declared"));
  const losing = quietFields.filter(([, of]) => of.some((s) => s.summed === "counted"));
  console.log(`\n## ${build}: ${quietFields.length} fields unchecked or declared today, ${losing.length} of them a proof counts\n`);
  for (const [key, of] of losing.slice(0, 40)) {
    const first = of.find((s) => s.summed === "counted")!;
    console.log(`- ${key} (${of[0]!.today}): ${first.defeatSummed} at ${first.at} in ${first.fn}, ${first.shape}`);
  }
}
// The two whole-use proofs (#2490, measured): A, a field lent only to
// parameters only ever borrowed is as its own borrows; B, the type's
// fixpoint, a field used whole whose every borrow of its cell's type in
// the crate is proved quiet by its guard's range, so no whole use, itself a
// moment's borrow, can meet one held.
for (const build of [...new Set(sites.map((s) => s.build))]) {
  const of = sites.filter((s) => s.build === build);
  const rangeQuiet = (s: Site) => s.summed === "quiet" || s.defeatSummed === "whole-use";
  const typeQuiet = new Map<string, boolean>();
  for (const s of of) typeQuiet.set(`${s.name} ${s.cell}`, (typeQuiet.get(`${s.name} ${s.cell}`) ?? true) && rangeQuiet(s));
  const whole = of.filter((s) => (s.today === "unchecked" || s.today === "momentary") && s.defeatSummed === "whole-use");
  const a = (s: Site) => wholes.get(`${build} ${s.name} ${s.field}`) === "lent-to-borrows";
  const b = (s: Site) => typeQuiet.get(`${s.name} ${s.cell}`) === true;
  console.log(`\n## ${build}: the whole-use proofs, of ${whole.length} quiet sites a whole use counts\n`);
  console.log(`- A, lent only to parameters only ever borrowed: wins back ${whole.filter(a).length}`);
  console.log(`- B, every borrow of the cell's type proved quiet: wins back ${whole.filter(b).length}`);
  console.log(`- either: ${whole.filter((s) => a(s) || b(s)).length}; neither: ${whole.filter((s) => !a(s) && !b(s)).length}`);
  for (const s of whole.filter((s) => !a(s) && !b(s)).slice(0, 20)) console.log(`  - ${s.name} ${s.field}: ${wholes.get(`${build} ${s.name} ${s.field}`)} at ${s.at}`);
}
if (out) writeFileSync(out, sites.map((s) => Object.values(s).join("\t")).join("\n") + "\n");
if (failed.length > 0) {
  console.error(`${failed.length} builds failed to compile: ${failed.join(", ")}`);
  process.exit(1);
}
