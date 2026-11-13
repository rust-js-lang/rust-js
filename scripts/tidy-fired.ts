// Which of tidy's passes changed a body over the corpus (#1461): each case
// compiled with `--tidy-fired`, on every core, its passes summed. A pass
// that changed nothing of any case is listed first, a candidate to delete
// once the ports and the generated programs say the same; then each that
// did, with the cases it fired in and the bodies it changed. "Fired" is
// "changed a body", not "was needed": a pass may fire and a later one undo
// it, and one that never fires here may of a user's program.
//
//   bun scripts/tidy-fired.ts                       # the corpus
//   bun scripts/tidy-fired.ts --stage jumps         # its passes alone
//   bun scripts/tidy-fired.ts --std-iterators       # in the build of std's iterator structs
//   bun scripts/tidy-fired.ts --programs 1000 200   # generated programs, 200 seeds from 1000
import { join } from "node:path";

import { buildCompiler, compiler, run, target } from "../test/support";
import { corpusPrograms, eachCompiled, generatedPrograms } from "./each-compiled";

const args = process.argv.slice(2);
const stageAt = args.indexOf("--stage");
const stage = stageAt < 0 ? undefined : args[stageAt + 1];
const build = args.includes("--std-iterators") ? ["--std-iterators"] : [];
const programsAt = args.indexOf("--programs");

buildCompiler();
const passes = run([compiler, "--tidy-passes"])
  .trim()
  .split("\n")
  .filter((pass) => stage === undefined || pass.startsWith(`${stage}/`));
if (passes.length === 0) throw new Error(`--stage ${stage}: no pass of it; \`rust-js --tidy-passes\` lists them`);

const programs =
  programsAt < 0
    ? corpusPrograms()
    : generatedPrograms(Number(args[programsAt + 1]), Number(args[programsAt + 2]), join(target, "tidy-fired-programs"));
const fired = new Map<string, { cases: number; bodies: number }>();
const { compiled, refused } = await eachCompiled(
  programs,
  (out) => [...build, "--tidy-fired", out],
  (text) => {
    for (const line of text.split("\n").filter(Boolean)) {
      const [pass, bodies] = line.split("\t");
      const at = fired.get(pass) ?? { cases: 0, bodies: 0 };
      fired.set(pass, { cases: at.cases + 1, bodies: at.bodies + Number(bodies) });
    }
  },
);

const never = passes.filter((pass) => !fired.has(pass));
console.log(`${compiled} programs compiled, ${refused} refused; ${passes.length - never.length} of ${passes.length} passes changed a body`);
console.log(`\nnever fired (${never.length}):`);
for (const pass of never) console.log(`  ${pass}`);
console.log("\nfired: programs, bodies");
for (const pass of passes.filter((pass) => fired.has(pass))) {
  const { cases: of, bodies } = fired.get(pass)!;
  console.log(`  ${pass}\t${of}\t${bodies}`);
}
