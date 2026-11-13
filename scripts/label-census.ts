// The compiler-made labels rust-js's JS keeps, by the MIR shape that left
// each (#1461): each program compiled with `--label-census`, on every
// core, its rows summed and ranked, each with a program it's of. A row is
// a label's kind, what decided each way to its block in MIR, and how far
// its jumps reach; its next fix is its emitter's, the decisions walk's, or
// none, a jump no structured form makes, which the program's own label
// would name.
//
//   bun scripts/label-census.ts                       # the corpus
//   bun scripts/label-census.ts --programs 1000 200   # generated programs, 200 seeds from 1000
//   bun scripts/label-census.ts --match-trees 1 24     # generated matches (test/match-trees.ts), 24 seeds from 1
//   bun scripts/label-census.ts --std-iterators       # in the build of std's iterator structs
import { join, relative } from "node:path";

import { buildCompiler, root, target } from "../test/support";
import { corpusPrograms, eachCompiled, generatedPrograms, matchTreePrograms } from "./each-compiled";

const args = process.argv.slice(2);
const build = args.includes("--std-iterators") ? ["--std-iterators"] : [];
const programsAt = args.indexOf("--programs");
const matchesAt = args.indexOf("--match-trees");
const seeds = (at: number) => [Number(args[at + 1]), Number(args[at + 2])] as const;

buildCompiler();
const programs =
  programsAt >= 0
    ? generatedPrograms(...seeds(programsAt), join(target, "label-census-programs"))
    : matchesAt >= 0
      ? matchTreePrograms(...seeds(matchesAt), join(target, "label-census-matches"))
      : corpusPrograms();
const rows = new Map<string, { labels: number; programs: number; first: string }>();
let labeled = 0;
const { compiled, refused } = await eachCompiled(
  programs,
  (out) => [...build, "--label-census", out],
  (text, program) => {
    const lines = text.split("\n").filter(Boolean);
    if (lines.length > 0) labeled++;
    for (const line of lines) {
      const [count, shape] = line.split("\t");
      const at = rows.get(shape) ?? { labels: 0, programs: 0, first: relative(root, program.file) };
      rows.set(shape, { labels: at.labels + Number(count), programs: at.programs + 1, first: at.first });
    }
  },
);

const total = [...rows.values()].reduce((sum, row) => sum + row.labels, 0);
console.log(`${compiled} programs compiled, ${refused} refused; ${labeled} keep a compiler-made label, ${total} labels in ${rows.size} shapes`);
console.log("\nlabels\tprograms\tshape\tfirst of it");
for (const [shape, row] of [...rows].sort((a, b) => b[1].labels - a[1].labels || a[0].localeCompare(b[0]))) {
  console.log(`${row.labels}\t${row.programs}\t${shape}\t${row.first}`);
}
